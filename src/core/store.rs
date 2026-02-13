//! 轻量级消息存储模块
//!
//! 提供内存存储和可选的 sled 持久化存储
//! 优先使用内存存储以保持无状态特性，仅在启用 `persistent-store` 特性时使用 sled

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub message_type: MessageType,
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    System,
    ToolCall,
    ToolResult,
}

/// 轻量级消息存储
#[derive(Clone)]
pub struct MessageStore {
    inner: Arc<RwLock<StoreInner>>,
}

struct StoreInner {
    messages: VecDeque<ChatMessage>,
    max_size: usize,
    #[cfg(feature = "persistent-store")]
    db: Option<sled::Db>,
}

impl MessageStore {
    /// 创建新的内存存储
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(StoreInner {
                messages: VecDeque::with_capacity(max_size),
                max_size,
                #[cfg(feature = "persistent-store")]
                db: None,
            })),
        }
    }

    /// 尝试创建持久化存储（仅在启用 `persistent-store` 特性时有效）
    #[cfg(feature = "persistent-store")]
    pub fn new_persistent(path: &str, max_size: usize) -> Result<Self> {
        let db = sled::open(path)?;
        let messages = Self::load_from_db(&db, max_size)?;

        Ok(Self {
            inner: Arc::new(RwLock::new(StoreInner {
                messages,
                max_size,
                db: Some(db),
            })),
        })
    }

    #[cfg(feature = "persistent-store")]
    fn load_from_db(db: &sled::Db, max_size: usize) -> Result<VecDeque<ChatMessage>> {
        let mut messages = VecDeque::with_capacity(max_size);
        for item in db.iter() {
            let (_, value) = item?;
            if let Ok(msg) = serde_json::from_slice::<ChatMessage>(&value) {
                messages.push_back(msg);
            }
            if messages.len() >= max_size {
                break;
            }
        }
        Ok(messages)
    }

    /// 添加消息
    pub async fn add_message(&self, message: ChatMessage) -> Result<()> {
        let mut inner = self.inner.write().await;

        // 如果超过最大大小，移除最旧的消息
        if inner.messages.len() >= inner.max_size {
            inner.messages.pop_front();
        }

        inner.messages.push_back(message.clone());

        // 如果启用持久化，写入数据库
        #[cfg(feature = "persistent-store")]
        if let Some(ref db) = inner.db {
            let key = message.id.as_bytes();
            let value = serde_json::to_vec(&message)?;
            db.insert(key, value)?;
        }

        Ok(())
    }

    /// 获取最近的消息
    pub async fn get_recent(&self, limit: usize) -> Vec<ChatMessage> {
        let inner = self.inner.read().await;
        inner
            .messages
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// 获取所有消息数量
    #[allow(dead_code)]
    pub async fn len(&self) -> usize {
        let inner = self.inner.read().await;
        inner.messages.len()
    }

    /// 是否为空
    #[allow(dead_code)]
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// 清空存储
    pub async fn clear(&self) -> Result<()> {
        let mut inner = self.inner.write().await;
        inner.messages.clear();

        #[cfg(feature = "persistent-store")]
        if let Some(ref db) = inner.db {
            db.clear()?;
        }

        Ok(())
    }

    /// 获取格式化的上下文字符串
    pub async fn get_context_string(&self, limit: usize) -> String {
        let messages = self.get_recent(limit).await;
        messages
            .iter()
            .map(|m| format!("{}: {}", m.sender, m.content))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_message(id: &str, sender: &str, content: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            sender: sender.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            message_type: MessageType::Text,
        }
    }

    #[tokio::test]
    async fn test_store_add_and_get() {
        let store = MessageStore::new(10);

        store
            .add_message(create_test_message("1", "alice", "Hello"))
            .await
            .unwrap();
        store
            .add_message(create_test_message("2", "bob", "Hi"))
            .await
            .unwrap();

        let messages = store.get_recent(10).await;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].sender, "alice");
        assert_eq!(messages[1].sender, "bob");
    }

    #[tokio::test]
    async fn test_store_max_size() {
        let store = MessageStore::new(3);

        store
            .add_message(create_test_message("1", "user1", "msg1"))
            .await
            .unwrap();
        store
            .add_message(create_test_message("2", "user2", "msg2"))
            .await
            .unwrap();
        store
            .add_message(create_test_message("3", "user3", "msg3"))
            .await
            .unwrap();
        store
            .add_message(create_test_message("4", "user4", "msg4"))
            .await
            .unwrap();

        let messages = store.get_recent(10).await;
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].sender, "user2"); // msg1 was removed
    }

    #[tokio::test]
    async fn test_get_context_string() {
        let store = MessageStore::new(10);

        store
            .add_message(create_test_message("1", "alice", "Hello"))
            .await
            .unwrap();
        store
            .add_message(create_test_message("2", "bob", "World"))
            .await
            .unwrap();

        let context = store.get_context_string(10).await;
        assert!(context.contains("alice: Hello"));
        assert!(context.contains("bob: World"));
    }
}
