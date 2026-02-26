use std::collections::HashMap;
use std::sync::Arc;

use imitatort_stateless_company::{
    core::capability::{CapabilityRegistry},
    infrastructure::capability::{
        executor::{CapabilityExecutorRegistry, FnCapabilityExecutor},
        McpProtocolHandler
    },
    domain::capability::*,
};

#[tokio::test]
async fn test_full_capability_lifecycle() {
    // Create registries
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    // Define a capability
    let capability = Capability {
        id: "math.calculator.add".to_string(),
        name: "Math Calculator Add".to_string(),
        description: "Adds two numbers together".to_string(),
        schema: CapabilitySchema::builder()
            .input_param("first", "number", "First number to add")
            .input_param("second", "number", "Second number to add")
            .output_type("number", "Sum of the two numbers")
            .build(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    // Register the capability
    capability_registry.register(capability).await.unwrap();

    // Create and register an executor for the capability
    let executor = FnCapabilityExecutor::new("calculator-add-executor".to_string(), |params| {
        Box::pin(async move {
            let first = params.get("first").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let second = params.get("second").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(serde_json::json!(first + second))
        })
    });

    executor_registry.register("math.calculator.add".to_string(), Box::new(executor)).unwrap();

    // Create protocol handler
    let handler = McpProtocolHandler::new(
        capability_registry.clone(),
        executor_registry.clone()
    );

    // Test listing capabilities
    let list_request = serde_json::json!({
        "method": "capabilities/list",
        "params": {}
    });

    let list_response = handler.handle_request(list_request).await;
    assert!(list_response.get("result").is_some());

    // Test executing the capability
    let execute_request = serde_json::json!({
        "method": "capabilities/execute",
        "params": {
            "name": "math.calculator.add",
            "arguments": {
                "first": 10.5,
                "second": 5.3
            }
        }
    });

    let execute_response = handler.handle_request(execute_request).await;

    if let Some(result) = execute_response.get("result") {
        let sum = result.as_f64().unwrap();
        assert!((sum - 15.8).abs() < 0.001); // Account for floating point precision
    } else {
        panic!("Expected result in execute response: {:?}", execute_response);
    }
}

#[tokio::test]
async fn test_capability_with_private_access() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    // Create a private capability
    let private_capability = Capability {
        id: "private.secret".to_string(),
        name: "Secret Capability".to_string(),
        description: "A private capability".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Private,
    };

    capability_registry.register(private_capability).await.unwrap();

    // Create protocol handler
    let handler = McpProtocolHandler::new(
        capability_registry,
        executor_registry
    );

    // Private capabilities should still be listable in this basic implementation
    // (Access control would be handled at a higher level with skills)
    let list_request = serde_json::json!({
        "method": "capabilities/list",
        "params": {}
    });

    let list_response = handler.handle_request(list_request).await;
    assert!(list_response.get("result").is_some());
}

#[tokio::test]
async fn test_multiple_capabilities_same_category() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    // Create multiple capabilities in the same category
    let cap1 = Capability {
        id: "math.ops.add".to_string(),
        name: "Add Operation".to_string(),
        description: "Addition operation".to_string(),
        schema: CapabilitySchema::builder()
            .input_param("a", "number", "First operand")
            .input_param("b", "number", "Second operand")
            .output_type("number", "Result")
            .build(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    let cap2 = Capability {
        id: "math.ops.multiply".to_string(),
        name: "Multiply Operation".to_string(),
        description: "Multiplication operation".to_string(),
        schema: CapabilitySchema::builder()
            .input_param("a", "number", "First operand")
            .input_param("b", "number", "Second operand")
            .output_type("number", "Result")
            .build(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    capability_registry.register(cap1).await.unwrap();
    capability_registry.register(cap2).await.unwrap();

    // Register executors
    let add_executor = FnCapabilityExecutor::new("add-executor".to_string(), |params| {
        Box::pin(async move {
            let a = params.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = params.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(serde_json::json!(a + b))
        })
    });
    executor_registry.register("math.ops.add".to_string(), Box::new(add_executor)).unwrap();

    let mul_executor = FnCapabilityExecutor::new("mul-executor".to_string(), |params| {
        Box::pin(async move {
            let a = params.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = params.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(serde_json::json!(a * b))
        })
    });
    executor_registry.register("math.ops.multiply".to_string(), Box::new(mul_executor)).unwrap();

    let handler = McpProtocolHandler::new(
        capability_registry,
        executor_registry
    );

    // Test addition
    let add_request = serde_json::json!({
        "method": "capabilities/execute",
        "params": {
            "name": "math.ops.add",
            "arguments": {
                "a": 4,
                "b": 6
            }
        }
    });

    let add_result = handler.handle_request(add_request).await;
    if let Some(result) = add_result.get("result") {
        assert_eq!(result.as_f64().unwrap(), 10.0);
    } else {
        panic!("Expected result in add response: {:?}", add_result);
    }

    // Test multiplication
    let mul_request = serde_json::json!({
        "method": "capabilities/execute",
        "params": {
            "name": "math.ops.multiply",
            "arguments": {
                "a": 3,
                "b": 7
            }
        }
    });

    let mul_result = handler.handle_request(mul_request).await;
    if let Some(result) = mul_result.get("result") {
        assert_eq!(result.as_f64().unwrap(), 21.0);
    } else {
        panic!("Expected result in multiply response: {:?}", mul_result);
    }
}

#[tokio::test]
async fn test_capability_error_handling() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    // Create a capability with an executor that returns an error
    let capability = Capability {
        id: "error.test".to_string(),
        name: "Error Test".to_string(),
        description: "A capability that returns an error".to_string(),
        schema: CapabilitySchema::default(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };

    capability_registry.register(capability).await.unwrap();

    let error_executor = FnCapabilityExecutor::new("error-executor".to_string(), |_params| {
        Box::pin(async move {
            Err("Test error occurred".to_string().into())
        })
    });
    executor_registry.register("error.test".to_string(), Box::new(error_executor)).unwrap();

    let handler = McpProtocolHandler::new(
        capability_registry,
        executor_registry
    );

    let error_request = serde_json::json!({
        "method": "capabilities/execute",
        "params": {
            "name": "error.test",
            "arguments": {}
        }
    });

    let error_response = handler.handle_request(error_request).await;
    assert!(error_response.get("error").is_some());
}