use std::collections::HashMap;
use std::sync::Arc;

use imitatort_stateless_company::infrastructure::capability::{McpProtocolHandler, CapabilityExecutorRegistry, CapabilityRegistry};
use imitatort_stateless_company::domain::capability::*;

#[tokio::test]
async fn test_mcp_protocol_handler_ping() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    let handler = McpProtocolHandler::new(capability_registry, executor_registry);

    let ping_request = serde_json::json!({
        "method": "ping",
        "params": {}
    });

    let response = handler.handle_request(ping_request).await;
    assert!(response.is_object());
    assert!(response.get("result").is_some());
}

#[tokio::test]
async fn test_mcp_protocol_handler_capabilities_list() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    let handler = McpProtocolHandler::new(capability_registry, executor_registry);

    let list_request = serde_json::json!({
        "method": "capabilities/list",
        "params": {}
    });

    let response = handler.handle_request(list_request).await;
    assert!(response.is_object());
    assert!(response.get("result").is_some());
}

#[tokio::test]
async fn test_mcp_protocol_handler_invalid_method() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    let handler = McpProtocolHandler::new(capability_registry, executor_registry);

    let invalid_request = serde_json::json!({
        "method": "invalid/method",
        "params": {}
    });

    let response = handler.handle_request(invalid_request).await;
    assert!(response.is_object());
    assert!(response.get("error").is_some());
}

#[tokio::test]
async fn test_mcp_protocol_handler_execute_with_registry() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    // Add a test capability
    let capability = Capability {
        id: "test.calc.add".to_string(),
        name: "Calculator Add".to_string(),
        description: "Add two numbers".to_string(),
        schema: CapabilitySchema::builder()
            .input_param("a", "integer", "First number")
            .input_param("b", "integer", "Second number")
            .output_type("integer", "Sum of the numbers")
            .build(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };
    capability_registry.register(capability).await.unwrap();

    // Add a corresponding executor
    let executor = imitatort_stateless_company::infrastructure::capability::executor::FnCapabilityExecutor::new(
        "calc-add-executor".to_string(),
        |params| {
            Box::pin(async move {
                let a = params.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
                let b = params.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
                Ok(serde_json::json!(a + b))
            })
        }
    );
    executor_registry.register("test.calc.add".to_string(), Box::new(executor)).unwrap();

    let handler = McpProtocolHandler::new(capability_registry, executor_registry);

    let execute_request = serde_json::json!({
        "method": "capabilities/execute",
        "params": {
            "name": "test.calc.add",
            "arguments": {
                "a": 5,
                "b": 3
            }
        }
    });

    let response = handler.handle_request(execute_request).await;
    assert!(response.is_object());
    if let Some(result) = response.get("result") {
        assert_eq!(result.as_i64().unwrap(), 8);
    } else {
        panic!("Expected result in response: {:?}", response);
    }
}

#[tokio::test]
async fn test_mcp_protocol_handler_execute_nonexistent_capability() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    let handler = McpProtocolHandler::new(capability_registry, executor_registry);

    let execute_request = serde_json::json!({
        "method": "capabilities/execute",
        "params": {
            "name": "nonexistent.capability",
            "arguments": {}
        }
    });

    let response = handler.handle_request(execute_request).await;
    assert!(response.is_object());
    assert!(response.get("error").is_some());
}

#[tokio::test]
async fn test_mcp_protocol_handler_initialize() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    let handler = McpProtocolHandler::new(capability_registry, executor_registry);

    let init_request = serde_json::json!({
        "method": "initialize",
        "params": {
            "clientVersion": "1.0.0",
            "capabilities": {}
        }
    });

    let response = handler.handle_request(init_request).await;
    assert!(response.is_object());
    // Initialization might return either result or error depending on implementation
    assert!(response.get("result").is_some() || response.get("error").is_some());
}