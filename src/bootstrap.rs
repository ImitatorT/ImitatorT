//! # ImitatorT Framework Bootstrap Module
//!
//! Provides Spring Boot-style auto-configuration and startup functionality
//! This module encapsulates the framework's auto-configuration logic, allowing developers to start a complete multi-Agent system and Web service with minimal configuration

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::{start_web_server, Agent, AppConfig, CompanyBuilder, CompanyConfig, VirtualCompany};

/// Framework Launcher - Provides auto-configured startup functionality
#[derive(Default)]
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
                if let Err(e) =
                    Self::start_agent_loops(company_for_agents, message_tx_for_agents).await
                {
                    tracing::error!("Agent loops error: {}", e);
                }
            });
        } else {
            info!("⏸️  Agent loops disabled by configuration");
        }

        // Start Web service
        if self.config.output_mode == "web" {
            info!(
                "🌐 Starting embedded web server on {}",
                self.config.web_bind
            );

            start_web_server(
                &self.config.web_bind,
                agents,
                message_tx,
                company_arc.store().clone(),
            )
            .await?;

            info!("✅ Web server started successfully");

            // Wait until terminated by interrupt signal
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl+c");
            info!("🛑 Received shutdown signal");
        } else {
            info!("ℹ️  Running in console mode");
            // Wait for interrupt signal in console mode
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl+c");
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

        // 启动事件驱动的Agent系统
        company.run().await
    }

    /// 初始化框架特定的技能和权限
    async fn initialize_framework_skills(&self, company: &VirtualCompany) -> Result<()> {
        use crate::domain::skill::{BindingType, Skill, SkillToolBinding, ToolAccessType};

        // 检测是否为哲学公司配置
        let is_philosophy_company = company.name() == "哲学讨论大会";

        if is_philosophy_company {
            tracing::info!("🎓 检测到哲学公司配置，注册哲学专属 Skills...");
            self.initialize_philosophy_skills(company)?;
        } else {
            // 注册访问思过崖线群聊的技能（默认）
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
        }

        tracing::info!("✅ Framework skills initialized successfully");
        Ok(())
    }

    /// 初始化哲学公司的 Skills
    fn initialize_philosophy_skills(&self, company: &VirtualCompany) -> Result<()> {
        use crate::domain::skill::{BindingType, Skill, SkillToolBinding, ToolAccessType};

        // --- 1. 设置工具访问类型 ---
        company.set_tool_access("http.fetch", ToolAccessType::Private)?;
        company.set_tool_access("file.read", ToolAccessType::Private)?;
        company.set_tool_access("file.write", ToolAccessType::Private)?;
        company.set_tool_access("file.delete", ToolAccessType::Private)?;
        company.set_tool_access("file.list", ToolAccessType::Private)?;
        company.set_tool_access("shell.exec", ToolAccessType::Private)?;

        // --- 2. CEO 专属 Skills ---
        // news_fetcher: 允许 CEO 获取实时新闻
        let news_fetcher_skill = Skill::new(
            "news_fetcher".to_string(),
            "新闻获取者".to_string(),
            "能够获取实时新闻和网络信息，用于生成基于当前事件的哲学议题".to_string(),
            "information".to_string(),
            "1.0.0".to_string(),
            "system".to_string(),
        );
        company.register_skill(news_fetcher_skill)?;
        company.bind_skill_tool(SkillToolBinding::new(
            "news_fetcher".to_string(),
            "http.fetch".to_string(),
            BindingType::Required,
        ))?;

        // topic_generator: 哲学议题生成技能
        let topic_generator_skill = Skill::new(
            "topic_generator".to_string(),
            "议题生成者".to_string(),
            "能够根据新闻和当前事件生成深刻的哲学议题".to_string(),
            "analysis".to_string(),
            "1.0.0".to_string(),
            "system".to_string(),
        );
        company.register_skill(topic_generator_skill)?;

        // --- 3. 中华哲学部 Skills ---
        // 道家技能
        let taoist_wisdom = Skill::new(
            "taoist_wisdom".to_string(),
            "道家智慧".to_string(),
            "理解道家经典和思想，能够引用《道德经》、《庄子》等经典".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "laozi".to_string(),
        );
        company.register_skill(taoist_wisdom)?;
        company.bind_skill_tool(SkillToolBinding::new(
            "taoist_wisdom".to_string(),
            "file.read".to_string(),
            BindingType::Optional,
        ))?;

        // 儒家技能
        let confucian_virtue = Skill::new(
            "confucian_virtue".to_string(),
            "儒家美德".to_string(),
            "理解儒家经典和思想，能够引用《论语》、《孟子》等经典".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "confucius".to_string(),
        );
        company.register_skill(confucian_virtue)?;

        // 法家技能
        let legalist_governance = Skill::new(
            "legalist_governance".to_string(),
            "法家治国".to_string(),
            "理解法家思想和治国理念，能够引用《韩非子》等经典".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "hanfeizi".to_string(),
        );
        company.register_skill(legalist_governance)?;

        // --- 4. 亚伯拉罕哲学部 Skills ---
        // 犹太哲学技能
        let jewish_wisdom = Skill::new(
            "jewish_wisdom".to_string(),
            "犹太智慧".to_string(),
            "理解犹太教义和哲学传统，能够引用《塔木德》等经典".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "maimonides".to_string(),
        );
        company.register_skill(jewish_wisdom)?;

        // 伊斯兰哲学技能
        let islamic_philosophy = Skill::new(
            "islamic_philosophy".to_string(),
            "伊斯兰哲学".to_string(),
            "理解伊斯兰哲学传统和教义学".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "avicenna".to_string(),
        );
        company.register_skill(islamic_philosophy)?;

        // 基督教神学技能
        let christian_theology = Skill::new(
            "christian_theology".to_string(),
            "基督教神学".to_string(),
            "理解基督教神学和教父哲学传统".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "aquinas".to_string(),
        );
        company.register_skill(christian_theology)?;

        // --- 5. 欧洲哲学部 Skills ---
        // 古希腊哲学技能
        let ancient_greek_wisdom = Skill::new(
            "ancient_greek_wisdom".to_string(),
            "古希腊智慧".to_string(),
            "理解古希腊哲学传统和经典著作".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "aristotle".to_string(),
        );
        company.register_skill(ancient_greek_wisdom)?;

        // 近代西方哲学技能
        let modern_western_philosophy = Skill::new(
            "modern_western_philosophy".to_string(),
            "近代西方哲学".to_string(),
            "理解笛卡尔、康德、黑格尔等近代哲学家思想".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "kant".to_string(),
        );
        company.register_skill(modern_western_philosophy)?;

        // 欧陆哲学技能
        let continental_philosophy = Skill::new(
            "continental_philosophy".to_string(),
            "欧陆哲学".to_string(),
            "理解现象学、存在主义等欧陆哲学传统".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "husserl".to_string(),
        );
        company.register_skill(continental_philosophy)?;

        // 分析哲学技能
        let analytic_philosophy = Skill::new(
            "analytic_philosophy".to_string(),
            "分析哲学".to_string(),
            "理解分析哲学传统和逻辑分析方法".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "russell".to_string(),
        );
        company.register_skill(analytic_philosophy)?;

        // --- 6. 科学哲学部 Skills ---
        // 科学哲学技能
        let philosophy_of_science = Skill::new(
            "philosophy_of_science".to_string(),
            "科学哲学".to_string(),
            "理解科学方法论和科学哲学问题".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "popper".to_string(),
        );
        company.register_skill(philosophy_of_science)?;

        // 心灵哲学技能
        let philosophy_of_mind = Skill::new(
            "philosophy_of_mind".to_string(),
            "心灵哲学".to_string(),
            "理解意识、心灵和认知科学哲学问题".to_string(),
            "philosophy".to_string(),
            "1.0.0".to_string(),
            "searle".to_string(),
        );
        company.register_skill(philosophy_of_mind)?;

        // --- 7. 争议解决部 Skills ---
        // 仲裁技能
        let mediation_skill = Skill::new(
            "mediation".to_string(),
            "争议调解".to_string(),
            "能够中立地仲裁哲学争议，促进理性对话".to_string(),
            "communication".to_string(),
            "1.0.0".to_string(),
            "judge".to_string(),
        );
        company.register_skill(mediation_skill)?;

        tracing::info!("✅ 哲学公司 Skills 注册完成 (共 15 个技能)");
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
