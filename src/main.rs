mod config;
mod llm;
mod logger;
mod matrix;
mod tool;

use anyhow::Result;
use clap::Parser;
use config::AppConfig;
use llm::{Message, OpenAIClient};
use logger::{LogConfig, RequestContext, Sanitizer, Timer};
use matrix::MatrixClient;
use tool::{ToolCall, ToolRegistry};
use tracing::{debug, error, info, info_span, Instrument};

#[tokio::main]
async fn main() -> Result<()> {
    // 从环境变量初始化日志配置
    let log_format = std::env::var("LOG_FORMAT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(logger::LogFormat::Pretty);

    let log_config = LogConfig {
        format: log_format,
        enable_color: true,
        show_target: true,
        show_thread_id: false,
        show_thread_names: false,
        show_file: true,
        show_time: true,
        time_format: logger::TimeFormat::Rfc3339,
        request_id_header: "request_id".to_string(),
    };
    logger::init(log_config);

    // 创建请求上下文，用于追踪本次执行
    let request_ctx = RequestContext::new().with_metadata("version", env!("CARGO_PKG_VERSION"));

    // 在 span 中执行主逻辑，自动关联 request_id
    let result = run(request_ctx.clone())
        .instrument(info_span!("request", request_id = %request_ctx.request_id))
        .await;

    // 记录执行时间
    info!(
        target: "metrics",
        request_id = %request_ctx.request_id,
        total_duration_ms = %format!("{:.2}", request_ctx.elapsed().as_secs_f64() * 1000.0),
        "request completed"
    );

    result
}

async fn run(request_ctx: RequestContext) -> Result<()> {
    let _timer = Timer::new("main_execution").with_context(&request_ctx);

    let cfg = AppConfig::parse();

    info!(
        target: "config",
        matrix_homeserver = %cfg.matrix_homeserver,
        matrix_room_id = %cfg.matrix_room_id,
        matrix_token = %Sanitizer::matrix_token(&cfg.matrix_access_token),
        openai_model = %cfg.openai_model,
        openai_key = %Sanitizer::api_key(&cfg.openai_api_key),
        context_limit = cfg.context_limit,
        "configuration loaded"
    );

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

    info!("tools loaded: {}", tools.len());

    // First LLM call with tools
    let (mut message, finish_reason) = llm.chat(messages.clone(), Some(tools)).await?;
    debug!("first response finish_reason: {:?}", finish_reason);

    // Handle tool calls if any
    if let Some(tool_calls) = message.tool_calls.clone() {
        info!(
            target: "llm",
            tool_calls_count = tool_calls.len(),
            "llm requested tool execution"
        );

        // Add assistant message with tool calls
        messages.push(Message {
            role: "assistant".to_string(),
            content: message.content,
            tool_calls: Some(tool_calls.clone()),
            tool_call_id: None,
        });

        // Execute each tool call and add results
        for (idx, tool_call) in tool_calls.iter().enumerate() {
            let _tool_timer = Timer::new(format!("tool_{}_{}", idx, tool_call.function.name))
                .with_context(&request_ctx);
            let result = execute_tool(tool_call).await;
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
    let answer = message
        .content
        .unwrap_or_else(|| "(无响应内容)".to_string());
    matrix
        .send_text_message(&cfg.matrix_room_id, &answer)
        .await?;

    info!("stateless cycle completed");
    Ok(())
}

/// Execute a tool call and return formatted result
async fn execute_tool(tool_call: &ToolCall) -> String {
    info!(
        target: "tool",
        tool_name = %tool_call.function.name,
        tool_id = %tool_call.id,
        "executing tool"
    );

    match ToolRegistry::execute(tool_call).await {
        Ok(result) => {
            info!(
                target: "tool",
                tool_name = %tool_call.function.name,
                result_length = result.len(),
                "tool executed successfully"
            );
            result
        }
        Err(e) => {
            let error_msg = format!("工具执行错误: {}", e);
            error!(
                target: "tool",
                tool_name = %tool_call.function.name,
                error = %e,
                "{}",
                error_msg
            );
            error_msg
        }
    }
}
