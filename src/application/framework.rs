//! 虚拟公司框架 API
//!
//! 框架主入口：VirtualCompany

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{broadcast, RwLock};
use tracing::info;

use crate::core::config::CompanyConfig;
use crate::core::messaging::MessageBus;
use crate::core::store::Store;
use crate::domain::{Message, Organization};
use crate::infrastructure::store::SqliteStore;

use super::company_runtime::{OrganizationManager, AgentManager, ToolCapabilityManager};

// 导入缺失的类型
use crate::{ToolRegistry, ToolEnvironment, FrameworkToolExecutor, CapabilityRegistry, McpServer, McpProtocolHandler};

// 默认数据库路径现在由 AppConfig 管理
// pub const DEFAULT_DB_PATH: &str = "imitatort.db"; // 已移除硬编码

/// 虚拟公司框架
///
/// 封装所有框架能力，提供简洁的 API
pub struct VirtualCompany {
    organization_manager: OrganizationManager,
    agent_manager: AgentManager,
    tool_capability_manager: ToolCapabilityManager,
    message_bus: Arc<MessageBus>,
    message_tx: broadcast::Sender<Message>,
    store: Arc<dyn Store>,
    watchdog_agent: Arc<crate::core::watchdog_agent::WatchdogAgent>,
}

impl VirtualCompany {
    /// 从配置创建虚拟公司，使用默认SQLite存储
    pub fn from_config(config: CompanyConfig) -> Result<Self> {
        Self::with_sqlite(config, std::env::var("DB_PATH").unwrap_or_else(|_| "imitatort.db".to_string()))
    }

    /// 从配置创建虚拟公司，使用指定路径的SQLite存储
    pub fn with_sqlite<P: AsRef<Path>>(config: CompanyConfig, db_path: P) -> Result<Self> {
        let store = Arc::new(SqliteStore::new(db_path)?);
        Ok(Self::with_store(config, store))
    }

    /// 从配置创建虚拟公司，使用指定的存储
    pub fn with_store(config: CompanyConfig, store: Arc<dyn Store>) -> Self {
        let message_bus = Arc::new(MessageBus::with_store(store.clone()));
        let (message_tx, _) = broadcast::channel(1000);

        let organization_manager = OrganizationManager::new(config);
        let tool_capability_manager = ToolCapabilityManager::new();
        let agent_manager = AgentManager::new(message_bus.clone());

        // 创建系统WatchdogAgent
        let watchdog_agent = Arc::new(crate::core::watchdog_agent::WatchdogAgent::new(
            crate::domain::Agent::new(
                "system_watchdog",
                "System Watchdog Agent",
                crate::domain::Role::simple("System", "System monitoring agent"),
                crate::domain::LLMConfig::openai("dummy-key"),
            )
        ));

        Self {
            organization_manager,
            agent_manager,
            tool_capability_manager,
            message_bus,
            message_tx,
            store,
            watchdog_agent,
        }
    }

    /// 从SQLite存储加载虚拟公司
    pub async fn from_sqlite<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let store = Arc::new(SqliteStore::new(db_path)?);
        Self::from_store(store).await
    }

    /// 从存储加载虚拟公司
    pub async fn from_store(store: Arc<dyn Store>) -> Result<Self> {
        let org = store.load_organization().await?;

        // 如果没有组织架构，返回错误
        if org.agents.is_empty() {
            return Err(anyhow::anyhow!(
                "No organization found in store. Please create config first."
            ));
        }

        let config = CompanyConfig {
            name: "Loaded Company".to_string(),
            organization: org,
        };

        Ok(Self::with_store(config, store))
    }

    /// 保存当前状态到存储
    pub async fn save(&self) -> Result<()> {
        info!("Saving company state to storage...");
        let org = self.organization_manager.organization().await;
        self.store.save_organization(&org).await?;
        info!("Company state saved successfully");
        Ok(())
    }

    /// 初始化并启动公司
    pub async fn run(&self) -> Result<()> {
        info!("Starting virtual company: {}", self.organization_manager.config().name);

        // 1. 初始化所有Agent
        let org = self.organization_manager.organization().await;
        self.agent_manager.initialize_agents(&org, Some(self.watchdog_agent.clone())).await?;
        drop(org); // 释放读锁

        info!("All {} agents initialized", self.agent_manager.get_agents().await?.len());

        // 现在Agent只在事件触发时激活，不再启动持续循环
        info!("Agents initialized and ready for event-driven activation");

        Ok(())
    }

    /// 获取消息流（用于外部监听）
    pub fn subscribe_messages(&self) -> broadcast::Receiver<Message> {
        self.message_tx.subscribe()
    }

    
    /// 获取组织架构（异步读取）
    pub async fn organization(&self) -> tokio::sync::RwLockReadGuard<'_, Organization> {
        self.organization_manager.organization().await
    }

    /// 获取组织架构引用（同步，仅用于需要 &Organization 的场景）
    pub fn organization_arc(&self) -> Arc<RwLock<Organization>> {
        self.organization_manager.organization_arc()
    }

    /// 获取存储引用
    pub fn store(&self) -> &Arc<dyn Store> {
        &self.store
    }

    /// 获取公司名称
    pub fn name(&self) -> &str {
        &self.organization_manager.config().name
    }

    /// 获取所有 Agent 列表（用于 Web API）
    pub async fn get_agents(&self) -> Result<Vec<crate::domain::Agent>> {
        let org = self.organization_manager.organization().await;
        Ok(org.agents.clone())
    }

    /// 获取 ToolRegistry 引用
    pub fn tool_registry(&self) -> Arc<ToolRegistry> {
        self.tool_capability_manager.tool_registry()
    }

    /// 注册应用自定义工具
    pub async fn register_app_tool(&self, tool: crate::domain::tool::Tool) -> Result<()> {
        self.tool_capability_manager.register_app_tool(tool).await
    }

    /// 创建工具执行环境
    pub fn create_tool_environment(&self) -> ToolEnvironment {
        self.tool_capability_manager.create_tool_environment(
            self.message_bus.clone(),
            self.organization_manager.organization_arc(),
            self.store.clone(),
        )
    }

    /// 获取框架工具执行器
    pub fn get_framework_tool_executor(&self) -> FrameworkToolExecutor {
        self.tool_capability_manager.get_framework_tool_executor(
            self.message_bus.clone(),
            self.organization_manager.organization_arc(),
            self.store.clone(),
        )
    }

    /// 获取 CapabilityRegistry 引用
    pub fn capability_registry(&self) -> Arc<CapabilityRegistry> {
        self.tool_capability_manager.capability_registry()
    }

    /// 注册应用自定义功能
    pub async fn register_app_capability(&self, capability: crate::domain::capability::Capability) -> Result<()> {
        self.tool_capability_manager.register_app_capability(capability).await
    }

    /// 创建 MCP 服务器
    pub fn create_mcp_server(&self, bind_addr: String) -> McpServer {
        self.tool_capability_manager.create_mcp_server(bind_addr)
    }

    /// 获取 MCP 协议处理器
    pub fn get_mcp_protocol_handler(&self) -> McpProtocolHandler {
        self.tool_capability_manager.get_mcp_protocol_handler()
    }

    /// 注册技能
    pub fn register_skill(&self, skill: crate::domain::skill::Skill) -> Result<()> {
        self.tool_capability_manager.register_skill(skill)
    }

    /// 绑定技能和工具
    pub fn bind_skill_tool(&self, binding: crate::domain::skill::SkillToolBinding) -> Result<()> {
        self.tool_capability_manager.bind_skill_tool(binding)
    }

    /// 设置工具访问类型
    pub fn set_tool_access(&self, tool_id: &str, access_type: crate::domain::skill::ToolAccessType) -> Result<()> {
        self.tool_capability_manager.set_tool_access(tool_id, access_type)
    }

    /// 获取技能管理器引用
    pub fn skill_manager(&self) -> Arc<crate::core::skill::SkillManager> {
        self.tool_capability_manager.skill_manager()
    }
}

/// 公司构建器
pub struct CompanyBuilder {
    config: Option<CompanyConfig>,
    store: Option<Arc<dyn Store>>,
}

impl CompanyBuilder {
    /// 创建新的构建器，使用默认SQLite路径
    pub fn new() -> Result<Self> {
        Self::with_sqlite(std::env::var("DB_PATH").unwrap_or_else(|_| "imitatort.db".to_string()))
    }

    /// 从配置创建，使用默认SQLite路径
    pub fn from_config(config: CompanyConfig) -> Result<Self> {
        let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "imitatort.db".to_string());
        let mut builder = Self::with_sqlite(&db_path)?;
        builder.config = Some(config);
        Ok(builder)
    }

    /// 使用指定路径的SQLite存储创建构建器
    pub fn with_sqlite<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let store = Arc::new(SqliteStore::new(db_path)?);
        Ok(Self {
            config: None,
            store: Some(store),
        })
    }

    /// 使用自定义存储创建构建器（高级用法）
    pub fn with_store(store: Arc<dyn Store>) -> Self {
        Self {
            config: None,
            store: Some(store),
        }
    }

    /// 设置配置
    pub fn config(mut self, config: CompanyConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// 从存储加载配置
    pub async fn load(mut self) -> Result<Self> {
        if let Some(ref store) = self.store {
            let org = store.load_organization().await?;
            if !org.agents.is_empty() {
                self.config = Some(CompanyConfig {
                    name: "Loaded Company".to_string(),
                    organization: org,
                });
            }
        }
        Ok(self)
    }

    /// 构建虚拟公司
    pub fn build(self) -> Result<VirtualCompany> {
        let config = self
            .config
            .ok_or_else(|| anyhow::anyhow!("Config not set. Use .config() or .load() first."))?;

        let store = self
            .store
            .ok_or_else(|| anyhow::anyhow!("Store not set."))?;

        Ok(VirtualCompany::with_store(config, store))
    }

    /// 构建并保存配置到存储
    pub async fn build_and_save(self) -> Result<VirtualCompany> {
        let company = self.build()?;
        company.save().await?;
        Ok(company)
    }
}
