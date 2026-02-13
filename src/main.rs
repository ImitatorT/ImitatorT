mod a2a;
mod config;
mod llm;
mod logger;
mod matrix;
mod output;
mod store;
mod tool;

use anyhow::{Context, Result};
use clap::Parser;
use config::{AppConfig, OutputMode};
use llm::{Message, OpenAIClient};
use logger::{LogConfig, RequestContext, Sanitizer, Timer};
use output::{Output, OutputFactory};
use std::sync::Arc;
use store::MessageStore;
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

    // 验证配置
    cfg.validate().context("Configuration validation failed")?;

    info!(
        target: "config",
        output_mode = %cfg.output_mode,
        agent_id = %cfg.agent_id,
        agent_name = %cfg.agent_name,
        openai_model = %cfg.openai_model,
        openai_key = %Sanitizer::api_key(&cfg.openai_api_key),
        context_limit = cfg.context_limit,
        store_type = %cfg.store_type,
        "configuration loaded"
    );

    // 创建消息存储
    let store = create_store(&cfg).await?;

    // 创建输出处理器
    let output = create_output(&cfg, store.clone()).await?;

    // 创建 LLM 客户端
    let llm = OpenAIClient::new(cfg.openai_api_key.clone(), cfg.openai_model.clone());

    // 根据模式执行不同的主循环
    match cfg.output_mode {
        OutputMode::Cli if cfg.interactive => {
            run_interactive_loop(&cfg, output.as_ref(), &llm, &store, &request_ctx).await
        }
        _ => {
            run_single_cycle(&cfg, output.as_ref(), &llm, &store, &request_ctx).await
        }
    }
}

/// 创建消息存储
async fn create_store(cfg: &AppConfig) -> Result<MessageStore> {
    let store = match cfg.store_type {
        config::StoreType::Memory => {
            info!("Using in-memory message store (max_size: {})", cfg.store_max_size);
            MessageStore::new(cfg.store_max_size)
        }
        #[cfg(feature = "persistent-store")]
        config::StoreType::Persistent => {
            info!(
                "Using persistent message store at {} (max_size: {})",
                cfg.store_path, cfg.store_max_size
            );
            MessageStore::new_persistent(&cfg.store_path, cfg.store_max_size)
                .context("Failed to create persistent store")?
        }
    };
    Ok(store)
}

/// 创建输出处理器
async fn create_output(
    cfg: &AppConfig,
    store: MessageStore,
) -> Result<Box<dyn Output>> {
    let output: Box<dyn Output> = match cfg.output_mode {
        OutputMode::Matrix => {
            let (homeserver, token, room_id) = cfg
                .matrix_config()
                .context("Matrix configuration missing")?;
            OutputFactory::create_matrix(
                homeserver.to_string(),
                token.to_string(),
                room_id.to_string(),
                store,
            )?
        }
        OutputMode::Cli => OutputFactory::create_cli(store, cfg.cli_echo),
        OutputMode::A2A => {
            let card = a2a::create_default_agent_card(&cfg.agent_id, &cfg.agent_name);
            let agent = Arc::new(a2a::A2AAgent::new(card));
            
            // 注册 Peer agents
            if let Some(ref peers) = cfg.a2a_peer_agents {
                for peer_id in peers.split(',') {
                    let peer_card = a2a::create_default_agent_card(peer_id.trim(), peer_id.trim());
                    agent.register_peer(peer_card).await;
                }
            }
            
            OutputFactory::create_a2a(agent, store, cfg.a2a_target_agent.clone())
        }
        OutputMode::Hybrid => {
            // Hybrid 模式: Matrix 作为前端 + A2A 内部通信
            let (homeserver, token, room_id) = cfg
                .matrix_config()
                .context("Matrix configuration missing for hybrid mode")?;
            
            // 创建 A2A Agent
            let card = a2a::create_default_agent_card(&cfg.agent_id, &cfg.agent_name);
            let agent = Arc::new(a2a::A2AAgent::new(card));
            
            // 注册 Peer agents
            if let Some(ref peers) = cfg.a2a_peer_agents {
                for peer_id in peers.split(',') {
                    let peer_card = a2a::create_default_agent_card(peer_id.trim(), peer_id.trim());
                    agent.register_peer(peer_card).await;
                }
            }
            
            // 创建 Matrix 输出（作为主要输出）
            // 注意：Hybrid 模式目前使用 Matrix 作为主要输出
            // 可以通过桥接器将 A2A 消息同步到 Matrix
            OutputFactory::create_matrix(
                homeserver.to_string(),
                token.to_string(),
                room_id.to_string(),
                store.clone(),
            )?
        }
    };

    info!("Output handler created: {:?}", output.mode());
    Ok(output)
}

/// 运行单次执行循环
async fn run_single_cycle(
    cfg: &AppConfig,
    output: &dyn Output,
    llm: &OpenAIClient,
    store: &MessageStore,
    request_ctx: &RequestContext,
) -> Result<()> {
    info!("building stateless context");
    let context = output.get_context(cfg.context_limit).await?;

    // 如果有输入消息，添加到存储
    if let Some(ref input) = cfg.input_message {
        let input_msg = store::ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: "user".to_string(),
            content: input.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            message_type: store::MessageType::Text,
        };
        store.add_message(input_msg).await?;
    }

    // Build messages with system prompt and context
    let user_content = if let Some(ref input) = cfg.input_message {
        format!(
            "上下文:\n{}\n\n当前消息:\n{}\n\n请基于上下文和当前消息分析并决定是否需要使用工具。",
            context, input
        )
    } else {
        format!(
            "上下文:\n{}\n\n请基于上下文分析并决定是否需要使用工具。如果需要执行命令或获取网页内容，请使用相应工具。",
            context
        )
    };

    let mut messages = vec![
        Message {
            role: "system".to_string(),
            content: Some(cfg.system_prompt.clone()),
            tool_calls: None,
            tool_call_id: None,
        },
        Message {
            role: "user".to_string(),
            content: Some(user_content),
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
            let _tool_timer =
                Timer::new(format!("tool_{}_{}", idx, tool_call.function.name))
                    .with_context(request_ctx);
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

    // Send final response
    let answer = message
        .content
        .unwrap_or_else(|| "(无响应内容)".to_string());
    
    output
        .send_message(&cfg.agent_name, &answer)
        .await
        .context("Failed to send response")?;

    info!("stateless cycle completed");
    Ok(())
}

/// 运行交互式循环（CLI 模式）
async fn run_interactive_loop(
    cfg: &AppConfig,
    output: &dyn Output,
    llm: &OpenAIClient,
    store: &MessageStore,
    _request_ctx: &RequestContext,
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    println!("\n=== Interactive Mode ===");
    println!("Agent: {} ({})", cfg.agent_name, cfg.agent_id);
    println!("Model: {}", cfg.openai_model);
    println!("Type '/quit' or '/exit' to quit\n");

    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    loop {
        print!("> ");
        let _ = std::io::Write::flush(&mut std::io::stdout());

        match lines.next_line().await {
            Ok(Some(line)) => {
                let input = line.trim();
                
                if input.is_empty() {
                    continue;
                }

                if input == "/quit" || input == "/exit" {
                    println!("Goodbye!");
                    break;
                }

                if input == "/help" {
                    println!("Commands: /quit, /exit, /help, /clear, /context");
                    continue;
                }

                if input == "/clear" {
                    store.clear().await?;
                    println!("Context cleared.");
                    continue;
                }

                if input == "/context" {
                    let context = store.get_context_string(cfg.context_limit).await;
                    println!("\n--- Current Context ---");
                    println!("{}", context);
                    println!("----------------------\n");
                    continue;
                }

                // 处理用户输入
                let input_msg = store::ChatMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    sender: "user".to_string(),
                    content: input.to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                    message_type: store::MessageType::Text,
                };
                store.add_message(input_msg).await?;

                // 获取上下文并调用 LLM
                let context = store.get_context_string(cfg.context_limit).await;
                
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
                            "上下文:\n{}\n\n当前消息:\n{}\n\n请基于上下文和当前消息回复。",
                            context, input
                        )),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                ];

                let tools = ToolRegistry::get_tools();
                let (mut message, _finish_reason) = llm.chat(messages.clone(), Some(tools)).await?;

                // 处理工具调用
                if let Some(tool_calls) = message.tool_calls.clone() {
                    messages.push(Message {
                        role: "assistant".to_string(),
                        content: message.content,
                        tool_calls: Some(tool_calls.clone()),
                        tool_call_id: None,
                    });

                    for tool_call in tool_calls.iter() {
                        let result = execute_tool(tool_call).await;
                        messages.push(Message {
                            role: "tool".to_string(),
                            content: Some(result),
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id.clone()),
                        });
                    }

                    let (final_message, _) = llm.chat(messages, None).await?;
                    message = final_message;
                }

                let answer = message
                    .content
                    .unwrap_or_else(|| "(无响应内容)".to_string());

                output.send_message(&cfg.agent_name, &answer).await?;
            }
            Ok(None) => break, // EOF
            Err(e) => {
                error!("Failed to read input: {}", e);
                break;
            }
        }
    }

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
