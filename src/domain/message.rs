//! 消息领域实体
//!
//! 简化的消息系统定义

use serde::{Deserialize, Serialize};

/// 消息ID
pub type MessageId = String;

/// 消息实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub from: String,
    pub to: MessageTarget,
    pub content: String,
    pub timestamp: i64,
}

impl Message {
    /// 创建私聊消息
    pub fn private(from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.into(),
            to: MessageTarget::Direct(to.into()),
            content: content.into(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// 创建群聊消息
    pub fn group(from: impl Into<String>, group_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.into(),
            to: MessageTarget::Group(group_id.into()),
            content: content.into(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// 创建广播消息
    pub fn broadcast(from: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.into(),
            to: MessageTarget::Broadcast,
            content: content.into(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// 获取目标Agent（如果是私聊）
    pub fn target_agent(&self) -> Option<&str> {
        match &self.to {
            MessageTarget::Direct(agent_id) => Some(agent_id),
            _ => None,
        }
    }

    /// 获取目标群组（如果是群聊）
    pub fn target_group(&self) -> Option<&str> {
        match &self.to {
            MessageTarget::Group(group_id) => Some(group_id),
            _ => None,
        }
    }

    /// 是否是广播消息
    pub fn is_broadcast(&self) -> bool {
        matches!(self.to, MessageTarget::Broadcast)
    }
}

/// 消息目标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageTarget {
    /// 私聊给指定Agent
    Direct(String),
    /// 群聊
    Group(String),
    /// 广播给所有人
    Broadcast,
}

/// 群组定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub creator_id: String,
    pub members: Vec<String>,
    pub created_at: i64,
}

impl Group {
    /// 创建新群组
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        creator_id: impl Into<String>,
        members: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            creator_id: creator_id.into(),
            members,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// 添加成员
    pub fn add_member(&mut self, agent_id: impl Into<String>) {
        let id = agent_id.into();
        if !self.members.contains(&id) {
            self.members.push(id);
        }
    }

    /// 移除成员
    pub fn remove_member(&mut self, agent_id: &str) {
        self.members.retain(|m| m != agent_id);
    }

    /// 检查是否是成员
    pub fn has_member(&self, agent_id: &str) -> bool {
        self.members.contains(&agent_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_message() {
        let msg = Message::private("agent-a", "agent-b", "Hello!");

        assert_eq!(msg.from, "agent-a");
        assert_eq!(msg.target_agent(), Some("agent-b"));
        assert!(!msg.is_broadcast());
    }

    #[test]
    fn test_group_message() {
        let msg = Message::group("agent-a", "group-1", "大家好！");

        assert_eq!(msg.target_group(), Some("group-1"));
        assert!(!msg.is_broadcast());
    }

    #[test]
    fn test_broadcast_message() {
        let msg = Message::broadcast("agent-a", "通知所有人");

        assert!(msg.is_broadcast());
        assert_eq!(msg.target_agent(), None);
        assert_eq!(msg.target_group(), None);
    }

    #[test]
    fn test_group_management() {
        let mut group = Group::new("g1", "测试群", "agent-a", vec!["agent-a".to_string()]);

        group.add_member("agent-b");
        assert!(group.has_member("agent-b"));
        assert_eq!(group.members.len(), 2);

        group.add_member("agent-b"); // 重复添加
        assert_eq!(group.members.len(), 2); // 不重复

        group.remove_member("agent-a");
        assert!(!group.has_member("agent-a"));
    }
}
