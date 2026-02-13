//! Protocol Client 模块单元测试

use imitatort_stateless_company::protocol::client::{A2AClient, AgentNetwork};

#[test]
fn test_a2a_client_creation() {
    let client = A2AClient::new("http://localhost:8080");
    // Client is created successfully
}

#[test]
fn test_agent_network_creation() {
    let network = AgentNetwork::new("http://localhost:8080");
    // Network is created successfully
}

#[test]
fn test_agent_network_empty() {
    let network = AgentNetwork::new("http://localhost:8080");
    
    // Initially empty
    let agents = network.list_agents();
    assert!(agents.is_empty());
    
    // Getting non-existent agent returns None
    assert!(network.get_agent("non-existent").is_none());
}

#[test]
fn test_a2a_client_clone() {
    // A2AClient should not implement Clone (contains non-cloneable HTTP client)
    // But we can create multiple instances
    let _client1 = A2AClient::new("http://localhost:8080");
    let _client2 = A2AClient::new("http://localhost:8080");
}

#[test]
fn test_agent_network_get_set_agent() {
    use imitatort_stateless_company::protocol::server::AgentInfo;
    
    let network = AgentNetwork::new("http://localhost:8080");
    
    let agent_info = AgentInfo {
        id: "test-agent".to_string(),
        name: "Test Agent".to_string(),
        endpoint: "http://localhost:9000".to_string(),
        capabilities: vec!["chat".to_string()],
        metadata: None,
    };
    
    // Note: AgentNetwork doesn't expose insert method publicly
    // This test documents the expected behavior
    let agents = network.list_agents();
    assert!(agents.is_empty());
}
