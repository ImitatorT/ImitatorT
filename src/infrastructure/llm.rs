//! LLM 客户端
//!
//! 使用 async-openai 提供与 OpenAI API 的交互能力
//! 支持 Tool Calling (Function Calling)

use anyhow::{Context, Result};
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTool, ChatCompletionToolChoiceOption,
    ChatCompletionTools, CreateChatCompletionRequestArgs, ToolChoiceOptions,
};
use async_openai::types::chat::{FunctionCall, FunctionObject};
use async_openai::Client;
use serde_json::Value;

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
        let messages = vec![Message::user(prompt)];

        self.chat(messages).await
    }

    /// 调用带 Tool 支持的聊天 API
    ///
    /// # Arguments
    /// * `messages` - 消息列表
    /// * `tools` - 可用工具列表
    ///
    /// # Returns
    /// * `ToolResponse` - 包含 assistant 的回复或 tool 调用请求
    pub async fn chat_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<ToolResponse> {
        let request_messages = self.build_request_messages(messages)?;
        let chat_tools = self.build_chat_tools(tools)?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(request_messages)
            .tools(chat_tools)
            .tool_choice(ChatCompletionToolChoiceOption::Mode(
                ToolChoiceOptions::Auto,
            ))
            .build()
            .context("构建请求失败")?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .context("调用 LLM API 失败")?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .context("API 返回空响应")?;

        let message = choice.message;

        // 检查是否有 tool_calls
        if let Some(tool_calls) = message.tool_calls {
            let calls: Vec<ToolCall> = tool_calls
                .into_iter()
                .filter_map(|tc| match tc {
                    ChatCompletionMessageToolCalls::Function(func_call) => {
                        let args = serde_json::from_str(&func_call.function.arguments)
                            .unwrap_or(Value::Null);
                        Some(ToolCall {
                            id: func_call.id,
                            name: func_call.function.name,
                            arguments: args,
                        })
                    }
                    _ => None,
                })
                .collect();

            if !calls.is_empty() {
                return Ok(ToolResponse::ToolCalls {
                    content: message.content.unwrap_or_default(),
                    tool_calls: calls,
                });
            }
        }

        // 返回普通文本回复
        Ok(ToolResponse::Message(message.content.unwrap_or_default()))
    }

    /// 构建请求消息
    fn build_request_messages(
        &self,
        messages: Vec<Message>,
    ) -> Result<Vec<ChatCompletionRequestMessage>> {
        messages
            .into_iter()
            .map(|msg| match msg.role.as_str() {
                "system" => ChatCompletionRequestSystemMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .map(ChatCompletionRequestMessage::System)
                    .context("构建系统消息失败"),
                "user" => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .map(ChatCompletionRequestMessage::User)
                    .context("构建用户消息失败"),
                "assistant" => {
                    let mut args = ChatCompletionRequestAssistantMessageArgs::default();
                    args.content(msg.content);

                    // 如果有 tool_calls，需要转换格式
                    if let Some(calls) = msg.tool_calls {
                        let tool_calls: Vec<ChatCompletionMessageToolCalls> = calls
                            .into_iter()
                            .map(|c| {
                                ChatCompletionMessageToolCalls::Function(
                                    ChatCompletionMessageToolCall {
                                        id: c.id,
                                        function: FunctionCall {
                                            name: c.name,
                                            arguments: c.arguments.to_string(),
                                        },
                                    },
                                )
                            })
                            .collect();
                        args.tool_calls(tool_calls);
                    }

                    args.build()
                        .map(ChatCompletionRequestMessage::Assistant)
                        .context("构建助手消息失败")
                }
                "tool" => ChatCompletionRequestToolMessageArgs::default()
                    .content(msg.content)
                    .tool_call_id(msg.tool_call_id.unwrap_or_default())
                    .build()
                    .map(ChatCompletionRequestMessage::Tool)
                    .context("构建工具消息失败"),
                _ => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .map(ChatCompletionRequestMessage::User)
                    .context("构建用户消息失败"),
            })
            .collect::<Result<Vec<_>>>()
    }

    /// 构建 ChatCompletionTools 列表
    fn build_chat_tools(&self, tools: Vec<Tool>) -> Result<Vec<ChatCompletionTools>> {
        Ok(tools
            .into_iter()
            .map(|tool| {
                ChatCompletionTools::Function(ChatCompletionTool {
                    function: FunctionObject {
                        name: tool.id,
                        description: Some(tool.description),
                        parameters: Some(tool.parameters),
                        strict: None,
                    },
                })
            })
            .collect())
    }
}

/// Tool 调用响应
#[derive(Clone, Debug)]
pub enum ToolResponse {
    /// 普通文本消息
    Message(String),
    /// Tool 调用请求
    ToolCalls {
        /// Assistant 的伴随文本（可能为空）
        content: String,
        /// Tool 调用列表
        tool_calls: Vec<ToolCall>,
    },
}

impl ToolResponse {
    /// 检查是否是 tool 调用
    pub fn is_tool_calls(&self) -> bool {
        matches!(self, Self::ToolCalls { .. })
    }

    /// 获取文本内容
    pub fn content(&self) -> &str {
        match self {
            Self::Message(c) => c,
            Self::ToolCalls { content, .. } => content,
        }
    }

    /// 获取 tool_calls（如果是 ToolCalls 变体）
    pub fn tool_calls(&self) -> Option<&Vec<ToolCall>> {
        match self {
            Self::ToolCalls { tool_calls, .. } => Some(tool_calls),
            _ => None,
        }
    }
}

/// 消息结构
#[derive(Clone, Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
    /// Tool calls (用于 assistant 消息)
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool call ID (用于 tool 消息)
    pub tool_call_id: Option<String>,
}

impl Message {
    /// 创建系统消息
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// 创建用户消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// 创建 assistant 消息
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// 创建带 tool_calls 的 assistant 消息
    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// 创建 tool 消息
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Tool 定义
#[derive(Clone, Debug)]
pub struct Tool {
    /// 工具 ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 参数 JSON Schema
    pub parameters: Value,
}

impl Tool {
    /// 从 domain Tool 创建
    pub fn from_domain_tool(tool: &crate::domain::tool::Tool) -> Self {
        Self {
            id: tool.id.clone(),
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.parameters.clone(),
        }
    }
}

/// Tool 调用
#[derive(Clone, Debug)]
pub struct ToolCall {
    /// 调用 ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 调用参数 (JSON)
    pub arguments: Value,
}
