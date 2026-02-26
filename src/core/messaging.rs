//! 消息通信层
//!
//! 提供 Agent 间的消息传递能力：私聊、群聊、广播

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, info, warn};

use crate::domain::{Group, Message, MessageTarget};

/// 消息总线
///
/// 负责消息的路由和分发，纯内存实现
pub struct MessageBus {
    /// 私聊通道映射
    private_txs: dashmap::DashMap<String, mpsc::Sender<Message>>,
    /// 群聊信息映射
    groups: Arc<RwLock<std::collections::HashMap<String, Group>>>,
    /// 群聊通道映射
    group_txs: dashmap::DashMap<String, broadcast::Sender<Message>>,
}

impl MessageBus {
    /// 创建新的消息总线
    pub fn new() -> Self {
        Self {
            private_txs: dashmap::DashMap::new(),
            groups: Arc::new(RwLock::new(std::collections::HashMap::new())),
            group_txs: dashmap::DashMap::new(),
        }
    }

    /// 注册 Agent 到消息总线
    pub fn register(&self, agent_id: &str) -> mpsc::Receiver<Message> {
        let (tx, rx) = mpsc::channel(100);
        self.private_txs.insert(agent_id.to_string(), tx);

        info!("Registered agent to message bus: {}", agent_id);
        rx
    }

    /// 注销 Agent
    pub fn unregister(&self, agent_id: &str) {
        self.private_txs.remove(agent_id);
        info!("Unregistered agent from message bus: {}", agent_id);
    }

    /// 创建群聊
    pub async fn create_group(
        &self,
        id: &str,
        name: &str,
        creator_id: &str,
        members: Vec<String>,
    ) -> Result<()> {
        // 验证创建者
        if !self.private_txs.contains_key(creator_id) {
            return Err(anyhow::anyhow!("Creator not registered: {}", creator_id));
        }

        let group = Group::new(id, name, creator_id, members);
        let (tx, _) = broadcast::channel(100);

        {
            let mut groups = self.groups.write().await;
            groups.insert(id.to_string(), group);
        }

        self.group_txs.insert(id.to_string(), tx);

        info!("Created group: {} by {}", id, creator_id);
        Ok(())
    }

    /// 发送消息（自动路由）
    pub async fn send(&self, message: Message) -> Result<()> {
        let target = message.to.clone();
        match target {
            MessageTarget::Direct(agent_id) => self.send_private(message, &agent_id).await,
            MessageTarget::Group(group_id) => self.send_group(message, &group_id).await,
        }
    }

    /// 发送私聊消息
    async fn send_private(&self, message: Message, to: &str) -> Result<()> {
        if let Some(tx) = self.private_txs.get(to) {
            tx.send(message)
                .await
                .context("Failed to send private message")?;
            debug!("Sent private message to {}", to);
            Ok(())
        } else {
            warn!("Recipient not found: {}", to);
            Err(anyhow::anyhow!("Recipient not found: {}", to))
        }
    }

    /// 发送群聊消息
    async fn send_group(&self, message: Message, group_id: &str) -> Result<()> {
        if let Some(tx) = self.group_txs.get(group_id) {
            tx.send(message).context("Failed to send group message")?;
            debug!("Sent group message to {}", group_id);
            Ok(())
        } else {
            warn!("Group not found: {}", group_id);
            Err(anyhow::anyhow!("Group not found: {}", group_id))
        }
    }

    /// 订阅群聊消息
    pub fn subscribe_group(&self, group_id: &str) -> Option<broadcast::Receiver<Message>> {
        self.group_txs.get(group_id).map(|tx| tx.subscribe())
    }

    /// 获取群组信息
    pub async fn get_group(&self, group_id: &str) -> Option<Group> {
        let groups = self.groups.read().await;
        groups.get(group_id).cloned()
    }

    /// 列出Agent所在的所有群组
    pub async fn list_agent_groups(&self, agent_id: &str) -> Vec<Group> {
        let groups = self.groups.read().await;
        groups
            .values()
            .filter(|g| g.has_member(agent_id))
            .cloned()
            .collect()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 消息接收器
///
/// 聚合私聊和群聊消息源
pub struct MessageReceiver {
    agent_id: String,
    private_rx: mpsc::Receiver<Message>,
    group_rxs: Vec<(String, broadcast::Receiver<Message>)>,
}

impl MessageReceiver {
    /// 创建新的消息接收器
    pub fn new(agent_id: String, private_rx: mpsc::Receiver<Message>) -> Self {
        Self {
            agent_id,
            private_rx,
            group_rxs: Vec::new(),
        }
    }

    /// 加入群聊（订阅群聊消息）
    pub fn join_group(&mut self, group_id: &str, bus: &MessageBus) -> Result<()> {
        if let Some(rx) = bus.subscribe_group(group_id) {
            self.group_rxs.push((group_id.to_string(), rx));
            Ok(())
        } else {
            Err(anyhow::anyhow!("Group not found: {}", group_id))
        }
    }

    /// 离开群聊
    pub fn leave_group(&mut self, group_id: &str) {
        self.group_rxs.retain(|(id, _)| id != group_id);
    }

    /// 接收下一条消息（阻塞）
    pub async fn recv(&mut self) -> Option<Message> {
        // 优先检查私聊
        if let Ok(msg) = self.private_rx.try_recv() {
            return Some(msg);
        }

        // 检查群聊
        for (_group_id, rx) in &mut self.group_rxs {
            if let Ok(msg) = rx.try_recv() {
                if msg.from != self.agent_id {
                    return Some(msg);
                }
            }
        }

        // 等待私聊
        self.private_rx.recv().await
    }

    /// 尝试接收消息（非阻塞）
    pub fn try_recv(&mut self) -> Option<Message> {
        if let Ok(msg) = self.private_rx.try_recv() {
            return Some(msg);
        }

        for (_group_id, rx) in &mut self.group_rxs {
            if let Ok(msg) = rx.try_recv() {
                if msg.from != self.agent_id {
                    return Some(msg);
                }
            }
        }

        None
    }
}
