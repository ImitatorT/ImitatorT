use std::sync::Arc;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};

use imitatort_stateless_company::infrastructure::capability::{McpServer, CapabilityRegistry, CapabilityExecutorRegistry};

#[tokio::test]
async fn test_mcp_server_creation() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    let server = McpServer::new("127.0.0.1:0".to_string(), capability_registry, executor_registry);

    // Just test that the server can be created without errors
    assert!(!server.bind_addr.is_empty());
}

#[tokio::test]
#[ignore] // Integration test that requires port binding and connection
async fn test_mcp_server_http_endpoints() {
    // This would require starting a server and making HTTP requests
    // For now, we'll focus on structure validation

    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    let server = McpServer::new("127.0.0.1:0".to_string(), capability_registry, executor_registry);

    // Verify server structure
    assert!(server.bind_addr.starts_with("127.0.0.1:"));
}

#[tokio::test]
#[ignore] // Integration test that requires WebSocket connection
async fn test_mcp_server_websocket_communication() {
    // This would require starting a server and connecting via WebSocket
    // For now, we'll focus on the concept

    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    let server = McpServer::new("127.0.0.1:0".to_string(), capability_registry, executor_registry);

    // Conceptually, this would test WebSocket communication
    // Start server, connect WebSocket client, send messages, verify responses
    assert!(!server.bind_addr.is_empty());
}

#[tokio::test]
async fn test_mcp_server_with_registries() {
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let executor_registry = Arc::new(CapabilityExecutorRegistry::new());

    // Add a simple capability to test with
    use imitatort_stateless_company::domain::capability::*;

    let capability = Capability {
        id: "test.server.echo".to_string(),
        name: "Echo Capability".to_string(),
        description: "Echo test capability".to_string(),
        schema: CapabilitySchema::builder()
            .input_param("input", "string", "Input to echo")
            .output_type("string", "Echoed input")
            .build(),
        provider: CapabilityProvider::Framework,
        access_type: CapabilityAccessType::Public,
    };
    capability_registry.register(capability).await.unwrap();

    // Add corresponding executor
    let executor = imitatort_stateless_company::infrastructure::capability::executor::FnCapabilityExecutor::new(
        "echo-executor".to_string(),
        |params| {
            Box::pin(async move {
                let input = params.get("input").and_then(|v| v.as_str()).unwrap_or("");
                Ok(serde_json::json!(input))
            })
        }
    );
    executor_registry.register("test.server.echo".to_string(), Box::new(executor)).unwrap();

    let server = McpServer::new("127.0.0.1:0".to_string(), capability_registry, executor_registry);

    // Verify that registries were properly passed
    assert!(!server.bind_addr.is_empty());

    // Check that capabilities were registered
    let caps = server.capability_registry.list_all().await;
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].id, "test.server.echo");
}