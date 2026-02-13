//! Protocol Server 模块单元测试

use imitatort_stateless_company::protocol::server::{AgentInfo, ApiResponse, A2AServerState};
use imitatort_stateless_company::core::messaging::MessageBus;
use std::sync::Arc;

#[test]
fn test_agent_info_creation() {
    let info = AgentInfo {
        id: "agent-001".to_string(),
        name: "Test Agent".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        capabilities: vec!["chat".to_string(), "task".to_string()],
        metadata: None,
    };

    assert_eq!(info.id, "agent-001");
    assert_eq!(info.endpoint, "http://localhost:8080");
}

#[test]
fn test_agent_info_clone() {
    let info = AgentInfo {
        id: "agent-001".to_string(),
        name: "Test Agent".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        capabilities: vec!["chat".to_string()],
        metadata: Some(serde_json::json!({"key": "value"})),
    };

    let cloned = info.clone();
    assert_eq!(info.id, cloned.id);
    assert_eq!(info.capabilities, cloned.capabilities);
}

#[test]
fn test_api_response_success() {
    let resp: ApiResponse<String> = ApiResponse::success("test data".to_string());
    assert!(resp.success);
    assert_eq!(resp.data, Some("test data".to_string()));
    assert!(resp.error.is_none());
}

#[test]
fn test_api_response_error() {
    let resp: ApiResponse<String> = ApiResponse::error("something went wrong");
    assert!(!resp.success);
    assert!(resp.data.is_none());
    assert_eq!(resp.error, Some("something went wrong".to_string()));
}

#[test]
fn test_api_response_error_from_string() {
    let error_msg = String::from("error from string");
    let resp: ApiResponse<String> = ApiResponse::error(error_msg);
    assert!(!resp.success);
    assert_eq!(resp.error, Some("error from string".to_string()));
}

#[test]
fn test_api_response_serialization() {
    let resp: ApiResponse<String> = ApiResponse::success("data".to_string());
    let json = serde_json::to_string(&resp).unwrap();
    
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("data"));
}

#[test]
fn test_api_response_deserialization() {
    let json = r#"{"success":true,"data":"test","error":null}"#;
    let resp: ApiResponse<String> = serde_json::from_str(json).unwrap();
    
    assert!(resp.success);
    assert_eq!(resp.data, Some("test".to_string()));
    assert!(resp.error.is_none());
}

#[test]
fn test_api_response_error_deserialization() {
    let json = r#"{"success":false,"data":null,"error":"failed"}"#;
    let resp: ApiResponse<String> = serde_json::from_str(json).unwrap();
    
    assert!(!resp.success);
    assert!(resp.data.is_none());
    assert_eq!(resp.error, Some("failed".to_string()));
}

#[tokio::test]
async fn test_server_state() {
    let bus = Arc::new(MessageBus::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    let state = Arc::new(A2AServerState::new(bus, addr));

    // 测试本地 Agent
    let local = AgentInfo {
        id: "local".to_string(),
        name: "Local".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        capabilities: vec![],
        metadata: None,
    };

    state.set_local_agent(local.clone()).await;
    assert_eq!(state.get_local_agent().await.unwrap().id, "local");

    // 测试远程 Agent
    let remote = AgentInfo {
        id: "remote".to_string(),
        name: "Remote".to_string(),
        endpoint: "http://remote:8080".to_string(),
        capabilities: vec![],
        metadata: None,
    };

    state.register_remote_agent(remote).await;
    assert_eq!(state.list_remote_agents().await.len(), 1);
    assert_eq!(
        state.get_remote_agent("remote").await.unwrap().name,
        "Remote"
    );
}

#[tokio::test]
async fn test_server_state_remove_remote_agent() {
    let bus = Arc::new(MessageBus::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    let state = Arc::new(A2AServerState::new(bus, addr));

    let remote = AgentInfo {
        id: "remote".to_string(),
        name: "Remote".to_string(),
        endpoint: "http://remote:8080".to_string(),
        capabilities: vec![],
        metadata: None,
    };

    state.register_remote_agent(remote).await;
    assert_eq!(state.list_remote_agents().await.len(), 1);

    state.remove_remote_agent("remote").await;
    assert_eq!(state.list_remote_agents().await.len(), 0);
    assert!(state.get_remote_agent("remote").await.is_none());
}

#[tokio::test]
async fn test_server_state_get_local_agent_empty() {
    let bus = Arc::new(MessageBus::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    let state = Arc::new(A2AServerState::new(bus, addr));

    // No local agent set
    assert!(state.get_local_agent().await.is_none());
}

#[test]
fn test_agent_info_debug() {
    let info = AgentInfo {
        id: "agent-001".to_string(),
        name: "Test".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        capabilities: vec![],
        metadata: None,
    };

    let debug_str = format!("{:?}", info);
    assert!(debug_str.contains("agent-001"));
}

#[test]
fn test_api_response_clone() {
    let resp: ApiResponse<String> = ApiResponse::success("data".to_string());
    let cloned = resp.clone();
    
    assert_eq!(resp.success, cloned.success);
    assert_eq!(resp.data, cloned.data);
    assert_eq!(resp.error, cloned.error);
}
