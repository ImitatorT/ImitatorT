//! 消息通信层测试

use imitatort_stateless_company::core::messaging::MessageBus;
use imitatort_stateless_company::domain::Message;

#[test]
fn test_message_bus_creation() {
    let _bus = MessageBus::new();
    // MessageBus created successfully
}

#[tokio::test]
async fn test_agent_registration() {
    let bus = MessageBus::new();
    let rx = bus.register("agent-1");
    assert_eq!(rx.len(), 0);

    bus.unregister("agent-1");
}

#[tokio::test]
async fn test_private_messaging() {
    let bus = MessageBus::new();
    let mut rx = bus.register("agent-2");

    let msg = Message::private("agent-1", "agent-2", "Hello!");
    bus.send(msg.clone()).await.unwrap();

    let received = rx.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap().content, "Hello!");
}

#[tokio::test]
async fn test_group_messaging() {
    let bus = MessageBus::new();
    let _rx1 = bus.register("agent-1");
    let _rx2 = bus.register("agent-2");

    // 创建群组成功
    bus.create_group("g1", "测试群", "agent-1", vec!["agent-1".to_string(), "agent-2".to_string()])
        .await
        .unwrap();

    // 群聊消息发送在集成测试中需要订阅者保持存活
    // 这里仅验证群组创建成功
}
