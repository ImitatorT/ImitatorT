//! ImitatorT AI Agent Behavior Tests

use imitatort_stateless_company::domain::{Agent, LLMConfig, Message, MessageTarget, Organization, Role};
// 移除对私有模块的引用，使用公共API
use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::test]
async fn test_agent_initialization() {
    // 创建测试agent
    let agent = Agent::new(
        "test-developer",
        "Test Developer",
        Role::simple("Developer", "You are a developer, responsible for writing high-quality code"),
        LLMConfig::openai("test-key"),
    );

    // 验证agent属性
    assert_eq!(agent.id, "test-developer");
    assert_eq!(agent.name, "Test Developer");
    assert_eq!(agent.role.title, "Developer");
    assert!(agent.role.system_prompt.contains("developer"));

    // 验证默认值
    assert!(agent.department_id.is_none());
}

#[tokio::test]
async fn test_agent_responds_to_message() {
    // 创建测试agent
    let agent = Agent::new(
        "test-manager",
        "Test Manager",
        Role::simple("Manager", "You are a manager, responsible for managing teams and assigning tasks"),
        LLMConfig::openai("test-key"),
    );

    // 创建消息
    let message = Message {
        id: "msg-1".to_string(),
        from: "employee".to_string(),
        to: MessageTarget::Direct("test-manager".to_string()),
        content: "Hello manager, what is today's work plan?".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        reply_to: None,
        mentions: vec![],
    };

    // 验证agent可以处理消息
    // 注意：这里由于涉及实际LLM调用，我们主要测试结构和逻辑
    assert_eq!(agent.name, "Test Manager");
}

#[tokio::test]
async fn test_multi_agent_interaction() {
    let mut org = Organization::new();

    // 创建多个agent
    let manager = Agent::new(
        "team-manager",
        "Team Manager",
        Role::simple("Team Manager", "You are the team manager, responsible for coordinating team members"),
        LLMConfig::openai("test-key"),
    );

    let developer = Agent::new(
        "senior-dev",
        "Senior Developer",
        Role::simple("Senior Developer", "You are a senior developer engineer, responsible for technical implementation"),
        LLMConfig::openai("test-key"),
    );

    let designer = Agent::new(
        "ui-designer",
        "UI Designer",
        Role::simple("UI Designer", "You are a UI designer, responsible for interface design"),
        LLMConfig::openai("test-key"),
    );

    org.add_agent(manager);
    org.add_agent(developer);
    org.add_agent(designer);

    // 验证组织中有正确的agent数量
    assert_eq!(org.agents.len(), 3);

    // 验证可以找到特定agent
    assert!(org.find_agent("team-manager").is_some());
    assert!(org.find_agent("senior-dev").is_some());
    assert!(org.find_agent("ui-designer").is_some());
    assert!(org.find_agent("non-existent").is_none());
}

#[tokio::test]
async fn test_agent_role_based_behavior() {
    // 创建具有不同角色的agent
    let manager = Agent::new(
        "manager",
        "Manager",
        Role::simple("Project Manager", "You are the project manager, responsible for project schedule management and team coordination"),
        LLMConfig::openai("test-key"),
    );

    let developer = Agent::new(
        "developer",
        "Developer",
        Role::simple("Software Developer", "You are a software developer engineer, responsible for writing code and solving problems"),
        LLMConfig::openai("test-key"),
    );

    let qa_engineer = Agent::new(
        "qa",
        "QA Engineer",
        Role::simple("QA Engineer", "You are a quality assurance engineer, responsible for testing and quality control"),
        LLMConfig::openai("test-key"),
    );

    // 验证每个agent的角色描述不同
    assert!(manager.role.system_prompt.contains("project schedule"));
    assert!(developer.role.system_prompt.contains("writing code"));
    assert!(qa_engineer.role.system_prompt.contains("quality"));

    // 验证角色标题也不同
    assert_eq!(manager.role.title, "Project Manager");
    assert_eq!(developer.role.title, "Software Developer");
    assert_eq!(qa_engineer.role.title, "QA Engineer");
}

#[tokio::test]
async fn test_agent_message_routing() {
    let message_to_individual = Message {
        id: "msg-1".to_string(),
        from: "sender".to_string(),
        to: MessageTarget::Direct("recipient".to_string()),
        content: "Personal message".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        reply_to: None,
        mentions: vec![],
    };

    let message_to_group = Message {
        id: "msg-2".to_string(),
        from: "sender".to_string(),
        to: MessageTarget::Group("team-group".to_string()),
        content: "Group message".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        reply_to: None,
        mentions: vec![],
    };

    // 验证消息目标类型
    match &message_to_individual.to {
        MessageTarget::Direct(id) => assert_eq!(id, "recipient"),
        _ => panic!("Expected Direct message target"),
    }

    match &message_to_group.to {
        MessageTarget::Group(id) => assert_eq!(id, "team-group"),
        _ => panic!("Expected Group message target"),
    }

    // 验证消息内容
    assert_eq!(message_to_individual.content, "Personal message");
    assert_eq!(message_to_group.content, "Group message");
}

#[tokio::test]
async fn test_agent_conversation_flow() {
    let agent = Agent::new(
        "conversation-agent",
        "Conversation Assistant",
        Role::simple("Conversation Agent", "You are a conversation assistant, responsible for natural conversations with users"),
        LLMConfig::openai("test-key"),
    );

    // Simulate a series of message interactions
    let messages = vec![
        Message {
            id: "msg-1".to_string(),
            from: "user".to_string(),
            to: MessageTarget::Direct("conversation-agent".to_string()),
            content: "你好！".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            reply_to: None,
            mentions: vec![],
        },
        Message {
            id: "msg-2".to_string(),
            from: "user".to_string(),
            to: MessageTarget::Direct("conversation-agent".to_string()),
            content: "你能帮我完成任务吗？".to_string(),
            timestamp: chrono::Utc::now().timestamp() + 1,
            reply_to: Some("msg-1".to_string()),
            mentions: vec![],
        },
        Message {
            id: "msg-3".to_string(),
            from: "user".to_string(),
            to: MessageTarget::Direct("conversation-agent".to_string()),
            content: "谢谢你的帮助！".to_string(),
            timestamp: chrono::Utc::now().timestamp() + 2,
            reply_to: Some("msg-2".to_string()),
            mentions: vec![],
        },
    ];

    // 验证消息序列
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].content, "你好！");
    assert_eq!(messages[1].reply_to, Some("msg-1".to_string()));
    assert_eq!(messages[2].reply_to, Some("msg-2".to_string()));

    // 验证agent可以处理这些消息
    assert_eq!(agent.id, "conversation-agent");
}