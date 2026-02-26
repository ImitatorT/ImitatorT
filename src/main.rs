//! ImitatorT - 主入口
//!
//! 支持 CLI 和 Web 两种模式

use std::sync::Arc;

use anyhow::Result;
use imitatort_stateless_company::{
    Agent, CompanyBuilder, CompanyConfig, VirtualCompany, start_web_server,
};
use tokio::sync::broadcast;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    info!("Starting ImitatorT...");

    // 读取环境变量
    let output_mode = std::env::var("OUTPUT_MODE").unwrap_or_else(|_| "cli".to_string());
    let web_bind = std::env::var("WEB_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    // 加载或创建公司
    let company = load_or_create_company().await?;

    match output_mode.as_str() {
        "web" => {
            info!("Running in Web mode on {}", web_bind);
            run_web_mode(company, &web_bind).await?;
        }
        _ => {
            info!("Running in CLI mode");
            run_cli_mode(company).await?;
        }
    }

    Ok(())
}

/// 加载或创建公司
async fn load_or_create_company() -> Result<VirtualCompany> {
    // 尝试从配置文件加载新配置
    if let Ok(config) = load_config() {
        info!("Loaded config from company_config.yaml");
        // 使用配置文件创建新公司，保存到SQLite
        return CompanyBuilder::from_config(config)?
            .build_and_save()
            .await;
    }

    // 尝试从已有的SQLite数据库加载
    info!("No config file found, trying to load from SQLite...");
    match VirtualCompany::from_sqlite("imitatort.db").await {
        Ok(company) => {
            info!("Loaded company from SQLite database");
            Ok(company)
        }
        Err(_) => {
            info!("No existing database found, using default test config");
            let config = CompanyConfig::test_config();
            CompanyBuilder::from_config(config)?
                .build_and_save()
                .await
        }
    }
}

/// 加载配置文件
fn load_config() -> Result<CompanyConfig> {
    // 尝试从 YAML 文件加载配置
    if let Ok(content) = std::fs::read_to_string("company_config.yaml") {
        let config: CompanyConfig = serde_yaml::from_str(&content)?;
        return Ok(config);
    }

    Err(anyhow::anyhow!("Config file not found"))
}

/// 运行 CLI 模式
async fn run_cli_mode(company: VirtualCompany) -> Result<()> {
    company.run().await?;
    Ok(())
}

/// 运行 Web 模式
async fn run_web_mode(company: VirtualCompany, bind_addr: &str) -> Result<()> {
    // 从公司获取 Agent 列表
    let agents: Vec<Agent> = company.get_agents().await?;
    info!("Loaded {} agents for Web API", agents.len());

    // 创建消息广播通道
    let (message_tx, _) = broadcast::channel::<imitatort_stateless_company::Message>(1000);

    // 创建公司实例的共享引用
    let company_arc = Arc::new(company);
    let company_for_agents = company_arc.clone();
    let message_tx_for_agents = message_tx.clone();

    tokio::spawn(async move {
        if let Err(e) = run_agent_loops(company_for_agents, message_tx_for_agents).await {
            error!("Agent loops error: {}", e);
        }
    });

    // 启动 Web 服务器
    start_web_server(bind_addr, agents, message_tx, company_arc.store().clone()).await?;

    Ok(())
}

/// 运行所有 Agent 的自主循环
async fn run_agent_loops(
    _company: Arc<VirtualCompany>,
    _message_tx: broadcast::Sender<imitatort_stateless_company::Message>,
) -> Result<()> {
    // 这里启动所有 Agent 的自主循环
    // 简化版本：只是保持运行状态
    info!("Agent loops started");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
