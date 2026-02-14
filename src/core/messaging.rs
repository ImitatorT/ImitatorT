//! 消息通信层
//!
//! 提供 Agent 间的消息传递能力：
//! - 私聊（1对1）：Agent 自主发起
//! - 群聊（1对多）：Agent 自主创建并邀请
//! - 广播（1对所有）
//!
//! 设计原则：框架只提供通信能力，所有聊天行为由 Agent 自主决策

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    /// 私聊
    Private,
    /// 群聊
    Group,
    /// 广播
    Broadcast,
    /// 系统消息
    System,
}

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub msg_type: MessageType,
    pub from: String,
    /// 接收者列表（私聊为单个人，群聊为群聊ID）
    pub to: Vec<String>,
    pub content: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Message {
    /// 创建私聊消息
    pub fn private(from: &str, to: &str, content: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            msg_type: MessageType::Private,
            from: from.to_string(),
            to: vec![to.to_string()],
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        }
    }

    /// 创建群聊消息
    pub fn group(from: &str, group_id: &str, content: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            msg_type: MessageType::Group,
            from: from.to_string(),
            to: vec![group_id.to_string()],
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        }
    }

    /// 创建广播消息
    pub fn broadcast(from: &str, content: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            msg_type: MessageType::Broadcast,
            from: from.to_string(),
            to: vec![], // 空表示所有
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        }
    }

    /// 创建系统消息
    pub fn system(content: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            msg_type: MessageType::System,
            from: "system".to_string(),
            to: vec![], // 空表示所有
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        }
    }
}

/// 群聊信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    pub creator: String,
    pub members: Vec<String>,
    pub created_at: i64,
}

impl GroupInfo {
    /// 创建新群聊
    pub fn new(id: &str, name: &str, creator: &str, members: Vec<String>) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            creator: creator.to_string(),
            members,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// 检查成员是否在群聊中
    pub fn has_member(&self, agent_id: &str) -> bool {
        self.members.contains(&agent_id.to_string())
    }

    /// 添加成员
    pub fn add_member(&mut self, agent_id: &str) {
        if !self.has_member(agent_id) {
            self.members.push(agent_id.to_string());
        }
    }

    /// 移除成员
    pub fn remove_member(&mut self, agent_id: &str) {
        self.members.retain(|m| m != agent_id);
    }
}

/// 消息总线
///
/// 负责消息的路由和分发
/// 所有群聊由 Agent 动态创建，不预设任何群聊结构
pub struct MessageBus {
    /// 广播通道（用于广播消息和公司全员群）
    broadcast_tx: broadcast::Sender<Message>,
    /// 广播通道 holder，防止 channel 被关闭
    #[allow(dead_code)]
    broadcast_rx_holder: broadcast::Receiver<Message>,
    /// 私聊通道映射
    private_txs: dashmap::DashMap<String, mpsc::Sender<Message>>,
    /// 群聊信息映射
    groups: Arc<RwLock<std::collections::HashMap<String, GroupInfo>>>,
    /// 群聊通道映射
    group_txs: dashmap::DashMap<String, broadcast::Sender<Message>>,
}

impl MessageBus {
    /// 创建新的消息总线
    ///
    /// 初始化时只创建公司全员群（广播通道）
    pub fn new() -> Self {
        let (broadcast_tx, broadcast_rx) = broadcast::channel(1000);

        Self {
            broadcast_tx,
            broadcast_rx_holder: broadcast_rx,
            private_txs: dashmap::DashMap::new(),
            groups: Arc::new(RwLock::new(std::collections::HashMap::new())),
            group_txs: dashmap::DashMap::new(),
        }
    }

    /// 注册 Agent 到消息总线
    ///
    /// 返回一个接收器，Agent 可以用它接收：
    /// - 私聊消息
    /// - 广播消息（公司全员群）
    pub fn register_agent(&self, agent_id: &str) -> AgentMessageReceiver {
        let (private_tx, private_rx) = mpsc::channel(100);
        self.private_txs.insert(agent_id.to_string(), private_tx);

        let broadcast_rx = self.broadcast_tx.subscribe();

        info!("Registered agent to message bus: {}", agent_id);

        AgentMessageReceiver {
            agent_id: agent_id.to_string(),
            private_rx,
            broadcast_rx,
            group_rxs: Vec::new(),
        }
    }

    /// 注销 Agent
    pub fn unregister_agent(&self, agent_id: &str) {
        self.private_txs.remove(agent_id);

        // 从所有群聊中移除
        let group_ids: Vec<String> = {
            let groups = self.groups.blocking_read();
            groups
                .iter()
                .filter(|(_, group)| group.has_member(agent_id))
                .map(|(group_id, _)| group_id.clone())
                .collect()
        };

        for group_id in group_ids {
            if let Ok(mut groups) = self.groups.try_write() {
                if let Some(g) = groups.get_mut(&group_id) {
                    g.remove_member(agent_id);
                }
            }
        }

        info!("Unregistered agent from message bus: {}", agent_id);
    }

    /// 创建群聊（由 Agent 自主发起）
    ///
    /// 返回群聊 ID
    pub async fn create_group(
        &self,
        group_id: &str,
        name: &str,
        creator: &str,
        members: Vec<String>,
    ) -> Result<String> {
        // 验证创建者是否已注册
        if !self.private_txs.contains_key(creator) {
            return Err(anyhow::anyhow!("Creator not registered: {}", creator));
        }

        // 验证所有成员是否已注册
        for member in &members {
            if !self.private_txs.contains_key(member) {
                return Err(anyhow::anyhow!("Member not registered: {}", member));
            }
        }

        let group = GroupInfo::new(group_id, name, creator, members);
        let (tx, _rx) = broadcast::channel(100);

        {
            let mut groups = self.groups.write().await;
            groups.insert(group_id.to_string(), group);
        }

        self.group_txs.insert(group_id.to_string(), tx);

        info!("Created group: {} by {}", group_id, creator);
        Ok(group_id.to_string())
    }

    /// 删除群聊（只有创建者可以删除）
    pub async fn delete_group(&self, group_id: &str, requester: &str) -> Result<()> {
        let groups = self.groups.read().await;

        if let Some(group) = groups.get(group_id) {
            if group.creator != requester {
                return Err(anyhow::anyhow!("Only creator can delete group"));
            }
        } else {
            return Err(anyhow::anyhow!("Group not found: {}", group_id));
        }

        drop(groups);

        let mut groups = self.groups.write().await;
        groups.remove(group_id);
        self.group_txs.remove(group_id);

        info!("Deleted group: {} by {}", group_id, requester);
        Ok(())
    }

    /// 邀请成员加入群聊
    pub async fn invite_to_group(
        &self,
        group_id: &str,
        inviter: &str,
        invitee: &str,
    ) -> Result<()> {
        // 验证被邀请者是否已注册
        if !self.private_txs.contains_key(invitee) {
            return Err(anyhow::anyhow!("Invitee not registered: {}", invitee));
        }

        let mut groups = self.groups.write().await;

        if let Some(group) = groups.get_mut(group_id) {
            // 验证邀请者是否在群聊中
            if !group.has_member(inviter) {
                return Err(anyhow::anyhow!("Inviter not in group"));
            }

            group.add_member(invitee);
            info!("{} invited {} to group {}", inviter, invitee, group_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Group not found: {}", group_id))
        }
    }

    /// 退出群聊
    pub async fn leave_group(&self, group_id: &str, agent_id: &str) -> Result<()> {
        let mut groups = self.groups.write().await;

        if let Some(group) = groups.get_mut(group_id) {
            group.remove_member(agent_id);
            info!("{} left group {}", agent_id, group_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Group not found: {}", group_id))
        }
    }

    /// 获取群聊信息
    pub async fn get_group(&self, group_id: &str) -> Option<GroupInfo> {
        let groups = self.groups.read().await;
        groups.get(group_id).cloned()
    }

    /// 列出 Agent 所在的所有群聊
    pub async fn list_agent_groups(&self, agent_id: &str) -> Vec<GroupInfo> {
        let groups = self.groups.read().await;
        groups
            .values()
            .filter(|g| g.has_member(agent_id))
            .cloned()
            .collect()
    }

    /// 发送私聊消息
    pub async fn send_private(&self, message: Message) -> Result<()> {
        if message.msg_type != MessageType::Private {
            return Err(anyhow::anyhow!("Expected private message"));
        }

        let to = message
            .to
            .first()
            .context("Private message must have a recipient")?
            .clone();

        if let Some(tx) = self.private_txs.get(&to) {
            tx.send(message)
                .await
                .context("Failed to send private message")?;
            debug!("Sent private message to {}", to);
        } else {
            warn!("Recipient not found: {}", to);
            return Err(anyhow::anyhow!("Recipient not found: {}", to));
        }

        Ok(())
    }

    /// 发送群聊消息
    pub async fn send_group(&self, message: Message) -> Result<()> {
        if message.msg_type != MessageType::Group {
            return Err(anyhow::anyhow!("Expected group message"));
        }

        let group_id = message
            .to
            .first()
            .context("Group message must have a group id")?
            .clone();

        // 验证发送者是否在群聊中
        let groups = self.groups.read().await;
        if let Some(group) = groups.get(&group_id) {
            if !group.has_member(&message.from) {
                return Err(anyhow::anyhow!("Sender not in group"));
            }
        }
        drop(groups);

        if let Some(tx) = self.group_txs.get(&group_id) {
            tx.send(message).context("Failed to send group message")?;
            debug!("Sent group message to {}", group_id);
            Ok(())
        } else {
            warn!("Group not found: {}", group_id);
            Err(anyhow::anyhow!("Group not found: {}", group_id))
        }
    }

    /// 发送广播消息（公司全员群）
    pub fn broadcast(&self, message: Message) -> Result<usize> {
        if message.msg_type != MessageType::Broadcast && message.msg_type != MessageType::System {
            return Err(anyhow::anyhow!("Expected broadcast or system message"));
        }

        let count = self.broadcast_tx.send(message)?;
        debug!("Broadcasted message to {} receivers", count);
        Ok(count)
    }

    /// 发送消息（自动根据类型路由）
    pub async fn send(&self, message: Message) -> Result<()> {
        match message.msg_type {
            MessageType::Private => self.send_private(message).await,
            MessageType::Group => self.send_group(message).await,
            MessageType::Broadcast | MessageType::System => {
                self.broadcast(message)?;
                Ok(())
            }
        }
    }

    /// 订阅群聊消息
    pub fn subscribe_group(&self, group_id: &str) -> Option<broadcast::Receiver<Message>> {
        self.group_txs.get(group_id).map(|tx| tx.subscribe())
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent 消息接收器
///
/// 聚合所有消息源：私聊、广播、群聊
pub struct AgentMessageReceiver {
    agent_id: String,
    private_rx: mpsc::Receiver<Message>,
    broadcast_rx: broadcast::Receiver<Message>,
    group_rxs: Vec<(String, broadcast::Receiver<Message>)>,
}

impl AgentMessageReceiver {
    /// 获取 Agent ID
    pub fn agent_id(&self) -> &str {
        &self.agent_id
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

    /// 离开群聊（取消订阅）
    pub fn leave_group(&mut self, group_id: &str) {
        self.group_rxs.retain(|(id, _)| id != group_id);
    }

    /// 接收下一条消息（从任何来源）
    ///
    /// 优先级：私聊 > 群聊 > 广播
    pub async fn recv(&mut self) -> Option<Message> {
        // 先检查私聊
        if let Ok(msg) = self.private_rx.try_recv() {
            return Some(msg);
        }

        // 再检查群聊
        for (_group_id, rx) in &mut self.group_rxs {
            if let Ok(msg) = rx.try_recv() {
                // 不接收自己发送的消息
                if msg.from != self.agent_id {
                    return Some(msg);
                }
            }
        }

        // 最后检查广播
        if let Ok(msg) = self.broadcast_rx.try_recv() {
            // 不接收自己发送的广播
            if msg.from != self.agent_id {
                return Some(msg);
            }
        }

        // 如果没有消息，等待私聊通道
        self.private_rx.recv().await
    }

    /// 尝试接收消息（非阻塞）
    pub fn try_recv(&mut self) -> Option<Message> {
        // 检查私聊
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

        // 检查广播
        if let Ok(msg) = self.broadcast_rx.try_recv() {
            if msg.from != self.agent_id {
                return Some(msg);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_message_creation() {
        let msg = Message::private("alice", "bob", "hello");
        assert_eq!(msg.msg_type, MessageType::Private);
        assert_eq!(msg.from, "alice");
        assert_eq!(msg.to, vec!["bob"]);
        assert_eq!(msg.content, "hello");
        assert!(!msg.id.is_empty());
        assert!(msg.timestamp > 0);
    }

    #[test]
    fn test_group_message_creation() {
        let msg = Message::group("alice", "group-1", "hello group");
        assert_eq!(msg.msg_type, MessageType::Group);
        assert_eq!(msg.from, "alice");
        assert_eq!(msg.to, vec!["group-1"]);
        assert_eq!(msg.content, "hello group");
    }

    #[test]
    fn test_broadcast_message_creation() {
        let msg = Message::broadcast("system", "announcement");
        assert_eq!(msg.msg_type, MessageType::Broadcast);
        assert_eq!(msg.from, "system");
        assert!(msg.to.is_empty());
    }

    #[test]
    fn test_group_info_creation() {
        let members = vec!["alice".to_string(), "bob".to_string()];
        let group = GroupInfo::new("group-1", "Test Group", "alice", members);

        assert_eq!(group.id, "group-1");
        assert_eq!(group.name, "Test Group");
        assert_eq!(group.creator, "alice");
        assert!(group.has_member("alice"));
        assert!(group.has_member("bob"));
        assert!(!group.has_member("charlie"));
    }

    #[test]
    fn test_group_info_add_remove_member() {
        let members = vec!["alice".to_string()];
        let mut group = GroupInfo::new("group-1", "Test", "alice", members);

        group.add_member("bob");
        assert!(group.has_member("bob"));

        group.remove_member("alice");
        assert!(!group.has_member("alice"));
    }

    #[test]
    fn test_message_bus_creation() {
        let bus = MessageBus::new();
        assert!(bus.private_txs.is_empty());
        assert!(bus.group_txs.is_empty());
    }
}
