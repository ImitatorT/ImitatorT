//! 框架内置工具实现测试

use imitatort::core::messaging::MessageBus;
use imitatort::core::tool::ToolRegistry;
use imitatort::domain::tool::ToolCallContext;
use imitatort::domain::{Agent, LLMConfig, Organization, Role};
use imitatort::infrastructure::tool::{FrameworkToolExecutor, ToolEnvironment, ToolExecutor};
use std::sync::Arc;
use tokio::sync::RwLock;

fn create_test_environment() -> ToolEnvironment {
    let message_bus = Arc::new(MessageBus::new());
    let organization = Arc::new(RwLock::new(Organization::new()));
    let tool_registry = Arc::new(ToolRegistry::new());
    let capability_registry = Arc::new(imitatort::core::capability::CapabilityRegistry::new());
    let store = Arc::new(imitatort::infrastructure::store::SqliteStore::new_in_memory().unwrap());
    let skill_manager = Arc::new(imitatort::core::skill::SkillManager::new(
        tool_registry.clone(),
        capability_registry,
    ));

    ToolEnvironment::new(
        message_bus,
        organization,
        tool_registry,
        store,
        skill_manager,
    )
}

#[tokio::test]
async fn test_framework_tool_executor_supported_tools() {
    let env = create_test_environment();
    let executor = FrameworkToolExecutor::new(env);

    let tools = executor.supported_tools();
    assert!(!tools.is_empty());
    assert!(tools.contains(&"tool.search".to_string()));
    assert!(tools.contains(&"time.now".to_string()));
    assert!(tools.contains(&"org.find_agents".to_string()));
}

#[tokio::test]
async fn test_tool_search() {
    let env = create_test_environment();
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    // 测试模糊搜索
    let result = executor
        .execute(
            "tool.search",
            serde_json::json!({
                "query": "time",
                "match_type": "fuzzy"
            }),
            &context,
        )
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.data.get("tools").is_some());

    // 测试精确搜索
    let result = executor
        .execute(
            "tool.search",
            serde_json::json!({
                "query": "time.now",
                "match_type": "exact"
            }),
            &context,
        )
        .await
        .unwrap();

    assert!(result.success);
    let tools = result.data["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
}

#[tokio::test]
async fn test_time_now() {
    let env = create_test_environment();
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let result = executor
        .execute("time.now", serde_json::json!({}), &context)
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.data.get("timestamp").is_some());
    assert!(result.data.get("iso").is_some());
    assert!(result.data.get("date").is_some());
}

#[tokio::test]
async fn test_org_find_agents() {
    let env = create_test_environment();
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let result = executor
        .execute(
            "org.find_agents",
            serde_json::json!({
                "query_type": "name",
                "query_value": "test",
                "fuzzy_match": true
            }),
            &context,
        )
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.data.get("agents").is_some());
}

#[tokio::test]
async fn test_unknown_tool() {
    let env = create_test_environment();
    let executor = FrameworkToolExecutor::new(env);
    let context = ToolCallContext::new("test-agent");

    let result = executor
        .execute("unknown.tool", serde_json::json!({}), &context)
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.error.is_some());
}
