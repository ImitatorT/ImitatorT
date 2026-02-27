//! Watchdog框架测试
//!
//! 测试Watchdog框架的核心功能

use imitatort::core::watchdog::{
    WatchdogFramework, WatchdogRule, TriggerCondition, ToolExecutionEvent,
    client::WatchdogClient,
};
use imitatort::domain::tool::ToolCallContext;
use serde_json::json;

#[tokio::test]
async fn test_watchdog_framework_creation() {
    let framework = WatchdogFramework::new();
    assert!(framework.is_enabled().await);
}

#[tokio::test]
async fn test_watchdog_rule_registration() {
    let framework = WatchdogFramework::new();

    let rule = WatchdogRule::new(
        "test_rule",
        "test_tool",
        TriggerCondition::NumericRange { min: 10.0, max: 20.0 },
        "test_agent",
    );

    assert!(framework.register_rule(rule).is_ok());
    assert!(framework.has_rule("test_rule"));
}

#[tokio::test]
async fn test_watchdog_rule_activation() {
    let framework = WatchdogFramework::new();

    let rule = WatchdogRule::new(
        "activation_test_rule",
        "test_tool",
        TriggerCondition::NumericRange { min: 5.0, max: 15.0 },
        "target_agent",
    );

    framework.register_rule(rule).unwrap();

    // 测试符合条件的事件
    let triggered: Vec<String> = framework.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "test_tool".to_string(),
        result: json!(10.0),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, vec!["target_agent"]);

    // 测试不符合条件的事件
    let triggered: Vec<String> = framework.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "test_tool".to_string(),
        result: json!(25.0),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, Vec::<String>::new());
}

#[tokio::test]
async fn test_watchdog_client() {
    let framework = std::sync::Arc::new(WatchdogFramework::new());
    let client = WatchdogClient::new(framework.clone(), "test_agent");

    // 注册规则
    client.register_rule(
        "client_test_rule".to_string(),
        "client_test_tool".to_string(),
        TriggerCondition::StringContains { content: "success".to_string() },
    ).await.unwrap();

    assert!(client.has_rule("client_test_rule").await);

    // 测试事件处理
    let triggered: Vec<String> = client.handle_event(&ToolExecutionEvent::PostExecute {
        tool_id: "client_test_tool".to_string(),
        result: json!("operation was successful"),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, vec!["test_agent"]);
}

#[tokio::test]
async fn test_watchdog_rule_string_matching() {
    let framework = WatchdogFramework::new();

    let rule = WatchdogRule::new(
        "string_match_rule",
        "string_test_tool",
        TriggerCondition::StringContains { content: "error".to_string() },
        "alert_agent",
    );

    framework.register_rule(rule).unwrap();

    // 测试匹配字符串
    let triggered: Vec<String> = framework.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "string_test_tool".to_string(),
        result: json!("An error occurred in the system"),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, vec!["alert_agent"]);

    // 测试不匹配字符串
    let triggered: Vec<String> = framework.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "string_test_tool".to_string(),
        result: json!("Operation completed successfully"),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, Vec::<String>::new());
}

#[tokio::test]
async fn test_watchdog_rule_status_matching() {
    let framework = WatchdogFramework::new();

    let rule = WatchdogRule::new(
        "status_match_rule",
        "status_test_tool",
        TriggerCondition::StatusMatches { expected_status: "failed".to_string() },
        "failure_handler",
    );

    framework.register_rule(rule).unwrap();

    // 测试状态匹配
    let triggered: Vec<String> = framework.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "status_test_tool".to_string(),
        result: json!({"status": "failed", "details": "Something went wrong"}),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, vec!["failure_handler"]);
}

#[tokio::test]
async fn test_watchdog_framework_disable_enable() {
    let framework = WatchdogFramework::new();

    let rule = WatchdogRule::new(
        "toggle_rule",
        "toggle_test_tool",
        TriggerCondition::NumericRange { min: 0.0, max: 100.0 },
        "test_agent",
    );

    framework.register_rule(rule).unwrap();

    // 禁用框架
    framework.set_enabled(false).await;
    let triggered: Vec<String> = framework.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "toggle_test_tool".to_string(),
        result: json!(50.0),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, Vec::<String>::new());

    // 重新启用框架
    framework.set_enabled(true).await;
    let triggered: Vec<String> = framework.process_event(&ToolExecutionEvent::PostExecute {
        tool_id: "toggle_test_tool".to_string(),
        result: json!(50.0),
        context: ToolCallContext::new("test_caller".to_string()),
    }).await.unwrap();

    assert_eq!(triggered, vec!["test_agent"]);
}