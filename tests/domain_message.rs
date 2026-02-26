//! Message 领域实体测试

use imitatort_stateless_company::{Group, Message};

#[test]
fn test_private_message() {
    let msg = Message::private("agent-a", "agent-b", "Hello!");

    assert_eq!(msg.from, "agent-a");
    assert_eq!(msg.target_agent(), Some("agent-b"));
}

#[test]
fn test_group_message() {
    let msg = Message::group("agent-a", "group-1", "大家好！");

    assert_eq!(msg.target_group(), Some("group-1"));
}

#[test]
fn test_group_management() {
    let mut group = Group::new("g1", "测试群", "agent-a", vec!["agent-a".to_string()]);

    group.add_member("agent-b");
    assert!(group.has_member("agent-b"));
    assert_eq!(group.members.len(), 2);

    group.add_member("agent-b"); // 重复添加
    assert_eq!(group.members.len(), 2); // 不重复

    group.remove_member("agent-a");
    assert!(!group.has_member("agent-a"));
}
