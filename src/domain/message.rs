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
    /// 引用的消息ID（回复功能）
    pub reply_to: Option<String>,
    /// @的用户列表
    pub mentions: Vec<String>,
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
            reply_to: None,
            mentions: Vec::new(),
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
            reply_to: None,
            mentions: Vec::new(),
        }
    }

    /// 设置回复的消息ID
    pub fn with_reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.reply_to = Some(message_id.into());
        self
    }

    /// 添加@用户
    pub fn with_mention(mut self, agent_id: impl Into<String>) -> Self {
        let id = agent_id.into();
        if !self.mentions.contains(&id) {
            self.mentions.push(id);
        }
        self
    }

    /// 批量添加@用户
    pub fn with_mentions(mut self, agent_ids: Vec<impl Into<String>>) -> Self {
        for id in agent_ids {
            let id = id.into();
            if !self.mentions.contains(&id) {
                self.mentions.push(id);
            }
        }
        self
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
}

/// 消息目标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageTarget {
    /// 私聊给指定Agent
    Direct(String),
    /// 群聊
    Group(String),
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
