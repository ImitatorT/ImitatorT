//! 测试WatchdogAgent默认注册功能
//!
//! 验证每个Agent在创建时都会自动注册私聊和@提及监控

use imitatort::domain::agent::{Agent, Role, LLMConfig};
use imitatort::core::messaging::MessageBus;
use imitatort::application::autonomous::AutonomousAgent;
use imitatort::core::watchdog_agent::WatchdogAgent;
use std::sync::Arc;

#[tokio::test]
async fn test_autonomous_agent_registers_default_watchers() {
    // 创建WatchdogAgent
    let watchdog_agent = Arc::new(WatchdogAgent::new(
        Agent::new(
            "system_watchdog",
            "System Watchdog Agent",
            Role::simple("System", "System monitoring agent"),
            LLMConfig::openai("test-key"),
        )
    ));

    // 创建消息总线
    let message_bus = Arc::new(MessageBus::new());

    // 创建Agent，传入WatchdogAgent以自动注册默认监控
    let agent = Agent::new(
        "test_agent",
        "Test Agent",
        Role::simple("Tester", "A test agent"),
        LLMConfig::openai("test-key"),
    );

    let autonomous_agent = AutonomousAgent::new(
        agent,
        message_bus,
        Some(watchdog_agent.clone()),
    ).await.unwrap();

    // 验证私聊监控规则已注册
    assert!(watchdog_agent.has_rule(&format!("direct_msg_{}", autonomous_agent.id())));

    // 验证@提及监控规则已注册
    assert!(watchdog_agent.has_rule(&format!("mention_{}", autonomous_agent.id())));
}

#[tokio::test]
async fn test_autonomous_agent_without_watchdog_registers_no_watchers() {
    // 创建消息总线
    let message_bus = Arc::new(MessageBus::new());

    // 创建WatchdogAgent (用来验证规则不存在)
    let watchdog_agent = Arc::new(WatchdogAgent::new(
        Agent::new(
            "system_watchdog",
            "System Watchdog Agent",
            Role::simple("System", "System monitoring agent"),
            LLMConfig::openai("test-key"),
        )
    ));

    // 创建Agent，不传入WatchdogAgent
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
    ).await.unwrap();

    // 验证私聊监控规则未注册
    assert!(!watchdog_agent.has_rule(&format!("direct_msg_{}", autonomous_agent.id())));

    // 验证@提及监控规则未注册
    assert!(!watchdog_agent.has_rule(&format!("mention_{}", autonomous_agent.id())));
}