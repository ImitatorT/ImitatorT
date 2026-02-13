use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::tool::{Tool, ToolCall};

#[derive(Clone)]
pub struct OpenAIClient {
    api_key: String,
    model: String,
    base_url: String,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
    finish_reason: Option<String>,
}

impl OpenAIClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self::new_with_base_url(api_key, model, "https://api.openai.com/v1".to_string())
    }

    pub fn new_with_base_url(api_key: String, model: String, base_url: String) -> Self {
        // 确保 base_url 不以斜杠结尾，避免路径拼接问题
        let base_url = base_url.trim_end_matches('/').to_string();
        Self {
            api_key,
            model,
            base_url,
            http: reqwest::Client::new(),
        }
    }

    /// Call LLM with optional tools
    pub async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Tool>>,
    ) -> Result<(Message, Option<String>)> {
        let req = ChatRequest {
            model: self.model.clone(),
            messages,
            tools,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let res: ChatResponse = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .context("failed to call openai")?
            .error_for_status()
            .context("openai returned non-success status")?
            .json()
            .await
            .context("failed to parse openai response")?;

        let choice = res
            .choices
            .into_iter()
            .next()
            .context("empty response from openai")?;

        Ok((choice.message, choice.finish_reason))
    }

    /// Simple completion without tools (for backward compatibility)
    #[allow(dead_code)]
    pub async fn complete(&self, system_prompt: &str, context: &str, task: &str) -> Result<String> {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: Some(system_prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "user".to_string(),
                content: Some(format!("上下文:\n{}\n\n任务:\n{}", context, task)),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let (message, _) = self.chat(messages, None).await?;

        Ok(message.content.unwrap_or_else(|| "(空响应)".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{Function, Tool};
    use serde_json::json;

    #[test]
    fn test_openai_client_new() {
        let client = OpenAIClient::new("test-api-key".to_string(), "gpt-4o-mini".to_string());
        assert_eq!(client.api_key, "test-api-key");
        assert_eq!(client.model, "gpt-4o-mini");
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
        use crate::tool::{FunctionCall, ToolCall};

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
    fn test_chat_request_serialization() {
        let tool = Tool {
            r#type: "function".to_string(),
            function: Function {
                name: "test".to_string(),
                description: "Test function".to_string(),
                parameters: json!({"type": "object"}),
            },
        };

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
}
