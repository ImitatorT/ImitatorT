//! 虚拟公司框架 API
//!
//! 提供简洁的接口供外部应用使用

use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use crate::protocol::server::{A2AServer, AgentInfo};
use crate::core::agent::{Agent, AgentConfig, AgentManager};
use crate::core::messaging::{Message, MessageBus};
use crate::protocol::router::{AgentConnector, MessageRouter};

/// 虚拟公司框架
///
/// 封装所有框架能力，提供简洁的 API
pub struct VirtualCompany {
    /// Agent 管理器
    agent_manager: AgentManager,
    /// 消息总线
    message_bus: Arc<MessageBus>,
    /// 消息路由器
    router: Arc<MessageRouter>,
    /// A2A 服务端（可选）
    server: Option<Arc<A2AServer>>,
    /// 本地端点地址
    local_endpoint: String,
}

impl VirtualCompany {
    /// 创建新的虚拟公司实例
    ///
    /// # Arguments
    /// * `local_endpoint` - 本地 HTTP 服务端点（如 "http://localhost:8080"）
    pub fn new(local_endpoint: impl Into<String>) -> Self {
        let local_endpoint = local_endpoint.into();
        let message_bus = Arc::new(MessageBus::new());
        let router = Arc::new(MessageRouter::new(
            message_bus.clone(),
            local_endpoint.clone(),
        ));

        Self {
            agent_manager: AgentManager::new(),
            message_bus,
            router,
            server: None,
            local_endpoint,
        }
    }

    /// 启动 HTTP 服务（用于跨节点通信）
    ///
    /// # Arguments
    /// * `bind_addr` - 绑定地址（如 "0.0.0.0:8080"）
    pub async fn start_server(&mut self, bind_addr: SocketAddr) -> Result<()> {
        let server = Arc::new(A2AServer::new(self.message_bus.clone(), bind_addr));

        // 后台启动
        let server_clone = Arc::clone(&server);
        let _handle = tokio::spawn(async move {
            if let Err(e) = server_clone.start().await {
                tracing::error!("A2A server error: {}", e);
            }
        });

        self.server = Some(server);
        info!("VirtualCompany server started on {}", bind_addr);

        Ok(())
    }

    /// 连接到种子节点（用于发现网络中的其他 Agent）
    ///
    /// # Arguments
    /// * `seed_endpoints` - 种子节点地址列表
    pub async fn connect_to_network(&self, seed_endpoints: &[String]) -> Result<()> {
        let connector = AgentConnector::new(self.router.clone(), self.local_endpoint.clone());

        connector.connect_to_network(seed_endpoints).await?;

        info!("Connected to network via {} seeds", seed_endpoints.len());
        Ok(())
    }

    /// 向网络广播自己的存在
    pub async fn announce_presence(&self, node_info: &AgentInfo) -> Result<()> {
        let connector = AgentConnector::new(self.router.clone(), self.local_endpoint.clone());

        connector.announce_presence(node_info).await
    }

    /// 创建 Agent
    ///
    /// # Arguments
    /// * `config` - Agent 配置
    pub async fn create_agent(&self, config: AgentConfig) -> Result<Arc<Agent>> {
        let agent = self.agent_manager.register(config).await?;

        // 注册到路由器
        self.router.register_local_agent(agent.id());

        // 连接消息总线
        // 注意：这里需要可变引用，实际使用时需要调整

        info!("Created agent: {}", agent.id());
        Ok(agent)
    }

    /// 获取 Agent
    pub fn get_agent(&self, id: &str) -> Option<Arc<Agent>> {
        self.agent_manager.get(id)
    }

    /// 列出所有本地 Agent
    pub fn list_agents(&self) -> Vec<Arc<Agent>> {
        self.agent_manager.list()
    }

    /// 注册远程 Agent
    pub fn register_remote_agent(&self, agent_id: &str, endpoint: &str) {
        self.router.register_remote_agent(agent_id, endpoint);
    }

    /// 创建群聊
    ///
    /// # Arguments
    /// * `group_id` - 群聊唯一标识
    /// * `name` - 群聊名称
    /// * `creator` - 创建者 Agent ID
    /// * `members` - 成员 Agent ID 列表
    pub async fn create_group(
        &self,
        group_id: &str,
        name: &str,
        creator: &str,
        members: Vec<String>,
    ) -> Result<String> {
        self.router
            .create_group(group_id, name, creator, members)
            .await
    }

    /// 邀请成员加入群聊
    pub async fn invite_to_group(
        &self,
        group_id: &str,
        inviter: &str,
        invitee: &str,
    ) -> Result<()> {
        self.router
            .invite_to_group(group_id, inviter, invitee)
            .await
    }

    /// 发送私聊消息
    ///
    /// # Arguments
    /// * `from` - 发送者 Agent ID
    /// * `to` - 接收者 Agent ID
    /// * `content` - 消息内容
    pub async fn send_private(&self, from: &str, to: &str, content: &str) -> Result<()> {
        let msg = Message::private(from, to, content);
        self.router.route(msg).await
    }

    /// 发送群聊消息
    ///
    /// # Arguments
    /// * `from` - 发送者 Agent ID
    /// * `group_id` - 群聊 ID
    /// * `content` - 消息内容
    pub async fn send_group(&self, from: &str, group_id: &str, content: &str) -> Result<()> {
        let msg = Message::group(from, group_id, content);
        self.router.route(msg).await
    }

    /// 发送广播消息（全员）
    ///
    /// # Arguments
    /// * `from` - 发送者 Agent ID
    /// * `content` - 消息内容
    pub async fn broadcast(&self, from: &str, content: &str) -> Result<()> {
        let msg = Message::broadcast(from, content);
        self.router.route(msg).await
    }

    /// 获取消息总线（用于高级用法）
    pub fn message_bus(&self) -> Arc<MessageBus> {
        self.message_bus.clone()
    }

    /// 获取消息路由器（用于高级用法）
    pub fn router(&self) -> Arc<MessageRouter> {
        self.router.clone()
    }

    /// 获取本地端点
    pub fn local_endpoint(&self) -> &str {
        &self.local_endpoint
    }
}

/// 应用构建器
///
/// 帮助应用快速搭建虚拟公司环境
pub struct AppBuilder {
    local_endpoint: String,
    bind_addr: Option<SocketAddr>,
    seed_endpoints: Vec<String>,
}

impl AppBuilder {
    /// 创建新的应用构建器
    pub fn new(local_endpoint: impl Into<String>) -> Self {
        Self {
            local_endpoint: local_endpoint.into(),
            bind_addr: None,
            seed_endpoints: vec![],
        }
    }

    /// 设置 HTTP 服务绑定地址
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.bind_addr = Some(addr);
        self
    }

    /// 添加种子节点
    pub fn seed(mut self, endpoint: impl Into<String>) -> Self {
        self.seed_endpoints.push(endpoint.into());
        self
    }

    /// 构建并启动
    pub async fn build(self) -> Result<VirtualCompany> {
        let mut company = VirtualCompany::new(self.local_endpoint);

        // 启动 HTTP 服务
        if let Some(addr) = self.bind_addr {
            company.start_server(addr).await?;
        }

        // 连接到网络
        if !self.seed_endpoints.is_empty() {
            company.connect_to_network(&self.seed_endpoints).await?;
        }

        Ok(company)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_company_creation() {
        let company = VirtualCompany::new("http://localhost:8080");
        assert_eq!(company.local_endpoint(), "http://localhost:8080");
        assert!(company.list_agents().is_empty());
    }

    #[test]
    fn test_app_builder() {
        let builder = AppBuilder::new("http://localhost:8080")
            .bind("0.0.0.0:8080".parse().unwrap())
            .seed("http://seed:8080");

        assert_eq!(builder.local_endpoint, "http://localhost:8080");
        assert!(builder.bind_addr.is_some());
        assert_eq!(builder.seed_endpoints.len(), 1);
    }
}
