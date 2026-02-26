//! Tool 执行器接口测试

use imitatort_stateless_company::infrastructure::tool::{FnToolExecutor, ToolContext, ToolExecutorRegistry, ToolResult, ToolExecutor};
use imitatort_stateless_company::core::skill::SkillManager;
use imitatort_stateless_company::core::tool::ToolRegistry;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_tool_executor_registry() {
    let tool_registry = Arc::new(ToolRegistry::new());
    let mut registry = ToolExecutorRegistry::with_default_skill_manager(tool_registry);

    // 创建一个简单的函数式执行器
    let executor = FnToolExecutor::new("test.echo", |params| async move {
        Ok(json!({ "echo": params }))
    });

    registry.register(Box::new(executor));

    // 创建一个工具调用上下文
    let context = imitatort_stateless_company::domain::tool::ToolCallContext::new("test-agent");

    // 测试执行
    let result = registry.execute("test.echo", json!("hello"), &context).await.unwrap();
    assert!(result.success);
    assert_eq!(result.data["echo"], "hello");

    // 测试不存在的工具
    let result = registry.execute("nonexistent", json!({}), &context).await.unwrap();
    assert!(!result.success);
    assert!(result.error.is_some());
}

#[tokio::test]
async fn test_fn_tool_executor() {
    let executor = FnToolExecutor::new("math.add", |params| async move {
        let a = params["a"].as_i64().unwrap_or(0);
        let b = params["b"].as_i64().unwrap_or(0);
        Ok(json!({ "result": a + b }))
    });

    assert!(executor.can_execute("math.add"));
    assert!(!executor.can_execute("math.sub"));

    let context = imitatort_stateless_company::domain::tool::ToolCallContext::new("test-agent");
    let result = executor.execute("math.add", json!({"a": 1, "b": 2}), &context).await.unwrap();
    assert_eq!(result["result"], 3);
}

#[test]
fn test_tool_result() {
    let success = ToolResult::success(json!({"data": "test"}));
    assert!(success.success);
    assert!(success.error.is_none());

    let error = ToolResult::error("something went wrong");
    assert!(!error.success);
    assert_eq!(error.error.unwrap(), "something went wrong");
}

#[test]
fn test_tool_context() {
    let ctx = ToolContext::new("agent-1")
        .with_metadata("session", "abc123")
        .with_metadata("ip", "127.0.0.1");

    assert_eq!(ctx.caller_id, "agent-1");
    assert_eq!(ctx.metadata.get("session").unwrap(), "abc123");
}
