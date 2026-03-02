//! Agent 与 WatchdogAgent 集成测试
//!
//! 验证 Agent 与 WatchdogAgent 的协同工作

use imitatort::application::autonomous::AutonomousAgent;
use imitatort::core::messaging::MessageBus;
use imitatort::core::watchdog_agent::{
    ToolExecutionEvent, WatchdogAgent, WatchdogClient, WatchdogRule,
};
use imitatort::domain::agent::{Agent, LLMConfig, Role, TriggerCondition};
use imitatort::domain::tool::ToolCallContext;
use imitatort::infrastructure::tool::ToolExecutor as ToolExecutorTrait;
use serde_json::{json, Value};
use std::sync::Arc;

/// Mock Tool Executor for testing
#[derive(Debug)]
struct MockToolExecutor;

#[async_trait::async_trait]
impl ToolExecutorTrait for MockToolExecutor {
    async fn execute(
        &self,
        _tool_id: &str,
        _params: Value,
        _context: &ToolCallContext,
    ) -> anyhow::Result<Value> {
        Ok(json!({"mock": "result"}))
    }

    fn can_execute(&self, _tool_id: &str) -> bool {
        true
    }

    fn supported_tools(&self) -> Vec<String> {
        vec!["mock_tool".to_string()]
    }
}

fn create_mock_tool_executor() -> Arc<dyn ToolExecutorTrait> {
    Arc::new(MockToolExecutor)
}

#[tokio::test]
async fn test_agent_with_watchdog_integration() {
    // 创建 WatchdogAgent
    let watchdog_agent = Arc::new(WatchdogAgent::new(Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    ), create_mock_tool_executor()));

    // 创建一个需要监控工具的 Agent
    let agent = Agent::new(
        "monitored_test_agent",
        "Monitored Test Agent",
        Role::simple("Tester", "A monitored test agent"),
        LLMConfig::openai("test-key"),
    );

    let message_bus = Arc::new(MessageBus::new());

    let autonomous_agent = AutonomousAgent::new(agent, message_bus, None)
        .await
        .unwrap();

    // 创建 Watchdog 客户端来注册规则
    let watchdog_client = WatchdogClient::new(watchdog_agent.clone());

    // 为 Agent 注册工具监控规则
    let condition = TriggerCondition::NumericRange {
        min: 20.0,
        max: 30.0,
    };
    watchdog_client
        .register_tool_watcher(autonomous_agent.id(), "temperature_monitor", condition)
        .expect("Failed to register tool watcher");

    // 验证规则已注册
    assert!(watchdog_agent.has_rule(&format!(
        "rule_{}_{}",
        autonomous_agent.id(),
        "temperature_monitor"
    )));

    // 触发一个符合条件的工具执行事件
    let event = ToolExecutionEvent::PostExecute {
        tool_id: "temperature_monitor".to_string(),
        result: json!(25.0), // 在 20-30 范围内，应该触发
        context: ToolCallContext::new("sensor".to_string()),
    };

    // 处理事件
    let triggered_agents = watchdog_agent.process_event(&event).await.unwrap();

    // 验证 Agent 被触发
    assert!(triggered_agents.contains(&autonomous_agent.id().to_string()));
}

#[tokio::test]
async fn test_watchdog_agent_private_message_watcher() {
    let watchdog_agent = Arc::new(WatchdogAgent::new(Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    ), create_mock_tool_executor()));

    let test_agent_id = "test_agent_for_private_msg";

    // 为 Agent 注册私聊监控
    watchdog_agent
        .register_direct_message_watcher(test_agent_id)
        .expect("Failed to register private message watcher");

    // 验证规则已注册
    assert!(watchdog_agent.has_rule(&format!("direct_msg_{}", test_agent_id)));

    // 测试触发事件
    let event = ToolExecutionEvent::PostExecute {
        tool_id: "message.send_direct".to_string(),
        result: json!({"target": test_agent_id}), // 包含目标 Agent ID
        context: ToolCallContext::new("sender".to_string()),
    };

    let triggered_agents = watchdog_agent.process_event(&event).await.unwrap();

    assert!(triggered_agents.contains(&test_agent_id.to_string()));
}

#[tokio::test]
async fn test_watchdog_agent_mention_watcher() {
    let watchdog_agent = Arc::new(WatchdogAgent::new(Agent::new(
        "watchdog_system",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    ), create_mock_tool_executor()));

    let test_agent_id = "test_agent_for_mention";

    // 为 Agent 注册艾特 (@) 监控
    watchdog_agent
        .register_mention_watcher(test_agent_id)
        .expect("Failed to register mention watcher");

    // 验证规则已注册
    assert!(watchdog_agent.has_rule(&format!("mention_{}", test_agent_id)));

    // 测试触发事件
    let event = ToolExecutionEvent::PostExecute {
        tool_id: "message.send_group".to_string(),
        result: json!({"mention_agent_ids": [test_agent_id]}), // 包含被艾特的 Agent ID
        context: ToolCallContext::new("sender".to_string()),
    };

    let triggered_agents = watchdog_agent.process_event(&event).await.unwrap();

    assert!(triggered_agents.contains(&test_agent_id.to_string()));
}

#[tokio::test]
async fn test_message_mentions_extraction() {
    use imitatort::domain::{Group, Message};

    // 测试消息中的@提及功能
    let group = Group::new(
        "test_group",
        "Test Group",
        "creator",
        vec![
            "alice".to_string(),
            "bob".to_string(),
            "charlie".to_string(),
        ],
    );

    // 创建一个包含@提及的消息
    let message = Message::group("sender", "test_group", "Hi @alice and @bob, how are you?")
        .with_mention("alice")
        .with_mention("bob");

    // 验证消息中的提及
    assert!(message.mentions.contains(&"alice".to_string()));
    assert!(message.mentions.contains(&"bob".to_string()));
    assert!(!message.mentions.contains(&"charlie".to_string())); // charlie 没有被提及
}
