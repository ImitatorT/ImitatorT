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
        let conn = self.conn.lock().unwrap();

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
                timestamp INTEGER NOT NULL
            );

            -- 创建索引
            CREATE INDEX IF NOT EXISTS idx_messages_from ON messages(from_agent);
            CREATE INDEX IF NOT EXISTS idx_messages_target ON messages(target_type, target_id);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);

            -- 创建部门索引
            CREATE INDEX IF NOT EXISTS idx_departments_parent ON departments(parent_id);
            CREATE INDEX IF NOT EXISTS idx_agents_department ON agents(department_id);

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
            let mut conn = conn.lock().unwrap();
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
                MessageTarget::Broadcast => ("broadcast", None),
            };

            conn.execute(
                "INSERT INTO messages (id, from_agent, target_type, target_id, content, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    &message.id,
                    &message.from,
                    target_type,
                    target_id,
                    &message.content,
                    &message.timestamp,
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
                    MessageTarget::Broadcast => ("broadcast", None),
                };

                tx.execute(
                    "INSERT INTO messages (id, from_agent, target_type, target_id, content, timestamp)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        &message.id,
                        &message.from,
                        target_type,
                        target_id,
                        &message.content,
                        &message.timestamp,
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
                "SELECT id, from_agent, target_type, target_id, content, timestamp
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
                    _ => MessageTarget::Broadcast,
                };

                Ok(Message {
                    id: row.get(0)?,
                    from: row.get(1)?,
                    to: target,
                    content: row.get(4)?,
                    timestamp: row.get(5)?,
                })
            });

            let mut messages = Vec::new();
            for msg in msg_iter {
                messages.push(msg?);
            }

            Ok(messages)
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Agent, Department, LLMConfig, Message, Organization, Role};

    fn create_test_organization() -> Organization {
        let mut org = Organization::new();

        org.add_department(Department::top_level("tech", "技术部"));
        org.add_department(Department::child("fe", "前端组", "tech"));

        let agent = Agent::new(
            "ceo",
            "CEO",
            Role::simple("CEO", "你是公司的CEO")
                .with_responsibilities(vec!["决策".to_string(), "管理".to_string()]),
            LLMConfig::openai("test-key"),
        )
        .with_department("tech");

        org.add_agent(agent);
        org
    }

    #[tokio::test]
    async fn test_sqlite_store_organization() {
        let store = SqliteStore::new_in_memory().unwrap();
        let org = create_test_organization();

        store.save_organization(&org).await.unwrap();
        let loaded = store.load_organization().await.unwrap();

        assert_eq!(loaded.agents.len(), 1);
        assert_eq!(loaded.departments.len(), 2);

        // 验证 Agent 数据完整
        let agent = loaded.find_agent("ceo").unwrap();
        assert_eq!(agent.name, "CEO");
        assert_eq!(agent.llm_config.model, "gpt-4o-mini");
        assert_eq!(agent.role.responsibilities.len(), 2);
    }

    #[tokio::test]
    async fn test_sqlite_store_messages() {
        let store = SqliteStore::new_in_memory().unwrap();

        let msg1 = Message::private("a1", "a2", "Hello!");
        let msg2 = Message::group("a1", "g1", "大家好！");
        let msg3 = Message::broadcast("a1", "通知");

        store.save_message(&msg1).await.unwrap();
        store.save_message(&msg2).await.unwrap();
        store.save_message(&msg3).await.unwrap();

        // 测试查询全部
        let messages = store.load_messages(MessageFilter::new().limit(10)).await.unwrap();
        assert_eq!(messages.len(), 3);

        // 测试按发送者查询
        let filter = MessageFilter::new().from("a1");
        let messages = store.load_messages(filter).await.unwrap();
        assert_eq!(messages.len(), 3);

        // 测试按目标类型查询
        let filter = MessageFilter::new().target_type("group");
        let messages = store.load_messages(filter).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "大家好！");

        // 测试按接收者查询
        let filter = MessageFilter::new().to("a2").target_type("direct");
        let messages = store.load_messages(filter).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello!");
    }

    #[tokio::test]
    async fn test_sqlite_store_groups() {
        let store = SqliteStore::new_in_memory().unwrap();

        let group = Group::new(
            "g1",
            "测试群",
            "agent1",
            vec!["agent1".to_string(), "agent2".to_string()],
        );

        store.save_group(&group).await.unwrap();

        let groups = store.load_groups().await.unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "测试群");
        assert_eq!(groups[0].members.len(), 2);

        store.delete_group("g1").await.unwrap();
        let groups = store.load_groups().await.unwrap();
        assert_eq!(groups.len(), 0);
    }

    #[tokio::test]
    async fn test_sqlite_store_batch_messages() {
        let store = SqliteStore::new_in_memory().unwrap();

        let messages: Vec<Message> = (0..100)
            .map(|i| Message::private("a1", "a2", format!("msg{}", i)))
            .collect();

        store.save_messages(&messages).await.unwrap();

        let loaded = store.load_messages(MessageFilter::new().limit(50)).await.unwrap();
        assert_eq!(loaded.len(), 50);
    }

    #[tokio::test]
    async fn test_sqlite_store_load_messages_by_agent() {
        let store = SqliteStore::new_in_memory().unwrap();

        // 创建一些消息
        let messages = vec![
            Message::private("a1", "a2", "a1->a2"),
            Message::private("a2", "a1", "a2->a1"),
            Message::private("a1", "a3", "a1->a3"),
            Message::broadcast("a1", "broadcast"),
        ];

        for msg in &messages {
            store.save_message(msg).await.unwrap();
        }

        // 查询与 a1 相关的消息
        let a1_messages = store.load_messages_by_agent("a1", 10).await.unwrap();
        assert_eq!(a1_messages.len(), 4); // 发送3条 + 接收1条
    }
}
