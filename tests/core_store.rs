//! 存储接口定义测试

use imitatort::core::store::{MessageFilter, Store};
use imitatort::domain::{Agent, Department, LLMConfig, Message, Organization, Role};
use imitatort::infrastructure::store::SqliteStore;

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
    let store = SqliteStore::new_in_memory().unwrap();

    // 测试组织架构
    let org = create_test_organization();
    store.save_organization(&org).await.unwrap();

    let loaded = store.load_organization().await.unwrap();
    assert_eq!(loaded.agents.len(), 1);
    assert_eq!(loaded.departments.len(), 1);
}

#[tokio::test]
async fn test_memory_store_messages() {
    let store = SqliteStore::new_in_memory().unwrap();

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
    let store = SqliteStore::new_in_memory().unwrap();

    let group = imitatort::domain::Group::new(
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
