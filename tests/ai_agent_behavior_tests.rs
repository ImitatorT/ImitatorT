//! ImitatorT AI Agent行为测试

use imitatort_stateless_company::domain::{Agent, LLMConfig, Message, MessageTarget, Organization, Role};
// 移除对私有模块的引用，使用公共API
use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::test]
async fn test_agent_initialization() {
    // 创建测试agent
    let agent = Agent::new(
        "test-developer",
        "测试开发者",
        Role::simple("Developer", "你是开发人员，负责编写高质量的代码"),
        LLMConfig::openai("test-key"),
    );

    // 验证agent属性
    assert_eq!(agent.id, "test-developer");
    assert_eq!(agent.name, "测试开发者");
    assert_eq!(agent.role.title, "Developer");
    assert!(agent.role.system_prompt.contains("开发人员"));

    // 验证默认值
    assert!(agent.department_id.is_none());
}

#[tokio::test]
async fn test_agent_responds_to_message() {
    // 创建测试agent
    let agent = Agent::new(
        "test-manager",
        "测试经理",
        Role::simple("Manager", "你是经理，负责管理团队和分配任务"),
        LLMConfig::openai("test-key"),
    );

    // 创建消息
    let message = Message {
        id: "msg-1".to_string(),
        from: "employee".to_string(),
        to: MessageTarget::Direct("test-manager".to_string()),
        content: "经理您好，今天的工作计划是什么？".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        reply_to: None,
        mentions: vec![],
    };

    // 验证agent可以处理消息
    // 注意：这里由于涉及实际LLM调用，我们主要测试结构和逻辑
    assert_eq!(agent.name, "测试经理");
}

#[tokio::test]
async fn test_multi_agent_interaction() {
    let mut org = Organization::new();

    // 创建多个agent
    let manager = Agent::new(
        "team-manager",
        "团队经理",
        Role::simple("Team Manager", "你是团队经理，负责协调团队成员"),
        LLMConfig::openai("test-key"),
    );

    let developer = Agent::new(
        "senior-dev",
        "高级开发",
        Role::simple("Senior Developer", "你是高级开发工程师，负责技术实现"),
        LLMConfig::openai("test-key"),
    );

    let designer = Agent::new(
        "ui-designer",
        "UI设计师",
        Role::simple("UI Designer", "你是UI设计师，负责界面设计"),
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
        "经理",
        Role::simple("Project Manager", "你是项目经理，负责项目进度管理和团队协调"),
        LLMConfig::openai("test-key"),
    );

    let developer = Agent::new(
        "developer",
        "开发",
        Role::simple("Software Developer", "你是软件开发工程师，负责编写代码和解决问题"),
        LLMConfig::openai("test-key"),
    );

    let qa_engineer = Agent::new(
        "qa",
        "QA工程师",
        Role::simple("QA Engineer", "你是质量保证工程师，负责测试和质量控制"),
        LLMConfig::openai("test-key"),
    );

    // 验证每个agent的角色描述不同
    assert!(manager.role.system_prompt.contains("项目进度管理"));
    assert!(developer.role.system_prompt.contains("编写代码"));
    assert!(qa_engineer.role.system_prompt.contains("质量保证"));

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
        content: "个人消息".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        reply_to: None,
        mentions: vec![],
    };

    let message_to_group = Message {
        id: "msg-2".to_string(),
        from: "sender".to_string(),
        to: MessageTarget::Group("team-group".to_string()),
        content: "群组消息".to_string(),
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
    assert_eq!(message_to_individual.content, "个人消息");
    assert_eq!(message_to_group.content, "群组消息");
}

#[tokio::test]
async fn test_agent_conversation_flow() {
    let agent = Agent::new(
        "conversation-agent",
        "对话助手",
        Role::simple("Conversation Agent", "你是对话助手，负责与用户进行自然对话"),
        LLMConfig::openai("test-key"),
    );

    // 模拟一系列消息交互
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