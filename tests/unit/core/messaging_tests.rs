//! Messaging 模块单元测试

use imitatort_stateless_company::core::messaging::{
    GroupInfo, Message, MessageBus, MessageType,
};

#[test]
fn test_private_message_creation() {
    let msg = Message::private("alice", "bob", "hello");
    assert_eq!(msg.msg_type, MessageType::Private);
    assert_eq!(msg.from, "alice");
    assert_eq!(msg.to, vec!["bob"]);
    assert_eq!(msg.content, "hello");
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    assert!(msg.metadata.is_none());
}

#[test]
fn test_group_message_creation() {
    let msg = Message::group("alice", "group-1", "hello group");
    assert_eq!(msg.msg_type, MessageType::Group);
    assert_eq!(msg.from, "alice");
    assert_eq!(msg.to, vec!["group-1"]);
    assert_eq!(msg.content, "hello group");
}

#[test]
fn test_broadcast_message_creation() {
    let msg = Message::broadcast("system", "announcement");
    assert_eq!(msg.msg_type, MessageType::Broadcast);
    assert_eq!(msg.from, "system");
    assert!(msg.to.is_empty());
}

#[test]
fn test_system_message_creation() {
    let msg = Message::system("system notification");
    assert_eq!(msg.msg_type, MessageType::System);
    assert_eq!(msg.from, "system");
    assert!(msg.to.is_empty());
    assert_eq!(msg.content, "system notification");
}

#[test]
fn test_message_type_equality() {
    assert_eq!(MessageType::Private, MessageType::Private);
    assert_ne!(MessageType::Private, MessageType::Group);
    assert_ne!(MessageType::Broadcast, MessageType::System);
}

#[test]
fn test_group_info_creation() {
    let members = vec!["alice".to_string(), "bob".to_string()];
    let group = GroupInfo::new("group-1", "Test Group", "alice", members);

    assert_eq!(group.id, "group-1");
    assert_eq!(group.name, "Test Group");
    assert_eq!(group.creator, "alice");
    assert!(group.has_member("alice"));
    assert!(group.has_member("bob"));
    assert!(!group.has_member("charlie"));
    assert!(group.created_at > 0);
}

#[test]
fn test_group_info_add_member() {
    let members = vec!["alice".to_string()];
    let mut group = GroupInfo::new("group-1", "Test", "alice", members);

    group.add_member("bob");
    assert!(group.has_member("bob"));

    // 重复添加应该被忽略
    group.add_member("bob");
    assert_eq!(group.members.len(), 2);
}

#[test]
fn test_group_info_remove_member() {
    let members = vec!["alice".to_string(), "bob".to_string()];
    let mut group = GroupInfo::new("group-1", "Test", "alice", members);

    group.remove_member("alice");
    assert!(!group.has_member("alice"));
    assert!(group.has_member("bob"));

    // 移除不存在的成员不应该报错
    group.remove_member("charlie");
}

#[test]
fn test_message_bus_creation() {
    let bus = MessageBus::new();
    // 新创建的 bus 应该为空
    assert!(bus.list_agent_groups("any").is_empty());
}

#[test]
fn test_message_clone() {
    let msg = Message::private("alice", "bob", "hello");
    let cloned = msg.clone();
    assert_eq!(msg.id, cloned.id);
    assert_eq!(msg.content, cloned.content);
}

#[test]
fn test_group_info_clone() {
    let members = vec!["alice".to_string()];
    let group = GroupInfo::new("group-1", "Test", "alice", members);
    let cloned = group.clone();
    assert_eq!(group.id, cloned.id);
    assert_eq!(group.members, cloned.members);
}

#[test]
fn test_message_type_debug() {
    let msg_type = MessageType::Private;
    let debug_str = format!("{:?}", msg_type);
    assert!(debug_str.contains("Private"));
}

#[tokio::test]
async fn test_message_bus_register_agent() {
    let bus = MessageBus::new();
    let receiver = bus.register_agent("test-agent");
    
    assert_eq!(receiver.agent_id(), "test-agent");
}

#[tokio::test]
async fn test_message_bus_create_group() {
    let bus = MessageBus::new();
    
    // 注册创建者
    let _receiver = bus.register_agent("creator");
    let _member_rx = bus.register_agent("member1");
    
    let result = bus.create_group("test-group", "Test Group", "creator", vec!["member1".to_string()]).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test-group");
}

#[tokio::test]
async fn test_message_bus_create_group_unregistered_creator() {
    let bus = MessageBus::new();
    
    // 不注册创建者，直接创建群聊
    let result = bus.create_group("test-group", "Test Group", "creator", vec![]).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_message_bus_invite_to_group() {
    let bus = MessageBus::new();
    
    let _creator_rx = bus.register_agent("creator");
    let _member_rx = bus.register_agent("member1");
    let _invitee_rx = bus.register_agent("invitee");
    
    // 创建群聊
    bus.create_group("test-group", "Test Group", "creator", vec!["member1".to_string()]).await.unwrap();
    
    // 邀请成员
    let result = bus.invite_to_group("test-group", "creator", "invitee").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_message_bus_invite_unregistered() {
    let bus = MessageBus::new();
    
    let _creator_rx = bus.register_agent("creator");
    
    bus.create_group("test-group", "Test Group", "creator", vec![]).await.unwrap();
    
    // 邀请未注册的成员
    let result = bus.invite_to_group("test-group", "creator", "unregistered").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_message_bus_get_group() {
    let bus = MessageBus::new();
    
    let _creator_rx = bus.register_agent("creator");
    bus.create_group("test-group", "Test Group", "creator", vec![]).await.unwrap();
    
    let group = bus.get_group("test-group").await;
    assert!(group.is_some());
    assert_eq!(group.unwrap().name, "Test Group");
    
    // 获取不存在的群聊
    let not_found = bus.get_group("non-existent").await;
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_message_bus_leave_group() {
    let bus = MessageBus::new();
    
    let _creator_rx = bus.register_agent("creator");
    let _member_rx = bus.register_agent("member1");
    
    bus.create_group("test-group", "Test Group", "creator", vec!["member1".to_string()]).await.unwrap();
    
    let result = bus.leave_group("test-group", "member1").await;
    assert!(result.is_ok());
    
    // 验证成员已离开
    let group = bus.get_group("test-group").await.unwrap();
    assert!(!group.has_member("member1"));
}

#[tokio::test]
async fn test_message_bus_delete_group() {
    let bus = MessageBus::new();
    
    let _creator_rx = bus.register_agent("creator");
    bus.create_group("test-group", "Test Group", "creator", vec![]).await.unwrap();
    
    // 只有创建者可以删除
    let result = bus.delete_group("test-group", "creator").await;
    assert!(result.is_ok());
    
    // 验证群聊已删除
    let group = bus.get_group("test-group").await;
    assert!(group.is_none());
}

#[tokio::test]
async fn test_message_bus_delete_group_not_creator() {
    let bus = MessageBus::new();
    
    let _creator_rx = bus.register_agent("creator");
    let _other_rx = bus.register_agent("other");
    bus.create_group("test-group", "Test Group", "creator", vec!["other".to_string()]).await.unwrap();
    
    // 非创建者不能删除
    let result = bus.delete_group("test-group", "other").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_message_bus_list_agent_groups() {
    let bus = MessageBus::new();
    
    let _creator_rx = bus.register_agent("creator");
    let _member_rx = bus.register_agent("member1");
    
    bus.create_group("group-1", "Group 1", "creator", vec!["member1".to_string()]).await.unwrap();
    bus.create_group("group-2", "Group 2", "creator", vec!["member1".to_string()]).await.unwrap();
    
    let groups = bus.list_agent_groups("member1").await;
    assert_eq!(groups.len(), 2);
}
