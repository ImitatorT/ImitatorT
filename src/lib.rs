//! ImitatorT - 多Agent公司模拟框架
//!
//! 基于Rust的轻量级框架，让多个AI Agent像真人一样在公司中协作。

// 领域层
pub mod domain;

// 核心层
pub mod core {
    pub mod agent;
    pub mod config;
    pub mod messaging;
    pub mod skill;
    pub mod store;
    pub mod tool;
    pub mod tool_provider;
    pub mod capability;
    pub mod capability_provider;
}

// 应用层
pub mod application {
    pub mod autonomous;
    pub mod framework;
}

// 基础设施层
pub mod infrastructure {
    pub mod llm;
    pub mod logger;
    pub mod store;
    pub mod web;
    pub mod tool;
    pub mod capability;
    pub mod auth;
}

// 重新导出主要类型
pub use application::framework::{CompanyBuilder, VirtualCompany, DEFAULT_DB_PATH};
pub use core::config::CompanyConfig;
pub use core::skill::SkillManager;
pub use domain::skill::{Skill, SkillToolBinding, BindingType, ToolAccessType};
pub use core::store::{MessageFilter, Store};
pub use core::tool::ToolRegistry;
pub use core::capability::CapabilityRegistry;
pub use domain::{Agent, AgentId, Department, Group, LLMConfig, Message, MessageTarget, Organization, Role};
pub use domain::user::User;
pub use domain::skill::{Skill as DomainSkill, SkillToolBinding as DomainSkillToolBinding, BindingType as DomainBindingType, ToolAccessType as DomainToolAccessType};
pub use domain::tool::{Tool, CategoryPath, ReturnType, ToolProvider, MatchType, CategoryNodeInfo, ToolCallContext, JsonSchema};
pub use domain::capability::{Capability, CapabilityPath, CapabilityProvider, CapabilityAccessType, SkillCapabilityBinding, CapabilityCallContext};
pub use infrastructure::store::SqliteStore;
pub use infrastructure::web::start_web_server;
pub use infrastructure::tool::{ToolExecutor, ToolResult, ToolExecutorRegistry, FrameworkToolExecutor, ToolEnvironment};
pub use infrastructure::capability::{CapabilityExecutor, CapabilityResult, CapabilityExecutorRegistry, McpServer, McpClient, McpProtocolHandler};
pub use core::tool_provider::{CompositeToolProvider, FrameworkToolProvider, RegistryToolProvider};
pub use core::capability_provider::{CompositeCapabilityProvider, FrameworkCapabilityProvider, RegistryCapabilityProvider};
