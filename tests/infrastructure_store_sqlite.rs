//! SQLite 存储实现测试

use imitatort_stateless_company::core::store::{MessageFilter, Store};
use imitatort_stateless_company::domain::{Agent, Department, Group, LLMConfig, Message, Organization, Role};
use imitatort_stateless_company::infrastructure::store::SqliteStore;

fn create_test_organization() -> Organization {
    let mut org = Organization::new();

    org.add_department(Department::top_level("tech", "技术部"));
    org.add_department(Department::child("fe", "前端组", "tech"));

    let agent = Agent::new(
        "ceo",
        "CEO",
        Role::simple("CEO", "你是公司的CEO")
            .with_responsibilities(vec!["决策".to_string(), "管理".to_string()]),
        LLMConfig::openai("test-key"),
    )
    .with_department("tech");

    org.add_agent(agent);
    org
}

#[tokio::test]
async fn test_sqlite_store_organization() {
    let store = SqliteStore::new_in_memory().unwrap();
    let org = create_test_organization();

    store.save_organization(&org).await.unwrap();
    let loaded = store.load_organization().await.unwrap();

    assert_eq!(loaded.agents.len(), 1);
    assert_eq!(loaded.departments.len(), 2);

    // 验证 Agent 数据完整
    let agent = loaded.find_agent("ceo").unwrap();
    assert_eq!(agent.name, "CEO");
    assert_eq!(agent.llm_config.model, "gpt-4o-mini");
    assert_eq!(agent.role.responsibilities.len(), 2);
}

#[tokio::test]
async fn test_sqlite_store_messages() {
    let store = SqliteStore::new_in_memory().unwrap();

    let msg1 = Message::private("a1", "a2", "Hello!");
    let msg2 = Message::group("a1", "g1", "大家好！");

    store.save_message(&msg1).await.unwrap();
    store.save_message(&msg2).await.unwrap();

    // 测试查询全部
    let messages = store.load_messages(MessageFilter::new().limit(10)).await.unwrap();
    assert_eq!(messages.len(), 2);

    // 测试按发送者查询
    let filter = MessageFilter::new().from("a1");
    let messages = store.load_messages(filter).await.unwrap();
    assert_eq!(messages.len(), 2);

    // 测试按目标类型查询
    let filter = MessageFilter::new().target_type("group");
    let messages = store.load_messages(filter).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "大家好！");

    // 测试按接收者查询
    let filter = MessageFilter::new().to("a2").target_type("direct");
    let messages = store.load_messages(filter).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Hello!");
}

#[tokio::test]
async fn test_sqlite_store_groups() {
    let store = SqliteStore::new_in_memory().unwrap();

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
    assert_eq!(groups[0].members.len(), 2);

    store.delete_group("g1").await.unwrap();
    let groups = store.load_groups().await.unwrap();
    assert_eq!(groups.len(), 0);
}

#[tokio::test]
async fn test_sqlite_store_batch_messages() {
    let store = SqliteStore::new_in_memory().unwrap();

    let messages: Vec<Message> = (0..100)
        .map(|i| Message::private("a1", "a2", format!("msg{}", i)))
        .collect();

    store.save_messages(&messages).await.unwrap();

    let loaded = store.load_messages(MessageFilter::new().limit(50)).await.unwrap();
    assert_eq!(loaded.len(), 50);
}

#[tokio::test]
async fn test_sqlite_store_load_messages_by_agent() {
    let store = SqliteStore::new_in_memory().unwrap();

    // 创建一些消息
    let messages = vec![
        Message::private("a1", "a2", "a1->a2"),
        Message::private("a2", "a1", "a2->a1"),
        Message::private("a1", "a3", "a1->a3"),
    ];

    for msg in &messages {
        store.save_message(msg).await.unwrap();
    }

    // 查询与 a1 相关的消息
    let a1_messages = store.load_messages_by_agent("a1", 10).await.unwrap();
    assert_eq!(a1_messages.len(), 3); // 发送2条 + 接收1条
}
