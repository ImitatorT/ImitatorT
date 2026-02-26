use std::collections::HashMap;
use std::sync::Arc;

use imitatort_stateless_company::infrastructure::capability::executor::{CapabilityExecutor, CapabilityExecutorRegistry, FnCapabilityExecutor};
use imitatort_stateless_company::domain::capability::*;

#[tokio::test]
async fn test_fn_capability_executor() {
    let executor = FnCapabilityExecutor::new("test-executor".to_string(), |params| {
        Box::pin(async move {
            let input = params.get("input").and_then(|v| v.as_str()).unwrap_or("");
            Ok(serde_json::json!({"result": format!("Processed: {}", input)}))
        })
    });

    let mut params = HashMap::new();
    params.insert("input".to_string(), serde_json::json!("hello"));

    let result = executor.execute(params).await.unwrap();
    assert_eq!(result["result"].as_str().unwrap(), "Processed: hello");
}

#[tokio::test]
async fn test_capability_executor_registry() {
    let registry = CapabilityExecutorRegistry::new();

    let executor = FnCapabilityExecutor::new("test-executor".to_string(), |params| {
        Box::pin(async move {
            let input = params.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(serde_json::json!({"result": input * 2}))
        })
    });

    registry.register("test-capability".to_string(), Box::new(executor)).unwrap();

    let executor_opt = registry.get("test-capability").await;
    assert!(executor_opt.is_some());

    let executor = executor_opt.unwrap();
    let mut params = HashMap::new();
    params.insert("value".to_string(), serde_json::json!(5));

    let result = executor.execute(params).await.unwrap();
    assert_eq!(result["result"].as_i64().unwrap(), 10);
}

#[tokio::test]
async fn test_capability_executor_registry_missing() {
    let registry = CapabilityExecutorRegistry::new();

    let executor_opt = registry.get("missing-capability").await;
    assert!(executor_opt.is_none());
}

#[tokio::test]
async fn test_multiple_executors_in_registry() {
    let registry = CapabilityExecutorRegistry::new();

    let executor1 = FnCapabilityExecutor::new("executor1".to_string(), |params| {
        Box::pin(async move {
            Ok(serde_json::json!({"result": "executor1"}))
        })
    });

    let executor2 = FnCapabilityExecutor::new("executor2".to_string(), |params| {
        Box::pin(async move {
            Ok(serde_json::json!({"result": "executor2"}))
        })
    });

    registry.register("cap1".to_string(), Box::new(executor1)).unwrap();
    registry.register("cap2".to_string(), Box::new(executor2)).unwrap();

    let exec1 = registry.get("cap1").await.unwrap();
    let result1 = exec1.execute(HashMap::new()).await.unwrap();
    assert_eq!(result1["result"].as_str().unwrap(), "executor1");

    let exec2 = registry.get("cap2").await.unwrap();
    let result2 = exec2.execute(HashMap::new()).await.unwrap();
    assert_eq!(result2["result"].as_str().unwrap(), "executor2");
}

#[tokio::test]
async fn test_executor_error_handling() {
    let executor = FnCapabilityExecutor::new("error-executor".to_string(), |_params| {
        Box::pin(async move {
            Err("Test error".to_string().into())
        })
    });

    let result = executor.execute(HashMap::new()).await;
    assert!(result.is_err());
}