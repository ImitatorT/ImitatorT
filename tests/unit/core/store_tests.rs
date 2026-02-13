//! Store 模块单元测试

use imitatort_stateless_company::core::store::{ChatMessage, MessageStore, MessageType};

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
    assert_eq!(messages[1].sender, "user3");
    assert_eq!(messages[2].sender, "user4");
}

#[tokio::test]
async fn test_store_get_recent_limit() {
    let store = MessageStore::new(10);

    for i in 1..=5 {
        store
            .add_message(create_test_message(&i.to_string(), &format!("user{}", i), "msg"))
            .await
            .unwrap();
    }

    let messages = store.get_recent(3).await;
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].sender, "user3");
    assert_eq!(messages[1].sender, "user4");
    assert_eq!(messages[2].sender, "user5");
}

#[tokio::test]
async fn test_store_get_context_string() {
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

#[tokio::test]
async fn test_store_clear() {
    let store = MessageStore::new(10);

    store
        .add_message(create_test_message("1", "alice", "Hello"))
        .await
        .unwrap();

    store.clear().await.unwrap();

    let messages = store.get_recent(10).await;
    assert!(messages.is_empty());
}

#[tokio::test]
async fn test_store_len_and_empty() {
    let store = MessageStore::new(10);

    assert_eq!(store.len().await, 0);

    store
        .add_message(create_test_message("1", "alice", "Hello"))
        .await
        .unwrap();

    assert_eq!(store.len().await, 1);
}

#[tokio::test]
async fn test_store_different_message_types() {
    let store = MessageStore::new(10);

    let text_msg = ChatMessage {
        id: "1".to_string(),
        sender: "alice".to_string(),
        content: "Hello".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        message_type: MessageType::Text,
    };

    let system_msg = ChatMessage {
        id: "2".to_string(),
        sender: "system".to_string(),
        content: "System message".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        message_type: MessageType::System,
    };

    store.add_message(text_msg).await.unwrap();
    store.add_message(system_msg).await.unwrap();

    let messages = store.get_recent(10).await;
    assert_eq!(messages.len(), 2);
}

#[tokio::test]
async fn test_store_empty_context_string() {
    let store = MessageStore::new(10);
    let context = store.get_context_string(10).await;
    assert_eq!(context, "");
}

#[tokio::test]
async fn test_store_clone() {
    let store = MessageStore::new(10);

    store
        .add_message(create_test_message("1", "alice", "Hello"))
        .await
        .unwrap();

    let cloned = store.clone();
    let messages = cloned.get_recent(10).await;
    assert_eq!(messages.len(), 1);
}

#[test]
fn test_message_type_debug() {
    let msg_type = MessageType::Text;
    let debug_str = format!("{:?}", msg_type);
    assert!(debug_str.contains("Text"));
}

#[test]
fn test_chat_message_clone() {
    let msg = create_test_message("1", "alice", "Hello");
    let cloned = msg.clone();
    assert_eq!(msg.id, cloned.id);
    assert_eq!(msg.content, cloned.content);
}
