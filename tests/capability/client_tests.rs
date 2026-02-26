use imitatort_stateless_company::infrastructure::capability::mcp_client::{McpClient, McpTransport};

#[tokio::test]
async fn test_mcp_client_creation() {
    // Test HTTP client creation
    let http_client = McpClient::new(McpTransport::Http {
        url: "http://localhost:8080".to_string(),
    });

    assert!(matches!(http_client.transport, McpTransport::Http { .. }));

    // Test WebSocket client creation
    let ws_client = McpClient::new(McpTransport::WebSocket {
        url: "ws://localhost:8080".to_string(),
    });

    assert!(matches!(ws_client.transport, McpTransport::WebSocket { .. }));
}

#[tokio::test]
#[ignore] // Ignore these tests that require external services
async fn test_mcp_client_ping() {
    // This test would require a running MCP server
    // We'll create a mock server for testing

    // For now, we'll just test the structure
    let client = McpClient::new(McpTransport::Http {
        url: "http://localhost:8080".to_string(),
    });

    // This would normally connect to a server and test ping
    // Since we don't have a server, we'll just verify the client structure
    assert!(matches!(client.transport, McpTransport::Http { .. }));
}

#[tokio::test]
#[ignore] // Ignore these tests that require external services
async fn test_mcp_client_capabilities_list() {
    let client = McpClient::new(McpTransport::Http {
        url: "http://localhost:8080".to_string(),
    });

    // This would normally connect to a server and list capabilities
    assert!(matches!(client.transport, McpTransport::Http { .. }));
}

#[tokio::test]
async fn test_mcp_transport_variants() {
    let http_transport = McpTransport::Http {
        url: "http://example.com".to_string(),
    };

    let ws_transport = McpTransport::WebSocket {
        url: "ws://example.com".to_string(),
    };

    let sse_transport = McpTransport::Sse {
        url: "http://example.com".to_string(),
    };

    let stdio_transport = McpTransport::Stdio;

    let cmd_transport = McpTransport::Command {
        command: "my-command".to_string(),
    };

    match http_transport {
        McpTransport::Http { url } => assert_eq!(url, "http://example.com"),
        _ => panic!("Expected Http variant"),
    }

    match ws_transport {
        McpTransport::WebSocket { url } => assert_eq!(url, "ws://example.com"),
        _ => panic!("Expected WebSocket variant"),
    }

    match sse_transport {
        McpTransport::Sse { url } => assert_eq!(url, "http://example.com"),
        _ => panic!("Expected Sse variant"),
    }

    match stdio_transport {
        McpTransport::Stdio => (), // Expected
        _ => panic!("Expected Stdio variant"),
    }

    match cmd_transport {
        McpTransport::Command { command } => assert_eq!(command, "my-command"),
        _ => panic!("Expected Command variant"),
    }
}

#[tokio::test]
async fn test_mcp_client_transport_methods() {
    // Test that we can properly access transport information
    let client = McpClient::new(McpTransport::Http {
        url: "http://localhost:8080".to_string(),
    });

    // Verify the client was created correctly
    match &client.transport {
        McpTransport::Http { url } => {
            assert!(url.starts_with("http://"));
        },
        _ => panic!("Expected Http transport"),
    }
}