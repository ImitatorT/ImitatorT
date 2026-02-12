mod config;
mod llm;
mod matrix;
mod mcp;

use anyhow::Result;
use clap::Parser;
use config::AppConfig;
use llm::OpenAIClient;
use matrix::MatrixClient;
use tracing::info;

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

    let mcp_output = mcp::run_stdio_tool(&cfg.mcp_tool_command, &context).await?;
    let task = if let Some(tool_data) = mcp_output {
        format!("结合以下 MCP 工具返回信息执行部门任务：\n{}", tool_data)
    } else {
        "请基于上下文输出下一步可执行决策。".to_string()
    };

    let answer = llm.complete(&cfg.system_prompt, &context, &task).await?;
    matrix
        .send_text_message(&cfg.matrix_room_id, &answer)
        .await?;

    info!("stateless cycle completed");
    Ok(())
}
