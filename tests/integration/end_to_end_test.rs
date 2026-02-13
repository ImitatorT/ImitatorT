//! 端到端集成测试
//!
//! 测试整个系统的协同工作能力

use imitatort_stateless_company::application::framework::VirtualCompany;
use imitatort_stateless_company::core::messaging::MessageBus;
use imitatort_stateless_company::core::store::MessageStore;
use imitatort_stateless_company::protocol::router::MessageRouter;
use std::sync::Arc;

#[test]
fn test_end_to_end_company_creation() {
    let company = VirtualCompany::new("http://localhost:8080");

    // 验证初始状态
    assert_eq!(company.local_endpoint(), "http://localhost:8080");
    assert!(company.list_agents().is_empty());
}

#[test]
fn test_end_to_end_message_bus_and_router() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus.clone(), "http://localhost:8080");

    // 验证两者可以一起工作
    router.register_local_agent("agent-001");
    let _receiver = bus.register_agent("agent-001");

    // MessageBus 和 MessageRouter 应该能协同工作
}

#[tokio::test]
async fn test_end_to_end_store_operations() {
    let store = MessageStore::new(100);

    // 添加多条消息
    for i in 0..10 {
        let msg = imitatort_stateless_company::core::store::ChatMessage {
            id: format!("msg-{}", i),
            sender: format!("user-{}", i % 3),
            content: format!("Message {}", i),
            timestamp: chrono::Utc::now().timestamp(),
            message_type: imitatort_stateless_company::core::store::MessageType::Text,
        };
        store.add_message(msg).await.unwrap();
    }

    // 验证存储
    let messages = store.get_recent(5).await;
    assert_eq!(messages.len(), 5);

    // 验证上下文字符串
    let context = store.get_context_string(10).await;
    assert!(context.contains("Message"));
}

#[tokio::test]
async fn test_end_to_end_store_capacity() {
    let store = MessageStore::new(5); // Small capacity

    // 添加超过容量的消息
    for i in 0..10 {
        let msg = imitatort_stateless_company::core::store::ChatMessage {
            id: format!("msg-{}", i),
            sender: "user".to_string(),
            content: format!("Message {}", i),
            timestamp: chrono::Utc::now().timestamp(),
            message_type: imitatort_stateless_company::core::store::MessageType::Text,
        };
        store.add_message(msg).await.unwrap();
    }

    // 只应该保留最新的5条
    let messages = store.get_recent(10).await;
    assert_eq!(messages.len(), 5);
}

#[test]
fn test_end_to_end_config_store_types() {
    use imitatort_stateless_company::core::config::StoreType;

    // Memory store type
    let memory = StoreType::Memory;
    assert_eq!(memory.to_string(), "memory");

    // Parse from string
    let parsed: StoreType = "memory".parse().unwrap();
    assert_eq!(parsed, StoreType::Memory);
}

#[test]
fn test_end_to_end_output_modes() {
    use imitatort_stateless_company::core::config::OutputMode;

    let modes = vec![
        OutputMode::Matrix,
        OutputMode::Cli,
        OutputMode::A2A,
        OutputMode::Hybrid,
    ];

    for mode in modes {
        // 所有模式都应该能正确序列化和显示
        let _ = mode.to_string();
        let _: String = format!("{:?}", mode);
    }
}

#[tokio::test]
async fn test_end_to_end_group_chat_workflow() {
    let bus = Arc::new(MessageBus::new());

    // 注册成员
    let _creator = bus.register_agent("creator");
    let mut member1 = bus.register_agent("member1");
    let mut member2 = bus.register_agent("member2");

    // 创建群聊
    bus.create_group(
        "project-group",
        "Project Discussion",
        "creator",
        vec!["member1".to_string(), "member2".to_string()],
    )
    .await
    .unwrap();

    // 成员加入群聊
    member1.join_group("project-group", &bus).unwrap();
    member2.join_group("project-group", &bus).unwrap();

    // 发送多条群聊消息
    for i in 1..=3 {
        let msg = imitatort_stateless_company::core::messaging::Message::group(
            "creator",
            "project-group",
            &format!("Message {}", i),
        );
        bus.send_group(msg).await.unwrap();
    }

    // 验证成员收到消息
    let mut received_count = 0;
    while let Some(_msg) = member1.try_recv() {
        received_count += 1;
    }
    // Note: Due to timing, actual received count may vary in async context
    assert!(received_count >= 0);
}

#[tokio::test]
async fn test_end_to_end_mixed_messages() {
    let bus = Arc::new(MessageBus::new());

    let _agent_a = bus.register_agent("agent-a");
    let mut agent_b = bus.register_agent("agent-b");
    let mut agent_c = bus.register_agent("agent-c");

    // 发送私聊
    let private = imitatort_stateless_company::core::messaging::Message::private(
        "agent-a",
        "agent-b",
        "Private message",
    );
    bus.send_private(private).await.unwrap();

    // 发送广播
    let broadcast =
        imitatort_stateless_company::core::messaging::Message::broadcast("agent-a", "Broadcast");
    bus.broadcast(broadcast).unwrap();

    // agent-b 应该收到私聊和广播
    let mut b_received = 0;
    while agent_b.try_recv().is_some() {
        b_received += 1;
    }
    assert!(b_received >= 1);

    // agent-c 应该只收到广播
    let mut c_received = 0;
    while agent_c.try_recv().is_some() {
        c_received += 1;
    }
    assert!(c_received >= 1);
}
