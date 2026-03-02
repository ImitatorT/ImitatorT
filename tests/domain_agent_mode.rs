//! Agent重构后功能测试
//!
//! 测试重构后的Agent功能，包括工具监听能力

use imitatort::core::watchdog_agent::{WatchdogAgent, WatchdogRule};
use imitatort::domain::agent::{Agent, LLMConfig, Role, TriggerCondition};
use std::sync::Arc;

#[test]
fn test_agent_creation_with_tool_watching() {
    let agent = Agent::new(
        "test_agent",
        "Test Agent",
        Role::simple("Tester", "A test agent"),
        LLMConfig::openai("test-key"),
    );

    // 验证Agent默认具有空的工具监听列表
    assert_eq!(agent.watched_tools.len(), 0);
    assert_eq!(agent.trigger_conditions.len(), 0);

    // 验证Agent可以通过构建器方法添加工具监听
    let agent_with_watching = Agent::new(
        "test_agent2",
        "Test Agent 2",
        Role::simple("Tester", "Another test agent"),
        LLMConfig::openai("test-key"),
    )
    .with_watched_tools(vec!["tool1".to_string(), "tool2".to_string()])
    .with_trigger_conditions(vec![
        TriggerCondition::NumericRange {
            min: 0.0,
            max: 100.0,
        },
        TriggerCondition::StringContains {
            content: "success".to_string(),
        },
    ]);

    assert_eq!(agent_with_watching.watched_tools.len(), 2);
    assert_eq!(agent_with_watching.trigger_conditions.len(), 2);
    assert_eq!(agent_with_watching.watched_tools[0], "tool1");
    assert_eq!(agent_with_watching.watched_tools[1], "tool2");
}

#[test]
fn test_agent_with_individual_watching_config() {
    let agent = Agent::new(
        "watch_agent",
        "Watcher Agent",
        Role::simple("Watcher", "An agent that watches tools"),
        LLMConfig::openai("test-key"),
    )
    .add_watched_tool("database_query")
    .add_watched_tool("api_call")
    .add_trigger_condition(TriggerCondition::NumericRange {
        min: 10.0,
        max: 90.0,
    })
    .add_trigger_condition(TriggerCondition::StringContains {
        content: "complete".to_string(),
    });

    assert_eq!(agent.watched_tools.len(), 2);
    assert_eq!(agent.trigger_conditions.len(), 2);
    assert_eq!(agent.watched_tools[0], "database_query");
    assert_eq!(agent.watched_tools[1], "api_call");
}

#[tokio::test]
async fn test_watchdog_agent_creation_and_rule_management() {
    let agent = Agent::new(
        "watchdog_system",
        "Watchdog Agent",
        Role::simple("System Monitor", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );

    let watchdog_agent = WatchdogAgent::new(agent);

    // 测试规则注册
    let rule = WatchdogRule::new(
        "test_rule",
        "test_tool",
        TriggerCondition::NumericRange {
            min: 10.0,
            max: 20.0,
        },
        "test_agent",
    );

    assert!(watchdog_agent.register_rule(rule).is_ok());
    assert!(watchdog_agent.has_rule("test_rule"));

    // 测试获取规则
    let retrieved_rule = watchdog_agent.get_rule("test_rule");
    assert!(retrieved_rule.is_some());
    assert_eq!(retrieved_rule.unwrap().id, "test_rule");

    // 测试移除规则
    let removed_rule = watchdog_agent.remove_rule("test_rule");
    assert!(removed_rule.is_some());
    assert!(!watchdog_agent.has_rule("test_rule"));
}

#[tokio::test]
async fn test_watchdog_agent_event_processing() {
    use imitatort::core::watchdog_agent::ToolExecutionEvent;
    use imitatort::domain::tool::ToolCallContext;
    use serde_json::json;

    let agent = Agent::new(
        "watchdog_system",
        "Watchdog Agent",
        Role::simple("System Monitor", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );

    let watchdog_agent = WatchdogAgent::new(agent);

    // 注册一个数值范围触发规则
    let rule = WatchdogRule::new(
        "numeric_rule",
        "test_tool",
        TriggerCondition::NumericRange {
            min: 10.0,
            max: 20.0,
        },
        "triggered_agent",
    );

    watchdog_agent.register_rule(rule).unwrap();

    // 测试在范围内的事件应触发
    let event_in_range = ToolExecutionEvent::PostExecute {
        tool_id: "test_tool".to_string(),
        result: json!(15.0),
        context: ToolCallContext::new("caller".to_string()),
    };

    let triggered_agents = watchdog_agent.process_event(&event_in_range).await.unwrap();
    assert_eq!(triggered_agents, vec!["triggered_agent"]);

    // 测试超出范围的事件不应触发
    let event_out_of_range = ToolExecutionEvent::PostExecute {
        tool_id: "test_tool".to_string(),
        result: json!(25.0),
        context: ToolCallContext::new("caller".to_string()),
    };

    let triggered_agents = watchdog_agent
        .process_event(&event_out_of_range)
        .await
        .unwrap();
    assert_eq!(triggered_agents.len(), 0);
}

#[test]
fn test_agent_department_setting_preserved_after_refactor() {
    let agent = Agent::new(
        "dept_agent",
        "Department Agent",
        Role::simple("Dept Member", "A department member agent"),
        LLMConfig::openai("test-key"),
    )
    .with_department("engineering")
    .add_watched_tool("eng_tool");

    assert_eq!(agent.department_id, Some("engineering".to_string()));
    assert_eq!(agent.watched_tools[0], "eng_tool");
}
