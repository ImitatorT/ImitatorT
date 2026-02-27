//! WatchdogAgent框架测试
//!
//! 测试WatchdogAgent框架的核心功能

use imitatort::core::watchdog_agent::{
    WatchdogAgent, WatchdogRule, ToolExecutionEvent,
    WatchdogClient,
};
use imitatort::domain::{agent::{Agent, TriggerCondition}, agent::Role, agent::LLMConfig};
use imitatort::domain::tool::ToolCallContext;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_watchdog_agent_creation() {
    let agent = Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );
    let watchdog_agent = WatchdogAgent::new(agent);
    assert!(watchdog_agent.is_enabled().await);
}

#[tokio::test]
async fn test_watchdog_rule_registration() {
    let agent = Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );
    let watchdog_agent = WatchdogAgent::new(agent);

    let rule = WatchdogRule::new(
        "test_rule",
        "test_tool",
        TriggerCondition::NumericRange { min: 10.0, max: 20.0 },
        "test_agent",
    );

    assert!(watchdog_agent.register_rule(rule).is_ok());
    assert!(watchdog_agent.has_rule("test_rule"));
}

#[tokio::test]
async fn test_watchdog_rule_activation() {
    let agent = Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );
    let watchdog_agent = WatchdogAgent::new(agent);

    let rule = WatchdogRule::new(
        "activation_test_rule",
        "test_tool",
        TriggerCondition::NumericRange { min: 5.0, max: 15.0 },
        "target_agent",
    );

    watchdog_agent.register_rule(rule).unwrap();

    // 测试符合条件的事件
    let triggered: Vec<String> = watchdog_agent.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "test_tool".to_string(),
        result: json!(10.0),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, vec!["target_agent"]);

    // 测试不符合条件的事件
    let triggered: Vec<String> = watchdog_agent.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "test_tool".to_string(),
        result: json!(25.0),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, Vec::<String>::new());
}

#[tokio::test]
async fn test_watchdog_client() {
    let agent = Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );
    let watchdog_agent = Arc::new(WatchdogAgent::new(agent));
    let client = WatchdogClient::new(watchdog_agent.clone());

    // 注册规则
    client.register_tool_watcher(
        "test_agent",
        "client_test_tool",
        TriggerCondition::StringContains { content: "success".to_string() },
    ).unwrap();

    assert!(watchdog_agent.has_rule(&format!("rule_test_agent_client_test_tool")));

    // 测试事件处理
    let triggered: Vec<String> = watchdog_agent.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "client_test_tool".to_string(),
        result: json!("operation was successful"),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, vec!["test_agent"]);
}

#[tokio::test]
async fn test_watchdog_rule_string_matching() {
    let agent = Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );
    let watchdog_agent = WatchdogAgent::new(agent);

    let rule = WatchdogRule::new(
        "string_match_rule",
        "string_test_tool",
        TriggerCondition::StringContains { content: "error".to_string() },
        "alert_agent",
    );

    watchdog_agent.register_rule(rule).unwrap();

    // 测试匹配字符串
    let triggered: Vec<String> = watchdog_agent.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "string_test_tool".to_string(),
        result: json!("An error occurred in the system"),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, vec!["alert_agent"]);

    // 测试不匹配字符串
    let triggered: Vec<String> = watchdog_agent.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "string_test_tool".to_string(),
        result: json!("Operation completed successfully"),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, Vec::<String>::new());
}

#[tokio::test]
async fn test_watchdog_rule_status_matching() {
    let agent = Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );
    let watchdog_agent = WatchdogAgent::new(agent);

    let rule = WatchdogRule::new(
        "status_match_rule",
        "status_test_tool",
        TriggerCondition::StatusMatches { expected_status: "failed".to_string() },
        "failure_handler",
    );

    watchdog_agent.register_rule(rule).unwrap();

    // 测试状态匹配
    let triggered: Vec<String> = watchdog_agent.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "status_test_tool".to_string(),
        result: json!("failed"),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, vec!["failure_handler"]);
}

#[tokio::test]
async fn test_watchdog_agent_disable_enable() {
    let agent = Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );
    let watchdog_agent = WatchdogAgent::new(agent);

    let rule = WatchdogRule::new(
        "toggle_rule",
        "toggle_test_tool",
        TriggerCondition::NumericRange { min: 0.0, max: 100.0 },
        "test_agent",
    );

    watchdog_agent.register_rule(rule).unwrap();

    // 禁用框架
    watchdog_agent.set_enabled(false).await;
    let triggered: Vec<String> = watchdog_agent.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "toggle_test_tool".to_string(),
        result: json!(50.0),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, Vec::<String>::new());

    // 重新启用框架
    watchdog_agent.set_enabled(true).await;
    let triggered: Vec<String> = watchdog_agent.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "toggle_test_tool".to_string(),
        result: json!(50.0),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, vec!["test_agent"]);
}

#[tokio::test]
async fn test_watchdog_agent_private_message_watcher() {
    let agent = Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );
    let watchdog_agent = WatchdogAgent::new(agent);

    let test_agent_id = "test_agent";

    // 为Agent注册私聊监控
    watchdog_agent.register_direct_message_watcher(test_agent_id)
        .expect("Failed to register private message watcher");

    // 验证规则已注册
    assert!(watchdog_agent.has_rule(&format!("direct_msg_{}", test_agent_id)));

    // 测试触发事件
    let event = ToolExecutionEvent::PostExecute {
        tool_id: "message.send_direct".to_string(),
        result: json!({"target": test_agent_id}), // 包含目标Agent ID
        context: ToolCallContext::new("sender".to_string()),
    };

    let triggered_agents = watchdog_agent.process_event(&event).await.unwrap();

    assert!(triggered_agents.contains(&test_agent_id.to_string()));
}

#[tokio::test]
async fn test_watchdog_agent_mention_watcher() {
    let agent = Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    );
    let watchdog_agent = WatchdogAgent::new(agent);

    let test_agent_id = "test_agent";

    // 为Agent注册艾特(@)监控
    watchdog_agent.register_mention_watcher(test_agent_id)
        .expect("Failed to register mention watcher");

    // 验证规则已注册
    assert!(watchdog_agent.has_rule(&format!("mention_{}", test_agent_id)));

    // 测试触发事件
    let event = ToolExecutionEvent::PostExecute {
        tool_id: "message.send_group".to_string(),
        result: json!({"mention_agent_ids": [test_agent_id]}), // 包含被艾特的Agent ID
        context: ToolCallContext::new("sender".to_string()),
    };

    let triggered_agents = watchdog_agent.process_event(&event).await.unwrap();

    assert!(triggered_agents.contains(&test_agent_id.to_string()));
}