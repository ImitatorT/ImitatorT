//! 公司运行时组件
//!
//! 将 VirtualCompany 的职责分解为更小的组件

use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::core::config::CompanyConfig;
use crate::core::messaging::MessageBus;
use crate::core::store::Store;
use crate::core::tool::ToolRegistry;
use crate::core::capability::CapabilityRegistry;
use crate::domain::Organization;
use crate::infrastructure::tool::{FrameworkToolExecutor, ToolEnvironment};
use crate::infrastructure::capability::{McpServer, McpProtocolHandler};
use super::autonomous::AutonomousAgent;

/// 组织架构管理器
pub struct OrganizationManager {
    organization: Arc<RwLock<Organization>>,
    config: CompanyConfig,
}

impl OrganizationManager {
    pub fn new(config: CompanyConfig) -> Self {
        let organization = Arc::new(RwLock::new(config.organization.clone()));
        Self {
            organization,
            config,
        }
    }

    /// 获取组织架构（异步读取）
    pub async fn organization(&self) -> tokio::sync::RwLockReadGuard<'_, Organization> {
        self.organization.read().await
    }

    /// 获取组织架构引用
    pub fn organization_arc(&self) -> Arc<RwLock<Organization>> {
        self.organization.clone()
    }

    /// 获取配置引用
    pub fn config(&self) -> &CompanyConfig {
        &self.config
    }
}

/// Agent 管理器
pub struct AgentManager {
    agents: DashMap<String, AutonomousAgent>,
    message_bus: Arc<MessageBus>,
}

impl AgentManager {
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        Self {
            agents: DashMap::new(),
            message_bus,
        }
    }

    /// 初始化所有 Agent
    pub async fn initialize_agents(&self, organization: &Organization) -> Result<()> {
        for agent_data in &organization.agents {
            let agent = AutonomousAgent::new(agent_data.clone(), self.message_bus.clone()).await?;
            let agent_id = agent.id().to_string();
            self.agents.insert(agent_id.clone(), agent);
            info!("Created agent: {}", agent_id);
        }
        Ok(())
    }

    /// 启动所有 Agent 的自主循环
    pub async fn start_agent_loops(&self) -> Result<Vec<tokio::task::JoinHandle<()>>> {
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

        Ok(handles)
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

    /// 获取所有 Agent 列表（用于 Web API）
    /// 注意：由于Agent数据存储在Organization中，这里返回空向量
    /// 实际的Agent列表应通过OrganizationManager获取
    pub async fn get_agents(&self) -> Result<Vec<crate::domain::Agent>> {
        Ok(vec![])
    }
}

/// 工具和功能管理器
pub struct ToolCapabilityManager {
    tool_registry: Arc<ToolRegistry>,
    capability_registry: Arc<CapabilityRegistry>,
}

impl ToolCapabilityManager {
    pub fn new() -> Self {
        Self {
            tool_registry: Arc::new(ToolRegistry::new()),
            capability_registry: Arc::new(CapabilityRegistry::new()),
        }
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
    pub fn create_tool_environment(
        &self,
        message_bus: Arc<MessageBus>,
        organization: Arc<RwLock<Organization>>,
        store: Arc<dyn Store>,
    ) -> ToolEnvironment {
        ToolEnvironment::new(
            message_bus,
            organization,
            self.tool_registry.clone(),
            store,
        )
    }

    /// 获取框架工具执行器
    pub fn get_framework_tool_executor(
        &self,
        message_bus: Arc<MessageBus>,
        organization: Arc<RwLock<Organization>>,
        store: Arc<dyn Store>,
    ) -> FrameworkToolExecutor {
        let env = self.create_tool_environment(message_bus, organization, store);
        FrameworkToolExecutor::new(env)
    }

    /// 获取 CapabilityRegistry 引用
    pub fn capability_registry(&self) -> Arc<CapabilityRegistry> {
        self.capability_registry.clone()
    }

    /// 注册应用自定义功能
    pub async fn register_app_capability(&self, capability: crate::domain::capability::Capability) -> Result<()> {
        let cap_id = capability.id.clone();
        self.capability_registry.register(capability).await?;
        info!("Registered app capability: {}", cap_id);
        Ok(())
    }

    /// 创建 MCP 服务器
    pub fn create_mcp_server(&self, bind_addr: String) -> McpServer {
        McpServer::new(bind_addr, self.capability_registry.clone())
    }

    /// 获取 MCP 协议处理器
    pub fn get_mcp_protocol_handler(&self) -> McpProtocolHandler {
        McpProtocolHandler::new(self.capability_registry.clone())
    }
}