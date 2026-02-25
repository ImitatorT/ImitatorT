//! LLM 客户端
//!
//! 使用 async-openai 提供与 OpenAI API 的交互能力

use anyhow::{Context, Result};
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};
use async_openai::Client;

/// OpenAI 客户端
#[derive(Clone)]
pub struct OpenAIClient {
    client: Client<OpenAIConfig>,
    model: String,
}

impl OpenAIClient {
    /// 创建新的 OpenAI 客户端
    pub fn new_with_base_url(api_key: String, model: String, base_url: String) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();

        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);

        let client = Client::with_config(config);

        Self { client, model }
    }

    /// 调用聊天 API
    pub async fn chat(&self, messages: Vec<Message>) -> Result<String> {
        let messages: Vec<ChatCompletionRequestMessage> = messages
            .into_iter()
            .map(|msg| match msg.role.as_str() {
                "system" => ChatCompletionRequestSystemMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .map(ChatCompletionRequestMessage::System),
                _ => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .map(ChatCompletionRequestMessage::User),
            })
            .collect::<Result<Vec<_>, _>>()
            .context("构建消息失败")?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .build()
            .context("构建请求失败")?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .context("调用 LLM API 失败")?;

        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default();

        Ok(content)
    }

    /// 简单的文本补全
    pub async fn complete(&self, prompt: &str) -> Result<String> {
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        self.chat(messages).await
    }
}

/// 消息结构
#[derive(Clone, Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OpenAIClient::new_with_base_url(
            "test-key".to_string(),
            "gpt-4o-mini".to_string(),
            "https://api.openai.com/v1".to_string(),
        );

        assert_eq!(client.model, "gpt-4o-mini");
    }

    #[test]
    fn test_message_creation() {
        let msg = Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }
}
