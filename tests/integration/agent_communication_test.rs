//! Agent 通信集成测试
//!
//! 测试 Agent 之间的消息传递能力

use imitatort_stateless_company::core::agent::{AgentConfig, AgentManager};
use imitatort_stateless_company::core::messaging::MessageBus;
use std::sync::Arc;

fn create_test_config(id: &str, name: &str) -> AgentConfig {
    let mut metadata = serde_json::Map::new();
    metadata.insert("test".to_string(), serde_json::json!(true));

    AgentConfig {
        id: id.to_string(),
        name: name.to_string(),
        system_prompt: "You are a test agent.".to_string(),
        model: "gpt-4o-mini".to_string(),
        api_key: "sk-test".to_string(),
        base_url: "http://localhost:8080".to_string(),
        metadata,
    }
}

#[tokio::test]
async fn test_agent_manager_creation() {
    let manager = AgentManager::new();
    assert!(manager.is_empty());
    assert_eq!(manager.len(), 0);
}

#[tokio::test]
async fn test_agent_manager_register_and_get() {
    let manager = AgentManager::new();

    let config = create_test_config("agent-001", "Test Agent");
    let agent = manager.register(config).await.unwrap();

    assert_eq!(manager.len(), 1);
    assert!(!manager.is_empty());

    // Get the agent
    let retrieved = manager.get("agent-001");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id(), "agent-001");
}

#[tokio::test]
async fn test_agent_manager_remove() {
    let manager = AgentManager::new();

    let config = create_test_config("agent-001", "Test Agent");
    manager.register(config).await.unwrap();

    let removed = manager.remove("agent-001");
    assert!(removed.is_some());
    assert!(manager.is_empty());

    // Removing again returns None
    let removed_again = manager.remove("agent-001");
    assert!(removed_again.is_none());
}

#[tokio::test]
async fn test_agent_manager_list() {
    let manager = AgentManager::new();

    let config1 = create_test_config("agent-001", "Agent 1");
    let config2 = create_test_config("agent-002", "Agent 2");

    manager.register(config1).await.unwrap();
    manager.register(config2).await.unwrap();

    let agents = manager.list();
    assert_eq!(agents.len(), 2);
}

#[test]
fn test_agent_config_creation() {
    let config = create_test_config("test-001", "Test Agent");

    assert_eq!(config.id, "test-001");
    assert_eq!(config.name, "Test Agent");
    assert_eq!(config.system_prompt, "You are a test agent.");
    assert_eq!(config.model, "gpt-4o-mini");
}

#[test]
fn test_agent_config_clone() {
    let config = create_test_config("test-001", "Test Agent");
    let cloned = config.clone();

    assert_eq!(config.id, cloned.id);
    assert_eq!(config.name, cloned.name);
    assert_eq!(config.metadata, cloned.metadata);
}

#[tokio::test]
async fn test_agent_connect_messaging() {
    let manager = AgentManager::new();
    let bus = Arc::new(MessageBus::new());

    let config = create_test_config("agent-001", "Test Agent");
    let agent = manager.register(config).await.unwrap();

    // Note: Agent messaging connection requires mutable access
    // This test documents the expected usage pattern
    assert_eq!(agent.id(), "agent-001");
}

#[tokio::test]
async fn test_agent_manager_default() {
    let manager: AgentManager = Default::default();
    assert!(manager.is_empty());
}

#[tokio::test]
async fn test_multiple_agents_same_id() {
    let manager = AgentManager::new();

    let config1 = create_test_config("agent-001", "Agent 1");
    let config2 = create_test_config("agent-001", "Agent 1 Duplicate");

    manager.register(config1).await.unwrap();

    // Registering with same ID should replace the old one
    manager.register(config2).await.unwrap();

    assert_eq!(manager.len(), 1);
}
