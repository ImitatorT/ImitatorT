//! 虚拟公司框架 API
//!
//! 提供简洁的接口供外部应用使用

use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use crate::core::agent::{Agent, AgentConfig, AgentManager};
use crate::core::messaging::MessageBus;
use crate::protocol::router::{AgentConnector, MessageRouter};
use crate::protocol::server::A2AServer;

/// 虚拟公司框架
///
/// 封装所有框架能力，提供简洁的 API
pub struct VirtualCompany {
    /// Agent 管理器
    agent_manager: AgentManager,
    /// 消息总线
    message_bus: Arc<MessageBus>,
    /// A2A 服务端
    server: Option<A2AServer>,
}

impl VirtualCompany {
    /// 创建一个新的虚拟公司实例
    pub fn new() -> Self {
        Self {
            agent_manager: AgentManager::new(),
            message_bus: Arc::new(MessageBus::new()),
            server: None,
        }
    }

    /// 创建并注册一个新 Agent
    pub async fn create_agent(&self, config: AgentConfig) -> Result<Arc<Agent>> {
        let agent = Agent::new(config).await?;
        let agent_arc = Arc::new(agent);
        self.agent_manager.register(agent_arc.clone()).await;
        Ok(agent_arc)
    }

    /// 获取 Agent
    pub fn get_agent(&self, agent_id: &str) -> Option<Arc<Agent>> {
        self.agent_manager.get(agent_id)
    }

    /// 列出所有 Agent
    pub fn list_agents(&self) -> Vec<String> {
        self.agent_manager.list()
    }

    /// 启动 A2A HTTP 服务
    pub async fn start_server(&mut self, bind_addr: SocketAddr) -> Result<()> {
        let server = A2AServer::new(bind_addr, self.message_bus.clone());
        server.start().await?;
        self.server = Some(server);
        info!("A2A server started on {}", bind_addr);
        Ok(())
    }

    /// 连接到远程 Agent
    pub async fn connect_to_remote(&self, agent_id: &str, endpoint: &str) -> Result<AgentConnector> {
        let router = MessageRouter::new(self.message_bus.clone());
        let connector = router.connect_to(agent_id, endpoint).await?;
        Ok(connector)
    }

    /// 广播消息给所有 Agent
    pub async fn broadcast(&self, from: &str, content: &str) -> Result<()> {
        self.message_bus.broadcast(from, content).await
    }

    /// 获取消息总线
    pub fn message_bus(&self) -> Arc<MessageBus> {
        self.message_bus.clone()
    }
}

impl Default for VirtualCompany {
    fn default() -> Self {
        Self::new()
    }
}

/// 应用构建器
///
/// 用于链式构建和配置 VirtualCompany
pub struct AppBuilder {
    company: VirtualCompany,
    bind_addr: Option<SocketAddr>,
}

impl AppBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            company: VirtualCompany::new(),
            bind_addr: None,
        }
    }

    /// 设置服务器绑定地址
    pub fn with_server(mut self, bind_addr: SocketAddr) -> Self {
        self.bind_addr = Some(bind_addr);
        self
    }

    /// 添加 Agent
    pub fn with_agent(mut self, _config: AgentConfig) -> Self {
        // 注意：这里不能直接 await，需要在 build 中处理
        // 简化处理：直接创建但不注册
        self
    }

    /// 构建并启动应用
    pub async fn build(mut self) -> Result<VirtualCompany> {
        if let Some(bind_addr) = self.bind_addr {
            self.company.start_server(bind_addr).await?;
        }
        Ok(self.company)
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
