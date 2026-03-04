//! Matrix 同步器
//!
//! 负责同步用户和房间到 Matrix Homeserver

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

use super::client::MatrixClient;
use super::config::MatrixConfig;
use super::mapper::Mapper;

/// 同步服务
pub struct SyncService {
    client: MatrixClient,
    config: MatrixConfig,
    mapper: Arc<tokio::sync::RwLock<Mapper>>,
    store: Arc<dyn crate::core::store::Store>,
}

impl SyncService {
    /// 创建新的同步服务
    pub fn new(
        client: MatrixClient,
        config: &MatrixConfig,
        mapper: Arc<tokio::sync::RwLock<Mapper>>,
        store: Arc<dyn crate::core::store::Store>,
    ) -> Self {
        Self {
            client,
            config: config.clone(),
            mapper,
            store,
        }
    }

    /// 同步所有用户
    ///
    /// 1. 从存储加载所有内部用户
    /// 2. 为每个用户生成 Matrix 用户 ID
    /// 3. 在 Homeserver 注册虚拟用户（如需要）
    /// 4. 设置用户显示名称
    pub async fn sync_all_users(&self) -> Result<()> {
        info!("Starting user sync...");

        // 从组织架构加载所有 Agent
        match self.store.load_organization().await {
            Ok(org) => {
                info!("Syncing {} agents", org.agents.len());

                for agent in &org.agents {
                    if let Err(e) = self.sync_agent(agent).await {
                        warn!("Failed to sync agent {}: {}", agent.id, e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to load organization: {}", e);
            }
        }

        // 从存储加载所有人类用户
        match self.store.load_users().await {
            Ok(users) => {
                info!("Syncing {} human users", users.len());

                for user in &users {
                    if let Err(e) = self.sync_user(user).await {
                        warn!("Failed to sync user {}: {}", user.id, e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to load users: {}", e);
            }
        }

        info!("User sync completed");
        Ok(())
    }

    /// 同步单个 Agent
    async fn sync_agent(&self, agent: &crate::domain::Agent) -> Result<()> {
        let localpart = format!("{}_{}", agent.role.title.to_lowercase().replace(' ', "_"), agent.id.chars().take(8).collect::<String>());
        let matrix_user_id = self.config.generate_user_id(&localpart);

        // 注册/确保用户存在
        self.client.register_user(&localpart).await?;

        // 设置显示名称
        if let Err(e) = self.client.set_user_displayname(&matrix_user_id, &agent.name).await {
            warn!("Failed to set displayname for {}: {}", matrix_user_id, e);
        }

        // 更新映射
        {
            let mut mapper = self.mapper.write().await;
            let internal_id = agent.id.clone();
            mapper.register_user(&internal_id, &agent.name);
            debug!("Synced agent: {} -> {}", internal_id, matrix_user_id);
        }

        Ok(())
    }

    /// 同步单个用户
    async fn sync_user(&self, user: &crate::domain::user::User) -> Result<()> {
        let localpart = format!("user_{}", user.username.to_lowercase());
        let matrix_user_id = self.config.generate_user_id(&localpart);

        // 注册/确保用户存在
        self.client.register_user(&localpart).await?;

        // 设置显示名称
        if let Err(e) = self.client.set_user_displayname(&matrix_user_id, &user.name).await {
            warn!("Failed to set displayname for {}: {}", matrix_user_id, e);
        }

        // 更新映射
        {
            let mut mapper = self.mapper.write().await;
            mapper.register_user(&user.id, &user.name);
            debug!("Synced user: {} -> {}", user.id, matrix_user_id);
        }

        Ok(())
    }

    /// 同步所有房间
    ///
    /// 1. 从组织架构加载所有部门
    /// 2. 为每个部门创建 Matrix 房间
    /// 3. 邀请相关用户加入房间
    pub async fn sync_all_rooms(&self) -> Result<()> {
        info!("Starting room sync...");

        // 从组织架构加载所有部门
        match self.store.load_organization().await {
            Ok(org) => {
                info!("Syncing {} departments as rooms", org.departments.len());

                // 获取 bot 用户 ID
                let bot_user_id = self.config.sender_user_id();

                for dept in &org.departments {
                    if let Err(e) = self.sync_department_room(&dept.name, &bot_user_id).await {
                        warn!("Failed to sync department {}: {}", dept.name, e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to load organization: {}", e);
            }
        }

        info!("Room sync completed");
        Ok(())
    }

    /// 同步单个部门房间
    async fn sync_department_room(
        &self,
        department: &str,
        bot_user_id: &str,
    ) -> Result<()> {
        let alias = format!("company_{}", department.to_lowercase().replace(' ', "_"));
        let room_name = format!("{} Department", department);

        // 尝试创建房间
        let room_id = match self
            .client
            .create_room(bot_user_id, Some(&alias), Some(&room_name), None)
            .await
        {
            Ok(room_id) => room_id,
            Err(e) => {
                // 如果房间已存在，尝试获取房间 ID
                warn!("Failed to create room, trying to get existing: {}", e);
                let room_alias = self.config.generate_room_alias(&alias);
                match self.client.get_room_id(&room_alias).await {
                    Ok(room_id) => room_id,
                    Err(e2) => {
                        error!("Failed to get existing room: {}", e2);
                        return Err(e);
                    }
                }
            }
        };

        // 更新映射
        {
            let mut mapper = self.mapper.write().await;
            mapper.register_room(department, &room_id, Some(&room_name));
            // 如果房间 ID 与别名不同，更新映射
            if room_id != alias {
                mapper.update_room_matrix_id(department, &room_id);
            }
            debug!("Synced department room: {} -> {}", department, room_id);
        }

        // 邀请部门成员加入房间
        self.invite_department_members(department, &room_id).await?;

        Ok(())
    }

    /// 邀请部门成员加入房间
    async fn invite_department_members(
        &self,
        department: &str,
        room_id: &str,
    ) -> Result<()> {
        // 获取部门成员列表
        match self.store.load_organization().await {
            Ok(org) => {
                // 找到属于该部门的 Agent
                let dept_agents: Vec<_> = org
                    .agents
                    .iter()
                    .filter(|a| a.department_id.as_deref() == Some(department))
                    .collect();

                if dept_agents.is_empty() {
                    debug!("No agents in department {}", department);
                    return Ok(());
                }

                // 获取成员的 Matrix 用户 ID
                let mapper = self.mapper.read().await;
                let mut invited = 0;

                for agent in &dept_agents {
                    if let Some(matrix_user_id) = mapper.get_matrix_user_id(&agent.id) {
                        // 加入房间（虚拟用户由 Appservice 管理，自动加入）
                        if let Err(e) = self.client.join_room(room_id, matrix_user_id).await {
                            warn!(
                                "Failed to invite agent {} to room {}: {}",
                                agent.id, room_id, e
                            );
                        } else {
                            invited += 1;
                        }
                    }
                }

                info!("Invited {} agents to room {}", invited, room_id);
            }
            Err(e) => {
                warn!("Failed to load organization for invites: {}", e);
            }
        }

        Ok(())
    }

    /// 获取映射器
    pub fn mapper(&self) -> Arc<tokio::sync::RwLock<Mapper>> {
        self.mapper.clone()
    }
}

/// Matrix 通知器
///
/// 用于异步发送消息到 Matrix（非阻塞）
pub struct MatrixNotifier {
    client: MatrixClient,
    mapper: Arc<tokio::sync::RwLock<Mapper>>,
}

impl MatrixNotifier {
    /// 创建新的通知器
    pub fn new(client: MatrixClient, mapper: Arc<tokio::sync::RwLock<Mapper>>) -> Self {
        Self { client, mapper }
    }

    /// 异步通知消息到 Matrix（fire-and-forget）
    pub fn notify(&self, message: &crate::domain::Message) {
        let client = self.client.clone();
        let mapper = self.mapper.clone();
        let message = message.clone();

        // 异步火射，不阻塞主流程
        tokio::spawn(async move {
            if let Err(e) = Self::send_message(client, mapper, &message).await {
                warn!("Failed to send message to Matrix: {}", e);
            }
        });
    }

    /// 发送消息到 Matrix
    async fn send_message(
        client: MatrixClient,
        mapper: Arc<tokio::sync::RwLock<Mapper>>,
        message: &crate::domain::Message,
    ) -> Result<()> {
        // 获取发送者的 Matrix 用户 ID
        let sender_matrix_id = {
            let mapper = mapper.read().await;
            mapper
                .get_matrix_user_id(&message.from)
                .map(String::from)
        };

        let sender = match sender_matrix_id {
            Some(id) => id,
            None => {
                warn!("No Matrix mapping for sender: {}", message.from);
                return Ok(());
            }
        };

        // 获取目标房间的 Matrix ID
        let room_id = {
            let mapper = mapper.read().await;
            match &message.to {
                crate::domain::MessageTarget::Group(group_id) => {
                    mapper.get_matrix_room_id(group_id).map(String::from)
                }
                crate::domain::MessageTarget::Direct(user_id) => {
                    // 私聊暂不处理
                    debug!("Direct message to {} not sent to Matrix", user_id);
                    return Ok(());
                }
            }
        };

        let room = match room_id {
            Some(id) => id,
            None => {
                warn!("No Matrix mapping for target: {:?}", message.to);
                return Ok(());
            }
        };

        // 发送消息
        client
            .send_text_message(&room, &sender, &message.content)
            .await?;

        debug!("Sent message to Matrix: {} -> {}", sender, room);

        Ok(())
    }
}

impl Clone for MatrixNotifier {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            mapper: self.mapper.clone(),
        }
    }
}
