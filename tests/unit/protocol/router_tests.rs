//! Router 模块单元测试

use imitatort_stateless_company::protocol::router::{AgentConnector, MessageRouter, RouteTarget};
use imitatort_stateless_company::core::messaging::{Message, MessageBus};
use std::sync::Arc;

#[test]
fn test_message_router_creation() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus, "http://localhost:8080");

    assert!(router.list_local_agents().is_empty());
    assert!(router.list_remote_agents().is_empty());
}

#[test]
fn test_register_local_agent() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus, "http://localhost:8080");

    router.register_local_agent("agent-001");

    assert!(router.is_local("agent-001"));
    assert!(!router.is_local("agent-002"));
    assert_eq!(router.list_local_agents().len(), 1);
}

#[test]
fn test_register_remote_agent() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus, "http://localhost:8080");

    router.register_remote_agent("agent-002", "http://remote:8080");

    assert!(!router.is_local("agent-002"));
    assert!(router.is_known("agent-002"));

    let remotes = router.list_remote_agents();
    assert_eq!(remotes.len(), 1);
    assert_eq!(remotes[0].0, "agent-002");
    assert_eq!(remotes[0].1, "http://remote:8080");
}

#[test]
fn test_unregister_agent() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus, "http://localhost:8080");

    router.register_local_agent("agent-001");
    router.unregister_agent("agent-001");

    assert!(!router.is_local("agent-001"));
    assert!(!router.is_known("agent-001"));
}

#[test]
fn test_get_route_target_local() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus, "http://localhost:8080");

    router.register_local_agent("local-agent");

    match router.get_route_target("local-agent").unwrap() {
        RouteTarget::Local => {}
        _ => panic!("Expected local route"),
    }
}

#[test]
fn test_get_route_target_remote() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus, "http://localhost:8080");

    router.register_remote_agent("remote-agent", "http://remote:8080");

    match router.get_route_target("remote-agent").unwrap() {
        RouteTarget::Remote(endpoint) => assert_eq!(endpoint, "http://remote:8080"),
        _ => panic!("Expected remote route"),
    }
}

#[test]
fn test_get_route_target_unknown() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus, "http://localhost:8080");

    assert!(router.get_route_target("unknown").is_none());
}

#[test]
fn test_is_known() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus, "http://localhost:8080");

    router.register_local_agent("local-agent");
    router.register_remote_agent("remote-agent", "http://remote:8080");

    assert!(router.is_known("local-agent"));
    assert!(router.is_known("remote-agent"));
    assert!(!router.is_known("unknown-agent"));
}

#[tokio::test]
async fn test_route_private_local() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus.clone(), "http://localhost:8080");

    // 注册本地 Agent
    router.register_local_agent("sender");
    router.register_local_agent("receiver");

    // 创建接收者的消息接收器
    let mut receiver = bus.register_agent("receiver");

    // 发送消息
    let msg = Message::private("sender", "receiver", "hello");
    router.route(msg).await.unwrap();

    // 验证接收
    let received = receiver.recv().await.unwrap();
    assert_eq!(received.content, "hello");
    assert_eq!(received.from, "sender");
}

#[tokio::test]
async fn test_route_broadcast_local() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus.clone(), "http://localhost:8080");

    router.register_local_agent("broadcaster");
    router.register_local_agent("listener");

    let mut listener = bus.register_agent("listener");

    let msg = Message::broadcast("broadcaster", "announcement");
    router.route(msg).await.unwrap();

    let received = listener.recv().await.unwrap();
    assert_eq!(received.content, "announcement");
}

#[tokio::test]
async fn test_create_group() {
    let bus = Arc::new(MessageBus::new());
    let router = MessageRouter::new(bus.clone(), "http://localhost:8080");

    router.register_local_agent("creator");
    router.register_local_agent("member1");

    let group_id = router
        .create_group("test-group", "Test Group", "creator", vec!["member1".to_string()])
        .await
        .unwrap();

    assert_eq!(group_id, "test-group");

    // 验证群聊已创建
    let group = bus.get_group("test-group").await;
    assert!(group.is_some());
}

#[tokio::test]
async fn test_invite_to_group() {
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
    let result = router.invite_to_group("test-group", "member1", "invitee").await;
    assert!(result.is_ok());

    // 验证新成员已加入
    let group = bus.get_group("test-group").await.unwrap();
    assert!(group.has_member("invitee"));
}

#[test]
fn test_route_target_clone() {
    let target = RouteTarget::Remote("http://example.com".to_string());
    let cloned = target.clone();
    
    match cloned {
        RouteTarget::Remote(endpoint) => assert_eq!(endpoint, "http://example.com"),
        _ => panic!("Expected remote route"),
    }
}

#[test]
fn test_route_target_debug() {
    let target = RouteTarget::Local;
    let debug_str = format!("{:?}", target);
    assert!(debug_str.contains("Local"));
}

#[test]
fn test_agent_connector_creation() {
    let bus = Arc::new(MessageBus::new());
    let router = Arc::new(MessageRouter::new(bus, "http://localhost:8080"));
    let connector = AgentConnector::new(router, "http://localhost:8080");
    // Connector is created successfully
}
