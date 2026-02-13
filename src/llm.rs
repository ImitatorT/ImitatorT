use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::tool::{Tool, ToolCall};

#[derive(Clone)]
pub struct OpenAIClient {
    api_key: String,
    model: String,
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
        Self {
            api_key,
            model,
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

        let res: ChatResponse = self
            .http
            .post("https://api.openai.com/v1/chat/completions")
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

        let choice = res.choices.into_iter().next().context("empty response from openai")?;
        
        Ok((choice.message, choice.finish_reason))
    }

    /// Simple completion without tools (for backward compatibility)
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
