//! ImitatorT - Spring Boot Style Launcher
//!
//! Automatically configures multi-Agent system and Web service

use std::sync::Arc;

use anyhow::Result;
use imitatort::{
    Agent, AppConfig, CompanyBuilder, CompanyConfig, VirtualCompany, start_web_server,
};
use tokio::sync::broadcast;
use tracing::{info, warn};

// 加载环境变量
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    // 加载 .env 文件
    dotenv().ok();

    // 初始化日志
    tracing_subscriber::fmt::init();

    info!("🚀 Starting ImitatorT - Multi-Agent Company Framework...");

    // 加载应用程序配置
    let app_config = AppConfig::from_env();
    info!("Using configuration: output_mode={}, web_bind={}", app_config.output_mode, app_config.web_bind);

    // Automatically configure and start multi-Agent system and Web service
    let company = initialize_framework(&app_config).await?;

    // 根据配置自动启动相应的服务
    start_services(company, &app_config).await?;

    Ok(())
}

/// Initialize framework - Automatically configure multi-Agent system
async fn initialize_framework(app_config: &AppConfig) -> Result<VirtualCompany> {
    info!("🔧 Initializing multi-agent framework...");

    // Try to load new configuration from config file
    if let Ok(config) = load_config() {
        info!("📋 Loaded company configuration from company_config.yaml");
        // Create new company using config file, save to SQLite
        let company = CompanyBuilder::from_config(config)?
            .build_and_save()
            .await?;
        info!("✅ Multi-agent system initialized with configuration");
        return Ok(company);
    }

    // Try to load from existing SQLite database
    info!("🔍 No config file found, attempting to load from database...");
    match VirtualCompany::from_sqlite(&app_config.db_path).await {
        Ok(company) => {
            info!("✅ Loaded existing company from database: {}", app_config.db_path);
            Ok(company)
        }
        Err(_) => {
            warn!("⚠️  No existing database found, initializing with default configuration");
            let config = CompanyConfig::test_config();
            let company = CompanyBuilder::from_config(config)?
                .build_and_save()
                .await?;
            info!("✅ Initialized default multi-agent system");
            Ok(company)
        }
    }
}

/// Load configuration file
fn load_config() -> Result<CompanyConfig> {
    // Try to load configuration from YAML file
    if let Ok(content) = std::fs::read_to_string("company_config.yaml") {
        let config: CompanyConfig = serde_yaml::from_str(&content)?;
        return Ok(config);
    }

    Err(anyhow::anyhow!("Config file not found"))
}

/// Start services - Automatically start corresponding functions based on configuration
async fn start_services(company: VirtualCompany, app_config: &AppConfig) -> Result<()> {
    info!("⚡ Starting framework services...");

    // Get Agent list for Web API
    let agents: Vec<Agent> = company.get_agents().await?;
    info!("👥 Loaded {} agents", agents.len());

    // Create message broadcast channel
    let (message_tx, _) = broadcast::channel::<imitatort::Message>(1000);

    // Create shared reference to company instance
    let company_arc = Arc::new(company);

    // Decide whether to start Agent loops based on configuration
    if app_config.run_agent_loops {
        info!("🔄 Starting agent autonomous loops...");
        let company_for_agents = company_arc.clone();
        let message_tx_for_agents = message_tx.clone();

        tokio::spawn(async move {
            start_agent_loops(company_for_agents, message_tx_for_agents).await;
        });
    } else {
        info!("⏸️  Agent loops disabled by configuration");
    }

    // Automatically start Web service (if configured for web mode)
    if app_config.output_mode == "web" {
        info!("🌐 Starting web server on {}", app_config.web_bind);

        start_web_server(
            &app_config.web_bind,
            agents,
            message_tx,
            company_arc.store().clone()
        ).await?;

        info!("✅ Web server started successfully");
    } else {
        info!("ℹ️  Running in console mode (no web interface)");
        // In console mode, we still keep Agent loops running
        // Wait until terminated by interrupt signal
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        info!("🛑 Received shutdown signal");
    }

    Ok(())
}

/// Start autonomous loops for all Agents
async fn start_agent_loops(
    company: Arc<VirtualCompany>,
    _message_tx: broadcast::Sender<imitatort::Message>,
) {
    info!("🤖 Starting agent autonomous operations...");

    // 启动事件驱动的Agent系统
    match company.run().await {
        Ok(_) => info!("Agent operations completed"),
        Err(e) => {
            tracing::error!("Agent operations error: {}", e);
        }
    }
}
