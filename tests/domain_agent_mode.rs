//! Agent模式测试
//!
//! 测试新的Agent模式功能

use imitatort::domain::agent::{Agent, AgentMode, TriggerCondition, Role, LLMConfig};

#[test]
fn test_agent_passive_mode_default() {
    let agent = Agent::new(
        "test_agent",
        "Test Agent",
        Role::simple("Tester", "A test agent"),
        LLMConfig::openai("test-key"),
    );

    // 验证默认模式为被动模式
    match agent.mode {
        AgentMode::Passive => assert!(true),
        _ => panic!("Expected passive mode as default"),
    }
}

#[test]
fn test_agent_active_mode_with_conditions() {
    let agent = Agent::new_with_mode(
        "active_agent",
        "Active Agent",
        Role::simple("Active Tester", "An active test agent"),
        LLMConfig::openai("test-key"),
        AgentMode::Active {
            watched_tools: vec!["tool1".to_string(), "tool2".to_string()],
            trigger_conditions: vec![
                TriggerCondition::NumericRange { min: 0.0, max: 100.0 },
                TriggerCondition::StringContains { content: "success".to_string() },
            ],
        },
    );

    // 验证主动模式设置
    match agent.mode {
        AgentMode::Active { watched_tools, trigger_conditions } => {
            assert_eq!(watched_tools.len(), 2);
            assert_eq!(watched_tools[0], "tool1");
            assert_eq!(watched_tools[1], "tool2");
            assert_eq!(trigger_conditions.len(), 2);
        },
        _ => panic!("Expected active mode"),
    }
}

#[test]
fn test_agent_mode_builder() {
    let agent = Agent::new(
        "builder_agent",
        "Builder Agent",
        Role::simple("Builder", "A builder agent"),
        LLMConfig::openai("test-key"),
    )
    .with_mode(AgentMode::Passive);

    match agent.mode {
        AgentMode::Passive => assert!(true),
        _ => panic!("Expected passive mode"),
    }

    let agent = agent.with_mode(AgentMode::Active {
        watched_tools: vec!["watched_tool".to_string()],
        trigger_conditions: vec![TriggerCondition::StatusMatches { expected_status: "ready".to_string() }],
    });

    match agent.mode {
        AgentMode::Active { watched_tools, trigger_conditions } => {
            assert_eq!(watched_tools.len(), 1);
            assert_eq!(watched_tools[0], "watched_tool");
            assert_eq!(trigger_conditions.len(), 1);
        },
        _ => panic!("Expected active mode"),
    }
}

#[test]
fn test_agent_department_setting_preserved() {
    let agent = Agent::new(
        "dept_agent",
        "Department Agent",
        Role::simple("Dept Member", "A department member agent"),
        LLMConfig::openai("test-key"),
    )
    .with_department("engineering")
    .with_mode(AgentMode::Active {
        watched_tools: vec!["eng_tool".to_string()],
        trigger_conditions: vec![],
    });

    assert_eq!(agent.department_id, Some("engineering".to_string()));

    match agent.mode {
        AgentMode::Active { watched_tools, .. } => {
            assert_eq!(watched_tools[0], "eng_tool");
        },
        _ => panic!("Expected active mode"),
    }
}