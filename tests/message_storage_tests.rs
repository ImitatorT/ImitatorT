use imitatort::{
    core::store::{MessageFilter, Store},
    domain::{Message, MessageTarget},
    infrastructure::store::SqliteStore,
};
use std::sync::Arc;

#[tokio::test]
async fn test_message_storage() {
    let store = Arc::new(SqliteStore::new_in_memory().unwrap());

    // 创建一条消息
    let message = Message {
        id: "test_msg_1".to_string(),
        from: "agent1".to_string(),
        to: MessageTarget::Direct("agent2".to_string()),
        content: "Hello, world!".to_string(),
        timestamp: 1234567890,
        reply_to: None,
        mentions: vec![],
    };

    // 保存消息
    store.save_message(&message).await.unwrap();

    // 查询消息
    let filter = MessageFilter::new().limit(10);
    let messages: Vec<Message> = store.load_messages(filter).await.unwrap();

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, "test_msg_1");
    assert_eq!(messages[0].content, "Hello, world!");
}

#[tokio::test]
async fn test_message_storage_with_filters() {
    let store = Arc::new(SqliteStore::new_in_memory().unwrap());

    // 创建多条消息
    let msg1 = Message {
        id: "test_msg_1".to_string(),
        from: "agent1".to_string(),
        to: MessageTarget::Direct("agent2".to_string()),
        content: "First message".to_string(),
        timestamp: 1234567890,
        reply_to: None,
        mentions: vec![],
    };

    let msg2 = Message {
        id: "test_msg_2".to_string(),
        from: "agent2".to_string(),
        to: MessageTarget::Direct("agent1".to_string()),
        content: "Second message".to_string(),
        timestamp: 1234567891,
        reply_to: None,
        mentions: vec![],
    };

    // 保存消息
    store.save_message(&msg1).await.unwrap();
    store.save_message(&msg2).await.unwrap();

    // 测试按发送者过滤
    let filter = MessageFilter::new().from("agent1").limit(10);
    let messages: Vec<Message> = store.load_messages(filter).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, "test_msg_1");

    // 测试按接收者过滤
    let filter = MessageFilter::new().to("agent1").limit(10);
    let messages: Vec<Message> = store.load_messages(filter).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, "test_msg_2");

    // 测试按时间范围过滤
    let filter = MessageFilter::new().since(1234567890).limit(10);
    let messages: Vec<Message> = store.load_messages(filter).await.unwrap();
    assert_eq!(messages.len(), 2);
}
