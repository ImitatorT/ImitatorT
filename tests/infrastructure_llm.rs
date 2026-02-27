//! LLM 客户端测试

use imitatort::infrastructure::llm::{Message, OpenAIClient};

#[test]
fn test_client_creation() {
    let _client = OpenAIClient::new_with_base_url(
        "test-key".to_string(),
        "gpt-4o-mini".to_string(),
        "https://api.openai.com/v1".to_string(),
    );

    // Client created successfully
}

#[test]
fn test_message_creation() {
    let msg = Message {
        role: "user".to_string(),
        content: "Hello".to_string(),
        tool_calls: None,
        tool_call_id: None,
    };

    assert_eq!(msg.role, "user");
    assert_eq!(msg.content, "Hello");
}
