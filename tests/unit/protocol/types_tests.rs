//! Protocol Types 模块单元测试

use imitatort_stateless_company::protocol::types::{
    create_default_agent_card, AgentCard, AgentEndpoints, A2AAgent, A2AMessage, Artifact,
    MessageContent, Skill, TaskRequest, TaskResult, TextMessageHandler,
};
use std::collections::HashMap;

#[test]
fn test_agent_card_creation() {
    let card = AgentCard {
        id: "agent-1".to_string(),
        name: "Test Agent".to_string(),
        description: "A test agent".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["chat".to_string()],
        endpoints: AgentEndpoints {
            a2a_endpoint: "/a2a/agent-1".to_string(),
            webhook_endpoint: Some("http://localhost/webhook".to_string()),
        },
        skills: vec![Skill {
            name: "conversation".to_string(),
            description: "Chat skill".to_string(),
            parameters: serde_json::json!({}),
        }],
    };

    assert_eq!(card.id, "agent-1");
    assert_eq!(card.name, "Test Agent");
}

#[tokio::test]
async fn test_default_agent_card_creation() {
    let card = create_default_agent_card("agent-1", "Test Agent");
    assert_eq!(card.id, "agent-1");
    assert_eq!(card.name, "Test Agent");
    assert!(!card.capabilities.is_empty());
    assert!(!card.skills.is_empty());
}

#[tokio::test]
async fn test_agent_register_peer() {
    let card = create_default_agent_card("agent-1", "Agent 1");
    let agent = A2AAgent::new(card);

    let peer_card = create_default_agent_card("agent-2", "Agent 2");
    agent.register_peer(peer_card.clone()).await;

    let peers = agent.list_peers().await;
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].id, "agent-2");
}

#[tokio::test]
async fn test_agent_get_peer() {
    let card = create_default_agent_card("agent-1", "Agent 1");
    let agent = A2AAgent::new(card);

    let peer_card = create_default_agent_card("agent-2", "Agent 2");
    agent.register_peer(peer_card.clone()).await;

    let peer = agent.get_peer("agent-2").await;
    assert!(peer.is_some());
    assert_eq!(peer.unwrap().name, "Agent 2");

    // 获取不存在的 peer
    let not_found = agent.get_peer("non-existent").await;
    assert!(not_found.is_none());
}

#[test]
fn test_message_creation() {
    let msg = A2AMessage {
        id: "msg-1".to_string(),
        sender: "agent-a".to_string(),
        receiver: "agent-b".to_string(),
        content: MessageContent::Text {
            text: "Hello".to_string(),
        },
        timestamp: chrono::Utc::now().timestamp(),
        metadata: HashMap::new(),
    };

    match &msg.content {
        MessageContent::Text { text } => assert_eq!(text, "Hello"),
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_task_request_creation() {
    let task = TaskRequest {
        id: "task-1".to_string(),
        description: "Test task".to_string(),
        parameters: Some(serde_json::json!({"key": "value"})),
        context: vec!["context-1".to_string()],
    };

    assert_eq!(task.id, "task-1");
    assert_eq!(task.description, "Test task");
}

#[test]
fn test_task_result_serialization() {
    let result = TaskResult::Success {
        output: "Test output".to_string(),
        artifacts: vec![Artifact {
            name: "test.txt".to_string(),
            content_type: "text/plain".to_string(),
            content: "Hello World".to_string(),
        }],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("Test output"));
    assert!(json.contains("test.txt"));
}

#[test]
fn test_task_result_error() {
    let result = TaskResult::Error {
        code: "ERROR_001".to_string(),
        message: "Something went wrong".to_string(),
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("ERROR_001"));
    assert!(json.contains("Something went wrong"));
}

#[test]
fn test_task_result_pending() {
    let result = TaskResult::Pending;
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("Pending"));
}

#[test]
fn test_message_content_types() {
    let text = MessageContent::Text {
        text: "Hello".to_string(),
    };
    let task = MessageContent::Task {
        task: TaskRequest {
            id: "task-1".to_string(),
            description: "Do something".to_string(),
            parameters: None,
            context: vec![],
        },
    };
    let response = MessageContent::TaskResponse {
        task_id: "task-1".to_string(),
        result: TaskResult::Pending,
    };
    let status = MessageContent::Status {
        status: "running".to_string(),
        message: Some("Processing".to_string()),
    };

    // Test serialization
    let _ = serde_json::to_string(&text).unwrap();
    let _ = serde_json::to_string(&task).unwrap();
    let _ = serde_json::to_string(&response).unwrap();
    let _ = serde_json::to_string(&status).unwrap();
}

#[test]
fn test_artifact_creation() {
    let artifact = Artifact {
        name: "result.txt".to_string(),
        content_type: "text/plain".to_string(),
        content: "Result content".to_string(),
    };

    assert_eq!(artifact.name, "result.txt");
    assert_eq!(artifact.content_type, "text/plain");
}

#[test]
fn test_skill_creation() {
    let skill = Skill {
        name: "data_analysis".to_string(),
        description: "Analyze data".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "data": { "type": "string" }
            }
        }),
    };

    assert_eq!(skill.name, "data_analysis");
}

#[test]
fn test_agent_endpoints_creation() {
    let endpoints = AgentEndpoints {
        a2a_endpoint: "/a2a/agent-1".to_string(),
        webhook_endpoint: None,
    };

    assert_eq!(endpoints.a2a_endpoint, "/a2a/agent-1");
    assert!(endpoints.webhook_endpoint.is_none());
}

#[test]
fn test_a2a_message_clone() {
    let msg = A2AMessage {
        id: "msg-1".to_string(),
        sender: "agent-a".to_string(),
        receiver: "agent-b".to_string(),
        content: MessageContent::Text {
            text: "Hello".to_string(),
        },
        timestamp: 1234567890,
        metadata: HashMap::new(),
    };

    let cloned = msg.clone();
    assert_eq!(msg.id, cloned.id);
    assert_eq!(msg.timestamp, cloned.timestamp);
}

#[test]
fn test_task_request_clone() {
    let task = TaskRequest {
        id: "task-1".to_string(),
        description: "Test".to_string(),
        parameters: None,
        context: vec!["ctx-1".to_string()],
    };

    let cloned = task.clone();
    assert_eq!(task.id, cloned.id);
    assert_eq!(task.context, cloned.context);
}
