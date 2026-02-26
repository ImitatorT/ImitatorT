//! SQLite 存储实现
//!
//! 使用 SQLite 作为后端，适合需要持久化的场景

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use rusqlite::Connection;

use crate::core::store::{MessageFilter, Store};
use crate::domain::{Agent, Department, Group, LLMConfig, Message, MessageTarget, Organization, Role};
use crate::domain::user::User;

/// SQLite 存储
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    /// 创建新的 SQLite 存储
    ///
    /// 如果数据库文件不存在，会自动创建
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// 创建内存数据库（用于测试）
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// 初始化数据库表结构
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire database lock: {}", e))?;

        conn.execute_batch(
            "
            -- 部门表
            CREATE TABLE IF NOT EXISTS departments (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                parent_id TEXT,
                leader_id TEXT
            );

            -- Agent 表
            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                department_id TEXT,
                role_title TEXT NOT NULL,
                role_responsibilities TEXT,
                role_expertise TEXT,
                role_system_prompt TEXT NOT NULL,
                llm_model TEXT NOT NULL,
                llm_api_key TEXT NOT NULL,
                llm_base_url TEXT NOT NULL,
                FOREIGN KEY (department_id) REFERENCES departments(id)
            );

            -- 群聊表
            CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                creator_id TEXT NOT NULL,
                members TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            -- 消息表
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                from_agent TEXT NOT NULL,
                target_type TEXT NOT NULL,
                target_id TEXT,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                reply_to TEXT,
                mentions TEXT
            );

            -- 用户表
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                email TEXT,
                password_hash TEXT NOT NULL,
                is_director BOOLEAN NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            );

            -- 创建索引
            CREATE INDEX IF NOT EXISTS idx_messages_from ON messages(from_agent);
            CREATE INDEX IF NOT EXISTS idx_messages_target ON messages(target_type, target_id);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);

            -- 创建部门索引
            CREATE INDEX IF NOT EXISTS idx_departments_parent ON departments(parent_id);
            CREATE INDEX IF NOT EXISTS idx_agents_department ON agents(department_id);

            -- 创建用户索引
            CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

            PRAGMA foreign_keys = ON;
            "
        )?;

        Ok(())
    }

    /// 在阻塞线程池中执行数据库操作
    async fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock()
                .map_err(|e| anyhow::anyhow!("Failed to acquire database lock: {}", e))?;
            f(&mut conn)
        })
        .await
        .map_err(|e| anyhow::anyhow!("Task failed: {}", e))?
    }
}

#[async_trait]
impl Store for SqliteStore {
    async fn save_organization(&self, org: &Organization) -> Result<()> {
        let org = org.clone();
        self.execute(move |conn| {
            // 清除旧数据
            conn.execute("DELETE FROM agents", [])?;
            conn.execute("DELETE FROM departments", [])?;

            // 插入部门
            for dept in &org.departments {
                let parent_id = dept.parent_id.as_deref();
                let leader_id = dept.leader_id.as_deref();

                conn.execute(
                    "INSERT INTO departments (id, name, parent_id, leader_id)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        &dept.id,
                        &dept.name,
                        parent_id,
                        leader_id,
                    ],
                )?;
            }

            // 插入 Agent
            for agent in &org.agents {
                let dept_id = agent.department_id.as_deref();
                let resp_json = serde_json::to_string(&agent.role.responsibilities).unwrap_or_default();
                let exp_json = serde_json::to_string(&agent.role.expertise).unwrap_or_default();

                conn.execute(
                    "INSERT INTO agents (
                        id, name, department_id,
                        role_title, role_responsibilities, role_expertise, role_system_prompt,
                        llm_model, llm_api_key, llm_base_url
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        &agent.id,
                        &agent.name,
                        dept_id,
                        &agent.role.title,
                        resp_json,
                        exp_json,
                        &agent.role.system_prompt,
                        &agent.llm_config.model,
                        &agent.llm_config.api_key,
                        &agent.llm_config.base_url,
                    ],
                )?;
            }

            Ok(())
        }).await
    }

    async fn load_organization(&self) -> Result<Organization> {
        self.execute(|conn| {
            let mut org = Organization::new();

            // 加载部门
            let mut stmt = conn.prepare(
                "SELECT id, name, parent_id, leader_id FROM departments"
            )?;

            let dept_iter = stmt.query_map([], |row| {
                Ok(Department {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    parent_id: row.get(2)?,
                    leader_id: row.get(3)?,
                })
            })?;

            for dept in dept_iter {
                org.add_department(dept?);
            }

            // 加载 Agent
            let mut stmt = conn.prepare(
                "SELECT
                    id, name, department_id,
                    role_title, role_responsibilities, role_expertise, role_system_prompt,
                    llm_model, llm_api_key, llm_base_url
                 FROM agents"
            )?;

            let agent_iter = stmt.query_map([], |row| {
                let responsibilities: String = row.get(4)?;
                let expertise: String = row.get(5)?;

                Ok(Agent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    department_id: row.get(2)?,
                    role: Role {
                        title: row.get(3)?,
                        responsibilities: serde_json::from_str(&responsibilities).unwrap_or_default(),
                        expertise: serde_json::from_str(&expertise).unwrap_or_default(),
                        system_prompt: row.get(6)?,
                    },
                    llm_config: LLMConfig {
                        model: row.get(7)?,
                        api_key: row.get(8)?,
                        base_url: row.get(9)?,
                    },
                })
            })?;

            for agent in agent_iter {
                org.add_agent(agent?);
            }

            Ok(org)
        }).await
    }

    async fn save_group(&self, group: &Group) -> Result<()> {
        let group = group.clone();
        self.execute(move |conn| {
            let members_json = serde_json::to_string(&group.members).unwrap_or_default();

            conn.execute(
                "INSERT OR REPLACE INTO groups (id, name, creator_id, members, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    &group.id,
                    &group.name,
                    &group.creator_id,
                    members_json,
                    &group.created_at,
                ],
            )?;
            Ok(())
        }).await
    }

    async fn load_groups(&self) -> Result<Vec<Group>> {
        self.execute(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, creator_id, members, created_at FROM groups"
            )?;

            let group_iter = stmt.query_map([], |row| {
                let members: String = row.get(3)?;
                let created_at: i64 = row.get(4)?;

                Ok(Group {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    creator_id: row.get(2)?,
                    members: serde_json::from_str(&members).unwrap_or_default(),
                    created_at,
                })
            })?;

            let mut groups = Vec::new();
            for group in group_iter {
                groups.push(group?);
            }

            Ok(groups)
        }).await
    }

    async fn delete_group(&self, group_id: &str) -> Result<()> {
        let group_id = group_id.to_string();
        self.execute(move |conn| {
            conn.execute("DELETE FROM groups WHERE id = ?1", [group_id])?;
            Ok(())
        }).await
    }

    async fn save_message(&self, message: &Message) -> Result<()> {
        let message = message.clone();
        self.execute(move |conn| {
            let (target_type, target_id): (&str, Option<&str>) = match &message.to {
                MessageTarget::Direct(id) => ("direct", Some(id.as_str())),
                MessageTarget::Group(id) => ("group", Some(id.as_str())),
            };

            conn.execute(
                "INSERT INTO messages (id, from_agent, target_type, target_id, content, timestamp, reply_to, mentions)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    &message.id,
                    &message.from,
                    target_type,
                    target_id,
                    &message.content,
                    &message.timestamp,
                    message.reply_to.as_ref(),
                    if message.mentions.is_empty() {
                        None
                    } else {
                        Some(message.mentions.join(","))
                    },
                ],
            )?;
            Ok(())
        }).await
    }

    async fn save_messages(&self, messages: &[Message]) -> Result<()> {
        let messages: Vec<Message> = messages.to_vec();
        self.execute(move |conn| {
            let tx = conn.transaction()?;

            for message in &messages {
                let (target_type, target_id): (&str, Option<&str>) = match &message.to {
                    MessageTarget::Direct(id) => ("direct", Some(id.as_str())),
                    MessageTarget::Group(id) => ("group", Some(id.as_str())),
                };

                tx.execute(
                    "INSERT INTO messages (id, from_agent, target_type, target_id, content, timestamp, reply_to, mentions)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        &message.id,
                        &message.from,
                        target_type,
                        target_id,
                        &message.content,
                        &message.timestamp,
                        message.reply_to.as_ref(),
                        if message.mentions.is_empty() {
                            None
                        } else {
                            Some(message.mentions.join(","))
                        },
                    ],
                )?;
            }

            tx.commit()?;
            Ok(())
        }).await
    }

    async fn load_messages(&self, filter: MessageFilter) -> Result<Vec<Message>> {
        self.execute(move |conn| {
            let mut conditions = Vec::new();
            let mut from_val: Option<String> = None;
            let mut target_type_val: Option<String> = None;
            let mut to_val: Option<String> = None;
            let mut since_val: Option<i64> = None;

            if let Some(from) = filter.from {
                conditions.push("from_agent = ?".to_string());
                from_val = Some(from);
            }

            if let Some(target_type) = filter.target_type {
                conditions.push("target_type = ?".to_string());
                target_type_val = Some(target_type);
            }

            if let Some(to) = filter.to {
                conditions.push("target_id = ?".to_string());
                to_val = Some(to);
            }

            if let Some(since) = filter.since {
                conditions.push("timestamp >= ?".to_string());
                since_val = Some(since);
            }

            let where_clause = if conditions.is_empty() {
                "".to_string()
            } else {
                format!("WHERE {}", conditions.join(" AND "))
            };

            let sql = format!(
                "SELECT id, from_agent, target_type, target_id, content, timestamp, reply_to, mentions
                 FROM messages
                 {}
                 ORDER BY timestamp DESC
                 LIMIT {}",
                where_clause,
                filter.limit
            );

            let mut stmt = conn.prepare(&sql)?;

            // Bind parameters in order
            let mut param_idx = 1;
            if let Some(ref val) = from_val {
                stmt.raw_bind_parameter(param_idx, val.as_str())?;
                param_idx += 1;
            }
            if let Some(ref val) = target_type_val {
                stmt.raw_bind_parameter(param_idx, val.as_str())?;
                param_idx += 1;
            }
            if let Some(ref val) = to_val {
                stmt.raw_bind_parameter(param_idx, val.as_str())?;
                param_idx += 1;
            }
            if let Some(val) = since_val {
                stmt.raw_bind_parameter(param_idx, val)?;
            }

            let msg_iter = stmt.raw_query().mapped(|row| {
                let target_type: String = row.get(2)?;
                let target_id: Option<String> = row.get(3)?;

                let target = match target_type.as_str() {
                    "direct" => MessageTarget::Direct(target_id.unwrap_or_default()),
                    "group" => MessageTarget::Group(target_id.unwrap_or_default()),
                    _ => MessageTarget::Direct(String::new()), // 未知类型默认为空direct
                };

                Ok(Message {
                    id: row.get(0)?,
                    from: row.get(1)?,
                    to: target,
                    content: row.get(4)?,
                    timestamp: row.get(5)?,
                    reply_to: row.get::<_, Option<String>>(6)?,
                    mentions: row.get::<_, Option<String>>(7)?
                        .map(|s| s.split(',').map(|s| s.to_string()).collect())
                        .unwrap_or_default(),
                })
            });

            let mut messages = Vec::new();
            for msg in msg_iter {
                messages.push(msg?);
            }

            Ok(messages)
        }).await
    }

    async fn save_user(&self, user: &User) -> Result<()> {
        let user = user.clone();
        self.execute(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO users (id, username, name, email, password_hash, is_director, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    &user.id,
                    &user.username,
                    &user.name,
                    user.email.as_deref(),
                    &user.password_hash,
                    &user.is_director,
                    &user.created_at,
                ],
            )?;
            Ok(())
        }).await
    }

    async fn load_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let username = username.to_string();
        self.execute(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, username, name, email, password_hash, is_director, created_at FROM users WHERE username = ?1"
            )?;

            let user_result = stmt.query_row([username], |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    name: row.get(2)?,
                    email: row.get(3)?,
                    password_hash: row.get(4)?,
                    is_director: row.get(5)?,
                    created_at: row.get(6)?,
                })
            });

            match user_result {
                Ok(user) => Ok(Some(user)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(anyhow::anyhow!(e)),
            }
        }).await
    }

    async fn load_users(&self) -> Result<Vec<User>> {
        self.execute(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, username, name, email, password_hash, is_director, created_at FROM users"
            )?;

            let user_iter = stmt.query_map([], |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    name: row.get(2)?,
                    email: row.get(3)?,
                    password_hash: row.get(4)?,
                    is_director: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?;

            let mut users = Vec::new();
            for user in user_iter {
                users.push(user?);
            }

            Ok(users)
        }).await
    }
}
