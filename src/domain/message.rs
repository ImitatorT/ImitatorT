//! Message Domain Entity
//!
//! Simplified message system definition

use serde::{Deserialize, Serialize};

/// Message ID
pub type MessageId = String;

/// Message Entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub from: String,
    pub to: MessageTarget,
    pub content: String,
    pub timestamp: i64,
    /// Referenced message ID (reply functionality)
    pub reply_to: Option<String>,
    /// List of @ users
    pub mentions: Vec<String>,
}

impl Message {
    /// Create private message
    pub fn private(
        from: impl Into<String>,
        to: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
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

    /// Create group message
    pub fn group(
        from: impl Into<String>,
        group_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
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

    /// Set reply message ID
    pub fn with_reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.reply_to = Some(message_id.into());
        self
    }

    /// Add @ user
    pub fn with_mention(mut self, agent_id: impl Into<String>) -> Self {
        let id = agent_id.into();
        if !self.mentions.contains(&id) {
            self.mentions.push(id);
        }
        self
    }

    /// Batch add @ users
    pub fn with_mentions(mut self, agent_ids: Vec<impl Into<String>>) -> Self {
        for id in agent_ids {
            let id = id.into();
            if !self.mentions.contains(&id) {
                self.mentions.push(id);
            }
        }
        self
    }

    /// Get target Agent (if private message)
    pub fn target_agent(&self) -> Option<&str> {
        match &self.to {
            MessageTarget::Direct(agent_id) => Some(agent_id),
            _ => None,
        }
    }

    /// Get target group (if group message)
    pub fn target_group(&self) -> Option<&str> {
        match &self.to {
            MessageTarget::Group(group_id) => Some(group_id),
            _ => None,
        }
    }
}

/// Message Target
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageTarget {
    /// Private message to specified Agent
    Direct(String),
    /// Group chat
    Group(String),
}

/// Group Visibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupVisibility {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "hidden")]
    Hidden,
}

/// Group Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub creator_id: String,
    pub members: Vec<String>,
    pub created_at: i64,
    pub visibility: GroupVisibility,
}

impl Group {
    /// Create new group
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
            visibility: GroupVisibility::Public,
        }
    }

    /// Create new hidden group
    pub fn new_hidden(
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
            visibility: GroupVisibility::Hidden,
        }
    }

    /// Add member
    pub fn add_member(&mut self, agent_id: impl Into<String>) {
        let id = agent_id.into();
        if !self.members.contains(&id) {
            self.members.push(id);
        }
    }

    /// Remove member
    pub fn remove_member(&mut self, agent_id: &str) {
        self.members.retain(|m| m != agent_id);
    }

    /// Check if is member
    pub fn has_member(&self, agent_id: &str) -> bool {
        self.members.contains(&agent_id.to_string())
    }
}
