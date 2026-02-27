//! # ImitatorT Framework Bootstrap Module
//!
//! Provides Spring Boot-style auto-configuration and startup functionality
//! This module encapsulates the framework's auto-configuration logic, allowing developers to start a complete multi-Agent system and Web service with minimal configuration

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::{
    Agent, AppConfig, CompanyBuilder, CompanyConfig, VirtualCompany, start_web_server,
};

/// Framework Launcher - Provides auto-configured startup functionality
pub struct FrameworkLauncher {
    config: AppConfig,
}

impl FrameworkLauncher {
    /// Create a new framework launcher
    pub fn new() -> Self {
        Self {
            config: AppConfig::from_env(),
        }
    }

    /// Create framework launcher with custom configuration
    pub fn with_config(config: AppConfig) -> Self {
        Self { config }
    }

    /// Auto-configure and start the complete framework services
    pub async fn launch(&self) -> Result<()> {
        info!("🚀 Launching ImitatorT Framework...");

        // Initialize multi-Agent system
        let company = self.initialize_multi_agent_system().await?;

        // Initialize framework skills and permissions
        self.initialize_framework_skills(&company).await?;

        // Start services
        self.start_services(company).await?;

        Ok(())
    }

    /// Initialize multi-Agent system
    async fn initialize_multi_agent_system(&self) -> Result<VirtualCompany> {
        info!("🔧 Initializing multi-agent system...");

        // Try to load new configuration from config file
        if let Ok(config) = self.load_company_config() {
            info!("📋 Loaded company configuration");
            let company = CompanyBuilder::from_config(config)?
                .build_and_save()
                .await?;
            info!("✅ Multi-agent system initialized with custom configuration");
            return Ok(company);
        }

        // Try to load from database
        info!("🔍 Attempting to load from database...");
        match VirtualCompany::from_sqlite(&self.config.db_path).await {
            Ok(company) => {
                info!("✅ Loaded existing company from database");
                Ok(company)
            }
            Err(_) => {
                warn!("⚠️  No existing configuration found, using default setup");
                let config = CompanyConfig::test_config();
                let company = CompanyBuilder::from_config(config)?
                    .build_and_save()
                    .await?;
                info!("✅ Initialized default multi-agent system");
                Ok(company)
            }
        }
    }

    /// Start all services
    async fn start_services(&self, company: VirtualCompany) -> Result<()> {
        info!("⚡ Starting framework services...");

        // Get Agent list
        let agents: Vec<Agent> = company.get_agents().await?;
        info!("👥 Loaded {} agents", agents.len());

        // Create message broadcast channel
        let (message_tx, _) = broadcast::channel::<crate::Message>(1000);

        // Create shared reference to company instance
        let company_arc = Arc::new(company);

        // Decide whether to start Agent loops based on configuration
        if self.config.run_agent_loops {
            info!("🔄 Starting agent autonomous loops...");
            let company_for_agents = company_arc.clone();
            let message_tx_for_agents = message_tx.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::start_agent_loops(company_for_agents, message_tx_for_agents).await {
                    tracing::error!("Agent loops error: {}", e);
                }
            });
        } else {
            info!("⏸️  Agent loops disabled by configuration");
        }

        // Start Web service
        if self.config.output_mode == "web" {
            info!("🌐 Starting embedded web server on {}", self.config.web_bind);

            start_web_server(
                &self.config.web_bind,
                agents,
                message_tx,
                company_arc.store().clone()
            ).await?;

            info!("✅ Web server started successfully");

            // Wait until terminated by interrupt signal
            tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
            info!("🛑 Received shutdown signal");
        } else {
            info!("ℹ️  Running in console mode");
            // Wait for interrupt signal in console mode
            tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
            info!("🛑 Received shutdown signal");
        }

        Ok(())
    }

    /// Load company configuration
    fn load_company_config(&self) -> Result<CompanyConfig> {
        // Try to load configuration from YAML file
        if let Ok(content) = std::fs::read_to_string("company_config.yaml") {
            let config: CompanyConfig = serde_yaml::from_str(&content)?;
            return Ok(config);
        }

        Err(anyhow::anyhow!("Config file not found"))
    }

    /// Start Agent loops
    async fn start_agent_loops(
        company: Arc<VirtualCompany>,
        _message_tx: broadcast::Sender<crate::Message>,
    ) -> Result<()> {
        info!("🤖 Starting agent autonomous operations...");

        // Start loops for all Agents via framework API
        company.run().await
    }

    /// 初始化框架特定的技能和权限
    async fn initialize_framework_skills(&self, company: &VirtualCompany) -> Result<()> {
        use crate::domain::skill::{Skill, SkillToolBinding, BindingType, ToolAccessType};

        // 注册访问思过崖线群聊的技能
        let guilty_line_access_skill = Skill::new(
            "guilty_line_access".to_string(),
            "Guilty Line Access".to_string(),
            "Permission to access the hidden Guilty Line group".to_string(),
            "communication".to_string(),
            "1.0".to_string(),
            "system".to_string(),
        );

        company.register_skill(guilty_line_access_skill)?;

        // 将技能绑定到发送到思过崖线群聊的工具
        let binding = SkillToolBinding::new(
            "guilty_line_access".to_string(),
            "message.send_to_guilty_line".to_string(),
            BindingType::Required,
        );

        company.bind_skill_tool(binding)?;

        // 设置工具访问类型为私有，需要特定技能才能访问
        company.set_tool_access("message.send_to_guilty_line", ToolAccessType::Private)?;

        tracing::info!("✅ Framework skills initialized successfully");
        Ok(())
    }
}

/// Quick start function - Provides the simplest startup method
pub async fn quick_start() -> Result<()> {
    let launcher = FrameworkLauncher::new();
    launcher.launch().await
}

/// Quick start with custom configuration
pub async fn start_with_config(config: AppConfig) -> Result<()> {
    let launcher = FrameworkLauncher::with_config(config);
    launcher.launch().await
}