mod config;
mod llm;
mod matrix;
mod tool;

use anyhow::Result;
use clap::Parser;
use config::AppConfig;
use llm::{Message, OpenAIClient};
use matrix::MatrixClient;
use tool::{ToolCall, ToolRegistry};
use tracing::{debug, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cfg = AppConfig::parse();
    let matrix = MatrixClient::new(
        cfg.matrix_homeserver.clone(),
        cfg.matrix_access_token.clone(),
    );
    let llm = OpenAIClient::new(cfg.openai_api_key.clone(), cfg.openai_model.clone());

    info!("building stateless context");
    let context = matrix
        .latest_context(&cfg.matrix_room_id, cfg.context_limit)
        .await?;

    // Build messages with system prompt and context
    let mut messages = vec![
        Message {
            role: "system".to_string(),
            content: Some(cfg.system_prompt.clone()),
            tool_calls: None,
            tool_call_id: None,
        },
        Message {
            role: "user".to_string(),
            content: Some(format!(
                "上下文:\n{}\n\n请基于上下文分析并决定是否需要使用工具。如果需要执行命令或获取网页内容，请使用相应工具。",
                context
            )),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    // Get available tools
    let tools = ToolRegistry::get_tools();
    let _tools_enabled = !tools.is_empty();

    // First LLM call with tools
    let (mut message, finish_reason) = llm.chat(messages.clone(), Some(tools)).await?;
    debug!("first response finish_reason: {:?}", finish_reason);

    // Handle tool calls if any
    if let Some(tool_calls) = message.tool_calls.clone() {
        info!("llm requested {} tool call(s)", tool_calls.len());

        // Add assistant message with tool calls
        messages.push(Message {
            role: "assistant".to_string(),
            content: message.content,
            tool_calls: Some(tool_calls.clone()),
            tool_call_id: None,
        });

        // Execute each tool call and add results
        for tool_call in tool_calls {
            let result = execute_tool(&tool_call).await;
            messages.push(Message {
                role: "tool".to_string(),
                content: Some(result),
                tool_calls: None,
                tool_call_id: Some(tool_call.id.clone()),
            });
        }

        // Second LLM call with tool results
        let (final_message, _) = llm.chat(messages, None).await?;
        message = final_message;
    }

    // Send final response to Matrix
    let answer = message.content.unwrap_or_else(|| "(无响应内容)".to_string());
    matrix
        .send_text_message(&cfg.matrix_room_id, &answer)
        .await?;

    info!("stateless cycle completed");
    Ok(())
}

/// Execute a tool call and return formatted result
async fn execute_tool(tool_call: &ToolCall) -> String {
    info!("executing tool: {}", tool_call.function.name);
    
    match ToolRegistry::execute(tool_call).await {
        Ok(result) => {
            info!("tool executed successfully");
            result
        }
        Err(e) => {
            let error_msg = format!("工具执行错误: {}", e);
            tracing::error!("{}", error_msg);
            error_msg
        }
    }
}
