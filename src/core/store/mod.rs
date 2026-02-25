//! 存储接口定义
//!
//! 提供持久化能力的抽象接口，支持内存和SQLite实现

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::{Group, Message, Organization};

/// 消息查询过滤器
#[derive(Debug, Clone, Default)]
pub struct MessageFilter {
    /// 发送者ID
    pub from: Option<String>,
    /// 接收者ID（私聊）或群ID（群聊）
    pub to: Option<String>,
    /// 目标类型: "direct", "group", "broadcast"
    pub target_type: Option<String>,
    /// 起始时间戳（包含）
    pub since: Option<i64>,
    /// 最大返回数量
    pub limit: usize,
}

impl MessageFilter {
    /// 创建新的过滤器
    pub fn new() -> Self {
        Self {
            limit: 100,
            ..Default::default()
        }
    }

    /// 设置发送者
    pub fn from(mut self, agent_id: impl Into<String>) -> Self {
        self.from = Some(agent_id.into());
        self
    }

    /// 设置接收者
    pub fn to(mut self, target_id: impl Into<String>) -> Self {
        self.to = Some(target_id.into());
        self
    }

    /// 设置目标类型
    pub fn target_type(mut self, type_: impl Into<String>) -> Self {
        self.target_type = Some(type_.into());
        self
    }

    /// 设置起始时间
    pub fn since(mut self, timestamp: i64) -> Self {
        self.since = Some(timestamp);
        self
    }

    /// 设置返回数量限制
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = n;
        self
    }
}

/// 存储接口
///
/// 提供组织架构、群聊、消息的持久化能力
#[async_trait]
pub trait Store: Send + Sync {
    /// 保存组织架构（完全覆盖）
    async fn save_organization(&self, org: &Organization) -> Result<()>;

    /// 加载组织架构
    ///
    /// 如果存储中没有组织架构，返回空Organization
    async fn load_organization(&self) -> Result<Organization>;

    /// 保存群聊
    async fn save_group(&self, group: &Group) -> Result<()>;

    /// 加载所有群聊
    async fn load_groups(&self) -> Result<Vec<Group>>;

    /// 删除群聊
    async fn delete_group(&self, group_id: &str) -> Result<()>;

    /// 保存消息
    async fn save_message(&self, message: &Message) -> Result<()>;

    /// 批量保存消息
    async fn save_messages(&self, messages: &[Message]) -> Result<()> {
        for msg in messages {
            self.save_message(msg).await?;
        }
        Ok(())
    }

    /// 根据过滤器查询消息
    async fn load_messages(&self, filter: MessageFilter) -> Result<Vec<Message>>;

    /// 加载与指定Agent相关的消息
    async fn load_messages_by_agent(&self, agent_id: &str, limit: usize) -> Result<Vec<Message>> {
        let filter = MessageFilter::new()
            .limit(limit)
            .from(agent_id);
        let from_msgs = self.load_messages(filter).await?;

        let filter = MessageFilter::new()
            .limit(limit)
            .to(agent_id);
        let to_msgs = self.load_messages(filter).await?;

        // 合并并去重
        let mut all_msgs = from_msgs;
        for msg in to_msgs {
            if !all_msgs.iter().any(|m| m.id == msg.id) {
                all_msgs.push(msg);
            }
        }

        // 按时间戳排序，最新的在前
        all_msgs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_msgs.truncate(limit);

        Ok(all_msgs)
    }

    /// 加载指定群聊的消息
    async fn load_messages_by_group(&self, group_id: &str, limit: usize) -> Result<Vec<Message>> {
        let filter = MessageFilter::new()
            .to(group_id)
            .target_type("group")
            .limit(limit);
        self.load_messages(filter).await
    }
}

mod memory;
pub use memory::MemoryStore;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Agent, Department, LLMConfig, Message, Organization, Role};

    fn create_test_organization() -> Organization {
        let mut org = Organization::new();

        org.add_department(Department::top_level("tech", "技术部"));

        let agent = Agent::new(
            "ceo",
            "CEO",
            Role::simple("CEO", "你是公司的CEO"),
            LLMConfig::openai("test-key"),
        )
        .with_department("tech");

        org.add_agent(agent);
        org
    }

    #[tokio::test]
    async fn test_message_filter_builder() {
        let filter = MessageFilter::new()
            .from("agent1")
            .to("agent2")
            .target_type("direct")
            .since(1000)
            .limit(50);

        assert_eq!(filter.from, Some("agent1".to_string()));
        assert_eq!(filter.to, Some("agent2".to_string()));
        assert_eq!(filter.target_type, Some("direct".to_string()));
        assert_eq!(filter.since, Some(1000));
        assert_eq!(filter.limit, 50);
    }

    #[tokio::test]
    async fn test_memory_store_basic() {
        let store = MemoryStore::new();

        // 测试组织架构
        let org = create_test_organization();
        store.save_organization(&org).await.unwrap();

        let loaded = store.load_organization().await.unwrap();
        assert_eq!(loaded.agents.len(), 1);
        assert_eq!(loaded.departments.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_store_messages() {
        let store = MemoryStore::new();

        // 保存消息
        let msg1 = Message::private("a1", "a2", "Hello!");
        let msg2 = Message::private("a2", "a1", "Hi!");

        store.save_message(&msg1).await.unwrap();
        store.save_message(&msg2).await.unwrap();

        // 查询消息
        let messages = store
            .load_messages(MessageFilter::new().limit(10))
            .await
            .unwrap();
        assert_eq!(messages.len(), 2);

        // 按发送者查询
        let messages = store.load_messages_by_agent("a1", 10).await.unwrap();
        assert_eq!(messages.len(), 2); // 包含发送和接收

        // 按接收者查询
        let filter = MessageFilter::new().to("a2").target_type("direct");
        let messages = store.load_messages(filter).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello!");
    }

    #[tokio::test]
    async fn test_memory_store_groups() {
        let store = MemoryStore::new();

        let group = Group::new(
            "g1",
            "测试群",
            "agent1",
            vec!["agent1".to_string(), "agent2".to_string()],
        );

        store.save_group(&group).await.unwrap();

        let groups = store.load_groups().await.unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "测试群");

        store.delete_group("g1").await.unwrap();
        let groups = store.load_groups().await.unwrap();
        assert_eq!(groups.len(), 0);
    }
}
