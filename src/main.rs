//! ImitatorT - Matrix Appservice Launcher
//!
//! Automatically configures multi-Agent system with Matrix integration

use std::sync::Arc;

use anyhow::Result;
use imitatort::{
    AppConfig, CompanyBuilder, CompanyConfig, VirtualCompany,
    infrastructure::matrix::{MatrixConfig, MatrixClient, AppService, SyncService, Mapper},
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn, error};

// 加载环境变量
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    // 加载 .env 文件
    dotenv().ok();

    // 初始化日志
    tracing_subscriber::fmt::init();

    info!("🚀 Starting ImitatorT - Matrix Appservice...");

    // 加载应用程序配置
    let app_config = AppConfig::from_env();
    info!(
        "Using configuration: output_mode={}, run_agent_loops={}",
        app_config.output_mode, app_config.run_agent_loops
    );

    // 加载 Matrix 配置
    let matrix_config = MatrixConfig::from_env()
        .expect("Failed to load Matrix configuration. Please set required environment variables.");
    info!("📋 Matrix configuration loaded for server: {}", matrix_config.server_name);

    // Initialize multi-Agent system
    let company = initialize_multi_agent_system(&app_config).await?;

    // Start Matrix integration
    start_matrix_services(company, &matrix_config, &app_config).await?;

    Ok(())
}

/// Initialize multi-Agent system
async fn initialize_multi_agent_system(app_config: &AppConfig) -> Result<VirtualCompany> {
    info!("🔧 Initializing multi-agent system...");

    // Try to load configuration from config file
    if let Ok(config) = load_company_config() {
        info!("📋 Loaded company configuration from company_config.yaml");
        let company = CompanyBuilder::from_config(config)?
            .build_and_save()
            .await?;
        info!("✅ Multi-agent system initialized with configuration");
        return Ok(company);
    }

    // Try to load from database
    info!("🔍 Attempting to load from database...");
    match VirtualCompany::from_sqlite(&app_config.db_path).await {
        Ok(company) => {
            info!("✅ Loaded existing company from database");
            Ok(company)
        }
        Err(_) => {
            warn!("⚠️  No existing configuration found, initializing with default setup");
            let config = CompanyConfig::test_config();
            let company = CompanyBuilder::from_config(config)?
                .build_and_save()
                .await?;
            info!("✅ Initialized default multi-agent system");
            Ok(company)
        }
    }
}

/// Load company configuration
fn load_company_config() -> Result<CompanyConfig> {
    if let Ok(content) = std::fs::read_to_string("company_config.yaml") {
        let config: CompanyConfig = serde_yaml::from_str(&content)?;
        return Ok(config);
    }
    Err(anyhow::anyhow!("Config file not found"))
}

/// Start Matrix services
async fn start_matrix_services(
    company: VirtualCompany,
    matrix_config: &MatrixConfig,
    app_config: &AppConfig,
) -> Result<()> {
    info!("⚡ Starting Matrix services...");

    // Get Agent list
    let agents = company.get_agents().await?;
    info!("👥 Loaded {} agents", agents.len());

    // Create message broadcast channel
    let (message_tx, _message_rx) = broadcast::channel::<imitatort::Message>(1000);

    // Create shared reference to company instance
    let company_arc = Arc::new(company);

    // Initialize Matrix client
    let matrix_client = MatrixClient::new(matrix_config);

    // Initialize mapper
    let mapper = Arc::new(RwLock::new(Mapper::new(matrix_config)));

    // Create sync service
    let sync_service = SyncService::new(
        matrix_client.clone(),
        matrix_config,
        mapper.clone(),
        company_arc.store().clone(),
    );

    // Sync users (register virtual users if needed)
    info!("🔄 Synchronizing users to Matrix...");
    if let Err(e) = sync_service.sync_all_users().await {
        error!("Failed to sync users: {}", e);
    }

    // Sync rooms (create department rooms)
    info!("🔄 Synchronizing rooms to Matrix...");
    if let Err(e) = sync_service.sync_all_rooms().await {
        error!("Failed to sync rooms: {}", e);
    }

    // Start Appservice server to listen for Homeserver events
    info!("🌐 Starting Matrix Appservice on port {}...", matrix_config.appservice_port);
    let appservice = AppService::new(matrix_config.clone(), message_tx.clone());

    // Spawn Appservice in background
    tokio::spawn(async move {
        if let Err(e) = appservice.run().await {
            error!("Appservice error: {}", e);
        }
    });

    // Start Agent loops if configured
    if app_config.run_agent_loops {
        info!("🔄 Starting agent autonomous loops...");
        let company_for_agents = company_arc.clone();
        tokio::spawn(async move {
            if let Err(e) = company_for_agents.run().await {
                error!("Agent operations error: {}", e);
            }
        });
    }

    info!("✅ Matrix Appservice started successfully");
    info!("ℹ️  Connect with Element or other Matrix clients to {}", matrix_config.homeserver_url);

    // Wait for shutdown signal
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl+c");

    info!("🛑 Received shutdown signal");

    Ok(())
}
