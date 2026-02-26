//! Agent 状态管理模块
//!
//! 提供 Agent 状态持久化和记忆管理功能

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use crate::domain::Message;

/// Agent 状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Agent ID
    pub id: String,
    /// 最后活跃时间戳
    pub last_active: i64,
    /// 对话历史
    pub conversation_history: Vec<Message>,
    /// 关键记忆
    pub memories: Vec<Memory>,
    /// 长期记忆摘要
    pub long_term_summary: String,
    /// Agent 特定状态
    pub custom_state: HashMap<String, serde_json::Value>,
}

impl AgentState {
    pub fn new(id: String) -> Self {
        Self {
            id,
            last_active: Utc::now().timestamp(),
            conversation_history: Vec::new(),
            memories: Vec::new(),
            long_term_summary: String::new(),
            custom_state: HashMap::new(),
        }
    }

    /// 添加记忆
    pub fn add_memory(&mut self, memory: Memory) {
        self.memories.push(memory);
        // 限制记忆数量，避免无限增长
        if self.memories.len() > 100 {
            self.memories.drain(0..10); // 移除最早的10条记忆
        }
    }

    /// 添加对话历史
    pub fn add_conversation(&mut self, message: Message) {
        self.conversation_history.push(message);
        // 限制对话历史长度
        if self.conversation_history.len() > 50 {
            self.conversation_history.drain(0..5); // 移除最早的5条消息
        }
    }

    /// 更新最后活跃时间
    pub fn update_last_active(&mut self) {
        self.last_active = Utc::now().timestamp();
    }
}

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// 记忆ID
    pub id: String,
    /// 记忆内容
    pub content: String,
    /// 重要性评分 (1-10)
    pub importance: u8,
    /// 创建时间戳
    pub timestamp: i64,
    /// 关联的上下文
    pub context: String,
    /// 记忆类型
    pub memory_type: MemoryType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    /// 事实记忆
    Fact,
    /// 事件记忆
    Event,
    /// 关系记忆
    Relationship,
    /// 任务记忆
    Task,
    /// 其他类型
    Other,
}

impl Memory {
    pub fn new(content: String, importance: u8, context: String, memory_type: MemoryType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            importance,
            timestamp: Utc::now().timestamp(),
            context,
            memory_type,
        }
    }
}

/// 状态管理器
pub struct StateManager {
    /// 存储所有Agent的状态
    states: Arc<RwLock<HashMap<String, AgentState>>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取或创建Agent状态
    pub async fn get_or_create_state(&self, agent_id: &str) -> AgentState {
        let mut states = self.states.write().await;
        if let Some(state) = states.get(agent_id) {
            state.clone()
        } else {
            let new_state = AgentState::new(agent_id.to_string());
            states.insert(agent_id.to_string(), new_state.clone());
            new_state
        }
    }

    /// 更新Agent状态
    pub async fn update_state(&self, state: AgentState) -> Result<()> {
        let mut states = self.states.write().await;
        states.insert(state.id.clone(), state);
        Ok(())
    }

    /// 添加记忆
    pub async fn add_memory(&self, agent_id: &str, memory: Memory) -> Result<()> {
        let mut state = self.get_or_create_state(agent_id).await;
        state.add_memory(memory);
        state.update_last_active();
        self.update_state(state).await
    }

    /// 添加对话历史
    pub async fn add_conversation(&self, agent_id: &str, message: Message) -> Result<()> {
        let mut state = self.get_or_create_state(agent_id).await;
        state.add_conversation(message);
        state.update_last_active();
        self.update_state(state).await
    }

    /// 获取Agent的记忆
    pub async fn get_memories(&self, agent_id: &str, limit: Option<usize>) -> Vec<Memory> {
        let state = self.get_or_create_state(agent_id).await;
        let mut memories = state.memories.clone();

        // 按重要性和时间排序
        memories.sort_by(|a, b| {
            b.importance.cmp(&a.importance)
                .then(b.timestamp.cmp(&a.timestamp))
        });

        if let Some(limit) = limit {
            memories.truncate(limit);
        }

        memories
    }

    /// 获取Agent的对话历史
    pub async fn get_conversation_history(&self, agent_id: &str, limit: Option<usize>) -> Vec<Message> {
        let state = self.get_or_create_state(agent_id).await;
        let mut history = state.conversation_history.clone();

        if let Some(limit) = limit {
            history.truncate(limit);
        }

        history
    }

    /// 获取所有Agent ID
    pub async fn get_all_agent_ids(&self) -> Vec<String> {
        let states = self.states.read().await;
        states.keys().cloned().collect()
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Message, MessageTarget};

    #[tokio::test]
    async fn test_state_manager() {
        let manager = StateManager::new();

        // 测试创建状态
        let state = manager.get_or_create_state("test_agent").await;
        assert_eq!(state.id, "test_agent");

        // 测试添加记忆
        let memory = Memory::new(
            "This is a test memory".to_string(),
            8,
            "test context".to_string(),
            MemoryType::Fact,
        );
        manager.add_memory("test_agent", memory).await.unwrap();

        // 测试获取记忆
        let memories = manager.get_memories("test_agent", Some(10)).await;
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].content, "This is a test memory");

        // 测试添加对话历史
        let message = Message {
            id: "msg1".to_string(),
            from: "test_agent".to_string(),
            to: MessageTarget::Direct("other_agent".to_string()),
            content: "Hello, world!".to_string(),
            timestamp: Utc::now().timestamp(),
            reply_to: None,
            mentions: vec![],
        };
        manager.add_conversation("test_agent", message).await.unwrap();

        // 测试获取对话历史
        let history = manager.get_conversation_history("test_agent", Some(10)).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "Hello, world!");
    }
}