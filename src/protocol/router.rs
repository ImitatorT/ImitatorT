//! 消息路由器
//!
//! 负责将消息路由到正确的目的地：
//! - 本地路由：消息在本地 Agent 之间传递
//! - 远程路由：消息通过 HTTP 发送到其他 Agent
//! - 智能路由：根据 Agent 位置自动选择路由方式

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::core::messaging::{Message, MessageBus, MessageType};
use crate::protocol::client::A2AClient;
use crate::protocol::server::AgentInfo;

/// 路由目标
#[derive(Debug, Clone)]
pub enum RouteTarget {
    /// 本地目标
    Local,
    /// 远程目标
    Remote(String),
}

/// 消息路由器
///
/// 维护 Agent 位置信息，智能选择路由方式
pub struct MessageRouter {
    /// 本地消息总线
    local_bus: Arc<MessageBus>,
    /// A2A 客户端（用于远程路由）
    client: A2AClient,
    /// 本地 Agent 列表
    local_agents: dashmap::DashMap<String, ()>,
    /// 远程 Agent 列表（id -> endpoint）
    remote_agents: dashmap::DashMap<String, String>,
}

impl MessageRouter {
    /// 创建新的消息路由器
    pub fn new(local_bus: Arc<MessageBus>, local_endpoint: impl Into<String>) -> Self {
        let client = A2AClient::new(local_endpoint);

        Self {
            local_bus,
            client,
            local_agents: dashmap::DashMap::new(),
            remote_agents: dashmap::DashMap::new(),
        }
    }

    /// 注册本地 Agent
    pub fn register_local_agent(&self, agent_id: &str) {
        self.local_agents.insert(agent_id.to_string(), ());
        self.remote_agents.remove(agent_id);
        info!("Registered local agent: {}", agent_id);
    }

    /// 注册远程 Agent
    pub fn register_remote_agent(&self, agent_id: &str, endpoint: &str) {
        self.remote_agents
            .insert(agent_id.to_string(), endpoint.to_string());
        self.local_agents.remove(agent_id);
        info!("Registered remote agent: {} at {}", agent_id, endpoint);
    }

    /// 注销 Agent
    pub fn unregister_agent(&self, agent_id: &str) {
        self.local_agents.remove(agent_id);
        self.remote_agents.remove(agent_id);
        info!("Unregistered agent: {}", agent_id);
    }

    /// 判断 Agent 是否在本地
    pub fn is_local(&self, agent_id: &str) -> bool {
        self.local_agents.contains_key(agent_id)
    }

    /// 判断 Agent 是否已知
    pub fn is_known(&self, agent_id: &str) -> bool {
        self.is_local(agent_id) || self.remote_agents.contains_key(agent_id)
    }

    /// 获取 Agent 的路由目标
    pub fn get_route_target(&self, agent_id: &str) -> Option<RouteTarget> {
        if self.is_local(agent_id) {
            Some(RouteTarget::Local)
        } else {
            self.remote_agents
                .get(agent_id)
                .map(|endpoint| RouteTarget::Remote(endpoint.clone()))
        }
    }

    /// 列出所有本地 Agent
    pub fn list_local_agents(&self) -> Vec<String> {
        self.local_agents.iter().map(|e| e.key().clone()).collect()
    }

    /// 列出所有远程 Agent
    pub fn list_remote_agents(&self) -> Vec<(String, String)> {
        self.remote_agents
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect()
    }

    /// 路由私聊消息
    async fn route_private(&self, message: &Message) -> Result<()> {
        let to = message
            .to
            .first()
            .context("Private message must have a recipient")?;

        match self.get_route_target(to) {
            Some(RouteTarget::Local) => {
                // 本地路由
                self.local_bus.send_private(message.clone()).await?;
                debug!("Routed private message locally to {}", to);
            }
            Some(RouteTarget::Remote(endpoint)) => {
                // 远程路由
                self.client
                    .send_private(&endpoint, &message.from, to, &message.content)
                    .await?;
                debug!("Routed private message remotely to {} via {}", to, endpoint);
            }
            None => {
                return Err(anyhow::anyhow!("Unknown recipient: {}", to));
            }
        }

        Ok(())
    }

    /// 路由群聊消息
    async fn route_group(&self, message: &Message) -> Result<()> {
        let group_id = message
            .to
            .first()
            .context("Group message must have a group id")?;

        // 获取群聊信息
        let group = self
            .local_bus
            .get_group(group_id)
            .await
            .context("Group not found")?;

        // 验证发送者在群聊中
        if !group.has_member(&message.from) {
            return Err(anyhow::anyhow!("Sender is not in the group"));
        }

        // 向群聊中的每个成员发送消息
        for member in &group.members {
            // 跳过发送者自己
            if member == &message.from {
                continue;
            }

            let member_msg = Message {
                id: message.id.clone(),
                msg_type: MessageType::Group,
                from: message.from.clone(),
                to: vec![group_id.clone()],
                content: message.content.clone(),
                timestamp: message.timestamp,
                metadata: message.metadata.clone(),
            };

            match self.get_route_target(member) {
                Some(RouteTarget::Local) => {
                    if let Err(e) = self.local_bus.send_group(member_msg).await {
                        warn!(
                            "Failed to send group message to local member {}: {}",
                            member, e
                        );
                    }
                }
                Some(RouteTarget::Remote(endpoint)) => {
                    if let Err(e) = self
                        .client
                        .send_group(&endpoint, &message.from, group_id, &message.content)
                        .await
                    {
                        warn!(
                            "Failed to send group message to remote member {}: {}",
                            member, e
                        );
                    }
                }
                None => {
                    warn!("Unknown member in group: {}", member);
                }
            }
        }

        info!(
            "Routed group message to {} members in group {}",
            group.members.len() - 1,
            group_id
        );
        Ok(())
    }

    /// 路由广播消息
    async fn route_broadcast(&self, message: &Message) -> Result<()> {
        // 本地广播
        self.local_bus.broadcast(message.clone())?;

        // 向所有远程 Agent 发送广播
        for entry in self.remote_agents.iter() {
            let agent_id = entry.key();
            let endpoint = entry.value();

            if let Err(e) = self
                .client
                .send_broadcast(endpoint, &message.from, &message.content)
                .await
            {
                warn!("Failed to send broadcast to {}: {}", agent_id, e);
            }
        }

        info!(
            "Routed broadcast message to {} remote agents",
            self.remote_agents.len()
        );
        Ok(())
    }

    /// 路由消息（自动选择路由方式）
    pub async fn route(&self, message: Message) -> Result<()> {
        match message.msg_type {
            MessageType::Private => self.route_private(&message).await,
            MessageType::Group => self.route_group(&message).await,
            MessageType::Broadcast | MessageType::System => self.route_broadcast(&message).await,
        }
    }

    /// 创建群聊（跨 Agent）
    pub async fn create_group(
        &self,
        group_id: &str,
        name: &str,
        creator: &str,
        members: Vec<String>,
    ) -> Result<String> {
        // 在本地创建群聊
        let group_id = self
            .local_bus
            .create_group(group_id, name, creator, members.clone())
            .await?;

        // 通知远程成员
        for member in &members {
            if member == creator {
                continue;
            }

            if let Some(RouteTarget::Remote(endpoint)) = self.get_route_target(member) {
                // 远程 Agent 需要在自己的节点上创建群聊
                if let Err(e) = self
                    .client
                    .create_group(&endpoint, &group_id, name, creator, members.clone())
                    .await
                {
                    warn!(
                        "Failed to notify remote agent {} about group creation: {}",
                        member, e
                    );
                }
            }
        }

        info!(
            "Created cross-agent group: {} with {} members",
            group_id,
            members.len()
        );
        Ok(group_id)
    }

    /// 邀请成员加入群聊（跨 Agent）
    pub async fn invite_to_group(
        &self,
        group_id: &str,
        inviter: &str,
        invitee: &str,
    ) -> Result<()> {
        // 本地邀请
        self.local_bus
            .invite_to_group(group_id, inviter, invitee)
            .await?;

        // 如果邀请的是远程 Agent，通知其节点
        if let Some(RouteTarget::Remote(endpoint)) = self.get_route_target(invitee) {
            self.client
                .invite_to_group(&endpoint, group_id, inviter, invitee)
                .await?;
        }

        info!("Invited {} to group {} by {}", invitee, group_id, inviter);
        Ok(())
    }
}

/// 智能 Agent 连接器
///
/// 帮助 Agent 自动发现和连接其他 Agent
pub struct AgentConnector {
    router: Arc<MessageRouter>,
    local_endpoint: String,
}

impl AgentConnector {
    /// 创建新的连接器
    pub fn new(router: Arc<MessageRouter>, local_endpoint: impl Into<String>) -> Self {
        Self {
            router,
            local_endpoint: local_endpoint.into(),
        }
    }

    /// 连接到种子节点并发现网络
    pub async fn connect_to_network(&self, seed_endpoints: &[String]) -> Result<()> {
        for endpoint in seed_endpoints {
            match self.discover_from_seed(endpoint).await {
                Ok(count) => {
                    info!("Discovered {} agents from seed {}", count, endpoint);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to connect to seed {}: {}", endpoint, e);
                }
            }
        }

        Err(anyhow::anyhow!("Failed to connect to any seed node"))
    }

    /// 从种子节点发现 Agent
    async fn discover_from_seed(&self, seed_endpoint: &str) -> Result<usize> {
        let client = A2AClient::new(&self.local_endpoint);

        // 健康检查
        if !client.health_check(seed_endpoint).await? {
            return Err(anyhow::anyhow!("Seed node is not healthy"));
        }

        // 发现 Agent
        let agents = client.discover_agents(seed_endpoint).await?;
        let count = agents.len();

        // 注册到路由器
        for agent in agents {
            if agent.endpoint != self.local_endpoint {
                self.router
                    .register_remote_agent(&agent.id, &agent.endpoint);
            }
        }

        Ok(count)
    }

    /// 向网络广播自己的存在
    pub async fn announce_presence(&self, agent_info: &AgentInfo) -> Result<()> {
        let client = A2AClient::new(&self.local_endpoint);

        for (agent_id, endpoint) in self.router.list_remote_agents() {
            if let Err(e) = client.register_to_remote(&endpoint, agent_info).await {
                debug!("Failed to announce to {}: {}", agent_id, e);
            }
        }

        info!(
            "Announced presence to {} remote agents",
            self.router.list_remote_agents().len()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_router_creation() {
        let bus = Arc::new(MessageBus::new());
        let router = MessageRouter::new(bus, "http://localhost:8080");

        assert!(router.list_local_agents().is_empty());
        assert!(router.list_remote_agents().is_empty());
    }

    #[test]
    fn test_register_local_agent() {
        let bus = Arc::new(MessageBus::new());
        let router = MessageRouter::new(bus, "http://localhost:8080");

        router.register_local_agent("agent-001");

        assert!(router.is_local("agent-001"));
        assert!(!router.is_local("agent-002"));
        assert_eq!(router.list_local_agents().len(), 1);
    }

    #[test]
    fn test_register_remote_agent() {
        let bus = Arc::new(MessageBus::new());
        let router = MessageRouter::new(bus, "http://localhost:8080");

        router.register_remote_agent("agent-002", "http://remote:8080");

        assert!(!router.is_local("agent-002"));
        assert!(router.is_known("agent-002"));

        let remotes = router.list_remote_agents();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].0, "agent-002");
        assert_eq!(remotes[0].1, "http://remote:8080");
    }

    #[test]
    fn test_get_route_target() {
        let bus = Arc::new(MessageBus::new());
        let router = MessageRouter::new(bus, "http://localhost:8080");

        router.register_local_agent("local-agent");
        router.register_remote_agent("remote-agent", "http://remote:8080");

        match router.get_route_target("local-agent").unwrap() {
            RouteTarget::Local => {}
            _ => panic!("Expected local route"),
        }

        match router.get_route_target("remote-agent").unwrap() {
            RouteTarget::Remote(endpoint) => assert_eq!(endpoint, "http://remote:8080"),
            _ => panic!("Expected remote route"),
        }

        assert!(router.get_route_target("unknown").is_none());
    }

    #[tokio::test]
    async fn test_route_private_local() {
        let bus = Arc::new(MessageBus::new());
        let router = MessageRouter::new(bus.clone(), "http://localhost:8080");

        // 注册本地 Agent
        router.register_local_agent("sender");
        router.register_local_agent("receiver");

        // 创建接收者的消息接收器
        let mut receiver = bus.register_agent("receiver");

        // 发送消息
        let msg = Message::private("sender", "receiver", "hello");
        router.route(msg).await.unwrap();

        // 验证接收
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.content, "hello");
        assert_eq!(received.from, "sender");
    }

    #[tokio::test]
    async fn test_route_broadcast_local() {
        let bus = Arc::new(MessageBus::new());
        let router = MessageRouter::new(bus.clone(), "http://localhost:8080");

        router.register_local_agent("broadcaster");
        router.register_local_agent("listener");

        let mut listener = bus.register_agent("listener");

        let msg = Message::broadcast("broadcaster", "announcement");
        router.route(msg).await.unwrap();

        let received = listener.recv().await.unwrap();
        assert_eq!(received.content, "announcement");
    }
}
