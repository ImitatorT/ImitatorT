//! 消息路由集成测试
//!
//! 测试消息在不同 Agent 之间的路由能力

use imitatort_stateless_company::core::messaging::{Message, MessageBus, MessageType};
use imitatort_stateless_company::protocol::router::MessageRouter;
use std::sync::Arc;

#[tokio::test]
async fn test_message_bus_private_message() {
    let bus = Arc::new(MessageBus::new());

    // 注册两个 Agent
    let mut receiver = bus.register_agent("receiver");
    let _sender = bus.register_agent("sender");

    // 发送私聊消息
    let msg = Message::private("sender", "receiver", "Hello!");
    bus.send_private(msg).await.unwrap();

    // 接收消息
    let received = receiver.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap().content, "Hello!");
}

#[tokio::test]
async fn test_message_bus_broadcast() {
    let bus = Arc::new(MessageBus::new());

    // 注册多个 Agent
    let mut listener1 = bus.register_agent("listener1");
    let mut listener2 = bus.register_agent("listener2");
    let _broadcaster = bus.register_agent("broadcaster");

    // 发送广播
    let msg = Message::broadcast("broadcaster", "Announcement!");
    let count = bus.broadcast(msg).unwrap();

    assert!(count >= 2); // At least 2 receivers

    // 两个监听器都应该收到
    let received1 = listener1.recv().await;
    let received2 = listener2.recv().await;

    assert!(received1.is_some());
    assert!(received2.is_some());
    assert_eq!(received1.unwrap().content, "Announcement!");
    assert_eq!(received2.unwrap().content, "Announcement!");
}

#[tokio::test]
async fn test_message_bus_group_chat() {
    let bus = Arc::new(MessageBus::new());

    // 注册成员
    let _creator = bus.register_agent("creator");
    let mut member1 = bus.register_agent("member1");
    let _member2 = bus.register_agent("member2");

    // 创建群聊
    bus.create_group(
        "test-group",
        "Test Group",
        "creator",
        vec!["member1".to_string(), "member2".to_string()],
    )
    .await
    .unwrap();

    // 成员1加入群聊
    member1.join_group("test-group", &bus).unwrap();

    // 发送群聊消息
    let msg = Message::group("creator", "test-group", "Hello group!");
    bus.send_group(msg).await.unwrap();

    // 成员1应该收到
    let received = member1.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap().content, "Hello group!");
}

#[tokio::test]
async fn test_router_local_routing() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus.clone(), "http://localhost:8080");

    // 注册本地 Agent
    router.register_local_agent("sender");
    router.register_local_agent("receiver");

    let mut receiver = bus.register_agent("receiver");

    // 路由私聊消息
    let msg = Message::private("sender", "receiver", "Routed message");
    router.route(msg).await.unwrap();

    // 验证接收
    let received = receiver.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap().content, "Routed message");
}

#[tokio::test]
async fn test_router_local_broadcast() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus.clone(), "http://localhost:8080");

    router.register_local_agent("broadcaster");
    router.register_local_agent("listener");

    let mut listener = bus.register_agent("listener");

    // 路由广播
    let msg = Message::broadcast("broadcaster", "Broadcast via router");
    router.route(msg).await.unwrap();

    let received = listener.recv().await;
    assert!(received.is_some());
}

#[tokio::test]
async fn test_router_create_group() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus.clone(), "http://localhost:8080");

    router.register_local_agent("creator");
    router.register_local_agent("member1");

    let group_id = router
        .create_group("test-group", "Test Group", "creator", vec!["member1".to_string()])
        .await
        .unwrap();

    assert_eq!(group_id, "test-group");

    // 验证群聊存在
    let group = bus.get_group("test-group").await;
    assert!(group.is_some());
}

#[tokio::test]
async fn test_router_invite_to_group() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus.clone(), "http://localhost:8080");

    router.register_local_agent("creator");
    router.register_local_agent("member1");
    router.register_local_agent("invitee");

    // 创建群聊
    router
        .create_group("test-group", "Test Group", "creator", vec!["member1".to_string()])
        .await
        .unwrap();

    // 邀请新成员
    router
        .invite_to_group("test-group", "creator", "invitee")
        .await
        .unwrap();

    // 验证新成员在群聊中
    let group = bus.get_group("test-group").await.unwrap();
    assert!(group.has_member("invitee"));
}

#[tokio::test]
async fn test_message_type_variants() {
    let private = Message::private("a", "b", "test");
    assert_eq!(private.msg_type, MessageType::Private);

    let group = Message::group("a", "group", "test");
    assert_eq!(group.msg_type, MessageType::Group);

    let broadcast = Message::broadcast("a", "test");
    assert_eq!(broadcast.msg_type, MessageType::Broadcast);

    let system = Message::system("test");
    assert_eq!(system.msg_type, MessageType::System);
}

#[tokio::test]
async fn test_message_bus_unregister_agent() {
    let bus = Arc::new(MessageBus::new());

    bus.register_agent("agent");
    bus.unregister_agent("agent");

    // After unregistering, sending to this agent should fail
    let msg = Message::private("sender", "agent", "test");
    let result = bus.send_private(msg).await;
    assert!(result.is_err());
}
