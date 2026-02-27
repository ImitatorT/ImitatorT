//! Agent与WatchdogAgent集成测试
//!
//! 验证Agent与WatchdogAgent的协同工作

use imitatort::domain::agent::{Agent, TriggerCondition, Role, LLMConfig};
use imitatort::core::watchdog_agent::{WatchdogAgent, WatchdogRule, ToolExecutionEvent, WatchdogClient};
use imitatort::domain::tool::ToolCallContext;
use imitatort::application::autonomous::AutonomousAgent;
use imitatort::core::messaging::MessageBus;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_agent_with_watchdog_integration() {
    // 创建WatchdogAgent
    let watchdog_agent = Arc::new(WatchdogAgent::new(
        Agent::new(
            "watchdog_system",
            "System Watchdog Agent",
            Role::simple("System", "System monitoring agent"),
            LLMConfig::openai("test-key"),
        )
    ));

    // 创建一个需要监控工具的Agent
    let agent = Agent::new(
        "monitored_test_agent",
        "Monitored Test Agent",
        Role::simple("Tester", "A monitored test agent"),
        LLMConfig::openai("test-key"),
    );

    let message_bus = Arc::new(MessageBus::new());

    let autonomous_agent = AutonomousAgent::new(
        agent,
        message_bus,
        None,
    ).await.unwrap();

    // 创建Watchdog客户端来注册规则
    let watchdog_client = WatchdogClient::new(watchdog_agent.clone());

    // 为Agent注册工具监控规则
    let condition = TriggerCondition::NumericRange { min: 20.0, max: 30.0 };
    watchdog_client.register_tool_watcher(autonomous_agent.id(), "temperature_monitor", condition)
        .expect("Failed to register tool watcher");

    // 验证规则已注册
    assert!(watchdog_agent.has_rule(&format!("rule_{}_{}", autonomous_agent.id(), "temperature_monitor")));

    // 触发一个符合条件的工具执行事件
    let event = ToolExecutionEvent::PostExecute {
        tool_id: "temperature_monitor".to_string(),
        result: json!(25.0), // 在20-30范围内，应该触发
        context: ToolCallContext::new("sensor".to_string()),
    };

    // 处理事件
    let triggered_agents = watchdog_agent.process_event(&event).await.unwrap();

    // 验证Agent被触发
    assert!(triggered_agents.contains(&autonomous_agent.id().to_string()));
}

#[tokio::test]
async fn test_watchdog_agent_private_message_watcher() {
    let watchdog_agent = Arc::new(WatchdogAgent::new(
        Agent::new(
            "watchdog_system",
            "System Watchdog Agent",
            Role::simple("System", "System monitoring agent"),
            LLMConfig::openai("test-key"),
        )
    ));

    let test_agent_id = "test_agent_for_private_msg";

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
    let watchdog_agent = Arc::new(WatchdogAgent::new(
        Agent::new(
            "watchdog_system",
            "System Watchdog Agent",
            Role::simple("System", "System monitoring agent"),
            LLMConfig::openai("test-key"),
        )
    ));

    let test_agent_id = "test_agent_for_mention";

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

#[tokio::test]
async fn test_message_mentions_extraction() {
    use imitatort::domain::{Message, Group};

    // 测试消息中的@提及功能
    let group = Group::new(
        "test_group",
        "Test Group",
        "creator",
        vec!["alice".to_string(), "bob".to_string(), "charlie".to_string()],
    );

    // 创建一个包含@提及的消息
    let message = Message::group("sender", "test_group", "Hi @alice and @bob, how are you?")
        .with_mention("alice")
        .with_mention("bob");

    // 验证消息中的提及
    assert!(message.mentions.contains(&"alice".to_string()));
    assert!(message.mentions.contains(&"bob".to_string()));
    assert!(!message.mentions.contains(&"charlie".to_string())); // charlie没有被提及
}