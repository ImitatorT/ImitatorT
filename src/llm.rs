use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

impl OpenAIClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http: reqwest::Client::new(),
        }
    }

    pub async fn complete(&self, system_prompt: &str, context: &str, task: &str) -> Result<String> {
        let req = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: format!("上下文:\n{}\n\n任务:\n{}", context, task),
                },
            ],
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

        let content = res
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_else(|| "(空响应)".to_string());

        Ok(content)
    }
}
