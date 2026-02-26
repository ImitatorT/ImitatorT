//! 虚拟公司框架 API
//!
//! 框架主入口：VirtualCompany

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info};

use crate::core::config::CompanyConfig;
use crate::core::messaging::MessageBus;
use crate::core::store::Store;
use crate::core::tool::ToolRegistry;
use crate::domain::{Message, Organization};
use crate::infrastructure::store::SqliteStore;
use crate::infrastructure::tool::{FrameworkToolExecutor, ToolEnvironment};

use super::autonomous::AutonomousAgent;

/// 默认数据库路径
pub const DEFAULT_DB_PATH: &str = "imitatort.db";

/// 虚拟公司框架
///
/// 封装所有框架能力，提供简洁的 API
pub struct VirtualCompany {
    config: CompanyConfig,
    organization: Arc<RwLock<Organization>>,
    agents: DashMap<String, AutonomousAgent>,
    message_bus: Arc<MessageBus>,
    message_tx: broadcast::Sender<Message>,
    store: Arc<dyn Store>,
    tool_registry: Arc<ToolRegistry>,
}

impl VirtualCompany {
    /// 从配置创建虚拟公司，使用默认SQLite存储
    pub fn from_config(config: CompanyConfig) -> Result<Self> {
        Self::with_sqlite(config, DEFAULT_DB_PATH)
    }

    /// 从配置创建虚拟公司，使用指定路径的SQLite存储
    pub fn with_sqlite<P: AsRef<Path>>(config: CompanyConfig, db_path: P) -> Result<Self> {
        let store = Arc::new(SqliteStore::new(db_path)?);
        Ok(Self::with_store(config, store))
    }

    /// 从配置创建虚拟公司，使用指定的存储
    pub fn with_store(config: CompanyConfig, store: Arc<dyn Store>) -> Self {
        let message_bus = Arc::new(MessageBus::new());
        let (message_tx, _) = broadcast::channel(1000);
        let tool_registry = Arc::new(ToolRegistry::new());

        Self {
            organization: Arc::new(RwLock::new(config.organization.clone())),
            config,
            agents: DashMap::new(),
            message_bus,
            message_tx,
            store,
            tool_registry,
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
        let org = self.organization.read().await;
        self.store.save_organization(&*org).await?;
        info!("Company state saved successfully");
        Ok(())
    }

    /// 初始化并启动公司
    pub async fn run(&self) -> Result<()> {
        info!("Starting virtual company: {}", self.config.name);

        // 1. 创建所有Agent
        let org = self.organization.read().await;
        let agents_to_create: Vec<_> = org.agents.clone();
        drop(org); // 释放读锁

        for agent in agents_to_create {
            let agent = AutonomousAgent::new(agent, self.message_bus.clone()).await?;

            let agent_id = agent.id().to_string();
            self.agents.insert(agent_id.clone(), agent);
            info!("Created agent: {}", agent_id);
        }

        info!("All {} agents initialized", self.agents.len());

        // 2. 启动所有Agent的自主循环
        let mut handles = vec![];

        for agent_ref in self.agents.iter() {
            let agent = agent_ref.value().clone();
            let handle = tokio::spawn(async move {
                if let Err(e) = agent.run_loop().await {
                    error!("Agent {} error: {}", agent.id(), e);
                }
            });
            handles.push(handle);
        }

        info!("All agents started, company is running...");

        // 3. 等待所有Agent（实际上不会结束）
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    /// 获取消息流（用于外部监听）
    pub fn subscribe_messages(&self) -> broadcast::Receiver<Message> {
        self.message_tx.subscribe()
    }

    /// 手动触发任务给指定Agent
    pub fn assign_task(&self, agent_id: &str, task: impl Into<String>) -> Result<()> {
        if let Some(agent) = self.agents.get(agent_id) {
            agent.assign_task(task)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Agent not found: {}", agent_id))
        }
    }

    /// 获取组织架构（异步读取）
    pub async fn organization(&self) -> tokio::sync::RwLockReadGuard<'_, Organization> {
        self.organization.read().await
    }

    /// 获取组织架构引用（同步，仅用于需要 &Organization 的场景）
    pub fn organization_arc(&self) -> Arc<RwLock<Organization>> {
        self.organization.clone()
    }

    /// 获取存储引用
    pub fn store(&self) -> &Arc<dyn Store> {
        &self.store
    }

    /// 获取所有 Agent 列表（用于 Web API）
    pub async fn get_agents(&self) -> Result<Vec<crate::domain::Agent>> {
        let org = self.organization.read().await;
        Ok(org.agents.clone())
    }

    /// 获取 ToolRegistry 引用
    pub fn tool_registry(&self) -> Arc<ToolRegistry> {
        self.tool_registry.clone()
    }

    /// 注册应用自定义工具
    pub async fn register_app_tool(&self, tool: crate::domain::tool::Tool) -> Result<()> {
        let tool_id = tool.id.clone();
        self.tool_registry.register(tool).await?;
        info!("Registered app tool: {}", tool_id);
        Ok(())
    }

    /// 创建工具执行环境
    pub fn create_tool_environment(&self) -> ToolEnvironment {
        ToolEnvironment::new(
            self.message_bus.clone(),
            self.organization.clone(),
            self.tool_registry.clone(),
        )
    }

    /// 获取框架工具执行器
    pub fn get_framework_tool_executor(&self) -> FrameworkToolExecutor {
        let env = self.create_tool_environment();
        FrameworkToolExecutor::new(env)
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
        Self::with_sqlite(DEFAULT_DB_PATH)
    }

    /// 从配置创建，使用默认SQLite路径
    pub fn from_config(config: CompanyConfig) -> Result<Self> {
        let mut builder = Self::with_sqlite(DEFAULT_DB_PATH)?;
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
