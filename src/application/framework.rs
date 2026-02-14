//! 虚拟公司框架 API
//!
//! 提供简洁的接口供外部应用使用

use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::info;

use crate::core::agent::{Agent, AgentConfig, AgentManager};
use crate::core::messaging::{Message, MessageBus, MessageType};
use crate::protocol::router::MessageRouter;
use crate::protocol::server::A2AServer;

/// 虚拟公司框架
///
/// 封装所有框架能力，提供简洁的 API
pub struct VirtualCompany {
    /// Agent 管理器
    agent_manager: AgentManager,
    /// 消息总线
    message_bus: Arc<MessageBus>,
    /// A2A 服务端（当前在后台运行，不存储实例）
    #[allow(dead_code)]
    server: Option<A2AServer>,
    /// 消息路由器
    router: MessageRouter,
    /// 本地端点
    local_endpoint: String,
}

impl VirtualCompany {
    /// 创建一个新的虚拟公司实例
    pub fn new(local_endpoint: impl Into<String>) -> Self {
        let local_endpoint = local_endpoint.into();
        let message_bus = Arc::new(MessageBus::new());
        Self {
            agent_manager: AgentManager::new(),
            message_bus: message_bus.clone(),
            server: None,
            router: MessageRouter::new(message_bus, local_endpoint.clone()),
            local_endpoint,
        }
    }

    /// 创建并注册一个新 Agent
    pub async fn create_agent(&self, config: AgentConfig) -> Result<Arc<Agent>> {
        let agent = self.agent_manager.register(config).await?;
        // 连接消息总线（保存接收器到 Agent 中）
        agent.connect_messaging(self.message_bus.clone()).await;
        // 注册到路由器
        self.router.register_local_agent(agent.id());
        Ok(agent)
    }

    /// 获取 Agent
    pub fn get_agent(&self, agent_id: &str) -> Option<Arc<Agent>> {
        self.agent_manager.get(agent_id)
    }

    /// 列出所有 Agent
    pub fn list_agents(&self) -> Vec<Arc<Agent>> {
        self.agent_manager.list()
    }

    /// 启动 A2A HTTP 服务（后台运行）
    pub async fn start_server(&mut self, bind_addr: SocketAddr) -> Result<JoinHandle<()>> {
        let server = A2AServer::new(self.message_bus.clone(), bind_addr);
        let handle = server.spawn();
        // server 被移动到后台任务中，不再存储
        info!("A2A server started on {} (background)", bind_addr);
        Ok(handle)
    }

    /// 注册远程 Agent
    pub fn register_remote_agent(&self, agent_id: &str, endpoint: &str) {
        self.router.register_remote_agent(agent_id, endpoint);
    }

    /// 发送广播消息给所有 Agent
    pub fn broadcast(&self, from: &str, content: &str) -> Result<usize> {
        let message = Message {
            id: uuid::Uuid::new_v4().to_string(),
            msg_type: MessageType::Broadcast,
            from: from.to_string(),
            to: vec![],
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        };
        self.message_bus.broadcast(message)
    }

    /// 创建群聊
    pub async fn create_group(
        &self,
        group_id: &str,
        name: &str,
        creator: &str,
        members: Vec<String>,
    ) -> Result<String> {
        self.message_bus
            .create_group(group_id, name, creator, members)
            .await
    }

    /// 获取消息总线
    pub fn message_bus(&self) -> Arc<MessageBus> {
        self.message_bus.clone()
    }

    /// 获取消息路由器
    pub fn router(&self) -> &MessageRouter {
        &self.router
    }

    /// 获取本地端点
    pub fn local_endpoint(&self) -> &str {
        &self.local_endpoint
    }
}

impl Default for VirtualCompany {
    fn default() -> Self {
        Self::new("http://localhost:8080")
    }
}

/// 应用构建器
///
/// 用于链式构建和配置 VirtualCompany
pub struct AppBuilder {
    bind_addr: Option<SocketAddr>,
    local_endpoint: String,
}

impl AppBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            bind_addr: None,
            local_endpoint: "http://localhost:8080".to_string(),
        }
    }

    /// 设置本地端点
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.local_endpoint = endpoint.into();
        self
    }

    /// 设置服务器绑定地址
    pub fn with_server(mut self, bind_addr: SocketAddr) -> Self {
        self.bind_addr = Some(bind_addr);
        self
    }

    /// 构建并启动应用
    pub async fn build(self) -> Result<VirtualCompany> {
        let mut company = VirtualCompany::new(self.local_endpoint);
        if let Some(bind_addr) = self.bind_addr {
            // 服务器在后台运行，不阻塞主流程
            #[allow(unused_variables)]
            let handle = company.start_server(bind_addr).await?;
        }
        Ok(company)
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
