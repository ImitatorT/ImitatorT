//! LLM 模块单元测试

use imitatort_stateless_company::infrastructure::llm::{Message, OpenAIClient};
use imitatort_stateless_company::application::tool::{Tool, Function};
use serde_json::json;

#[test]
fn test_openai_client_new() {
    let client = OpenAIClient::new("test-api-key".to_string(), "gpt-4o-mini".to_string());
    // Client is created successfully
}

#[test]
fn test_openai_client_new_with_base_url() {
    let client = OpenAIClient::new_with_base_url(
        "test-api-key".to_string(),
        "gpt-4o-mini".to_string(),
        "https://api.example.com/v1".to_string(),
    );
    // Client is created successfully
}

#[test]
fn test_message_serialization() {
    let msg = Message {
        role: "user".to_string(),
        content: Some("Hello".to_string()),
        tool_calls: None,
        tool_call_id: None,
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(json_str.contains("user"));
    assert!(json_str.contains("Hello"));
}

#[test]
fn test_message_deserialization() {
    let json_str = r#"{"role": "assistant", "content": "Hi there"}"#;
    let msg: Message = serde_json::from_str(json_str).unwrap();
    assert_eq!(msg.role, "assistant");
    assert_eq!(msg.content, Some("Hi there".to_string()));
}

#[test]
fn test_message_with_tool_calls() {
    use imitatort_stateless_company::application::tool::{ToolCall, FunctionCall};

    let tool_call = ToolCall {
        id: "call_123".to_string(),
        r#type: "function".to_string(),
        function: FunctionCall {
            name: "execute_command".to_string(),
            arguments: r#"{"command": "ls"}"#.to_string(),
        },
    };

    let msg = Message {
        role: "assistant".to_string(),
        content: None,
        tool_calls: Some(vec![tool_call]),
        tool_call_id: None,
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(json_str.contains("tool_calls"));
    assert!(json_str.contains("execute_command"));
}

#[test]
fn test_message_with_tool_response() {
    let msg = Message {
        role: "tool".to_string(),
        content: Some("Command output".to_string()),
        tool_calls: None,
        tool_call_id: Some("call_123".to_string()),
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(json_str.contains("tool"));
    assert!(json_str.contains("tool_call_id"));
}

#[test]
fn test_chat_request_serialization() {
    let tool = Tool {
        r#type: "function".to_string(),
        function: Function {
            name: "test".to_string(),
            description: "Test function".to_string(),
            parameters: json!({"type": "object"}),
        },
    };

    #[derive(serde::Serialize)]
    struct ChatRequest {
        model: String,
        messages: Vec<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tools: Option<Vec<Tool>>,
    }

    let req = ChatRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }],
        tools: Some(vec![tool]),
    };

    let json_str = serde_json::to_string(&req).unwrap();
    assert!(json_str.contains("gpt-4o-mini"));
    assert!(json_str.contains("tools"));
}

#[test]
fn test_chat_request_without_tools() {
    #[derive(serde::Serialize)]
    struct ChatRequest {
        model: String,
        messages: Vec<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tools: Option<Vec<Tool>>,
    }

    let req = ChatRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }],
        tools: None,
    };

    let json_str = serde_json::to_string(&req).unwrap();
    // When tools is None, it should be skipped
    assert!(!json_str.contains("tools"));
}

#[test]
fn test_chat_response_deserialization() {
    #[derive(serde::Deserialize)]
    struct ChatResponse {
        choices: Vec<Choice>,
    }

    #[derive(serde::Deserialize)]
    struct Choice {
        message: Message,
        finish_reason: Option<String>,
    }

    let json_str = r#"{
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                },
                "finish_reason": "stop"
            }
        ]
    }"#;

    let resp: ChatResponse = serde_json::from_str(json_str).unwrap();
    assert_eq!(resp.choices.len(), 1);
    assert_eq!(resp.choices[0].message.role, "assistant");
    assert_eq!(resp.choices[0].message.content, Some("Hello!".to_string()));
    assert_eq!(resp.choices[0].finish_reason, Some("stop".to_string()));
}

#[test]
fn test_message_clone() {
    let msg = Message {
        role: "user".to_string(),
        content: Some("Hello".to_string()),
        tool_calls: None,
        tool_call_id: None,
    };
    let cloned = msg.clone();
    assert_eq!(msg.role, cloned.role);
    assert_eq!(msg.content, cloned.content);
}

#[test]
fn test_message_debug() {
    let msg = Message {
        role: "assistant".to_string(),
        content: Some("Hi".to_string()),
        tool_calls: None,
        tool_call_id: None,
    };
    let debug_str = format!("{:?}", msg);
    assert!(debug_str.contains("assistant"));
}
