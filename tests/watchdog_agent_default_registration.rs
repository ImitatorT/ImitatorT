//! 测试 WatchdogAgent 默认注册功能
//!
//! 验证每个 Agent 在创建时都会自动注册私聊和@提及监控

use imitatort::application::autonomous::AutonomousAgent;
use imitatort::core::messaging::MessageBus;
use imitatort::core::watchdog_agent::WatchdogAgent;
use imitatort::domain::agent::{Agent, LLMConfig, Role};
use imitatort::infrastructure::tool::ToolExecutor as ToolExecutorTrait;
use imitatort::domain::tool::ToolCallContext;
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
async fn test_autonomous_agent_registers_default_watchers() {
    // 创建 WatchdogAgent
    let watchdog_agent = Arc::new(WatchdogAgent::new(Agent::new(
        "system_watchdog",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    ), create_mock_tool_executor()));

    // 创建消息总线
    let message_bus = Arc::new(MessageBus::new());

    // 创建 Agent，传入 WatchdogAgent 以自动注册默认监控
    let agent = Agent::new(
        "test_agent",
        "Test Agent",
        Role::simple("Tester", "A test agent"),
        LLMConfig::openai("test-key"),
    );

    let autonomous_agent = AutonomousAgent::new(agent, message_bus, Some(watchdog_agent.clone()))
        .await
        .unwrap();

    // 验证私聊监控规则已注册
    assert!(watchdog_agent.has_rule(&format!("direct_msg_{}", autonomous_agent.id())));

    // 验证@提及监控规则已注册
    assert!(watchdog_agent.has_rule(&format!("mention_{}", autonomous_agent.id())));
}

#[tokio::test]
async fn test_autonomous_agent_without_watchdog_registers_no_watchers() {
    // 创建消息总线
    let message_bus = Arc::new(MessageBus::new());

    // 创建 WatchdogAgent (用来验证规则不存在)
    let watchdog_agent = Arc::new(WatchdogAgent::new(Agent::new(
        "system_watchdog",
        "System Watchdog Agent",
        Role::simple("System", "System monitoring agent"),
        LLMConfig::openai("test-key"),
    ), create_mock_tool_executor()));

    // 创建 Agent，不传入 WatchdogAgent
    let agent = Agent::new(
        "test_agent_no_watchdog",
        "Test Agent No Watchdog",
        Role::simple("Tester", "A test agent without watchdog"),
        LLMConfig::openai("test-key"),
    );

    let autonomous_agent = AutonomousAgent::new(
        agent,
        message_bus,
        None, // No watchdog agent
    )
    .await
    .unwrap();

    // 验证私聊监控规则未注册
    assert!(!watchdog_agent.has_rule(&format!("direct_msg_{}", autonomous_agent.id())));

    // 验证@提及监控规则未注册
    assert!(!watchdog_agent.has_rule(&format!("mention_{}", autonomous_agent.id())));
}
