//! ImitatorT - 多Agent公司模拟框架
//!
//! 基于Rust的轻量级框架，让多个AI Agent像真人一样在公司中协作。

// 应用程序配置管理 (AppConfig) - 管理运行时设置如数据库路径、网络绑定等
pub mod config;

// 跨层错误类型定义 - 提供统一的错误处理机制
pub mod errors;

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
    pub mod company_runtime;
    pub mod framework;
    pub mod organization;
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
pub use application::framework::{CompanyBuilder, VirtualCompany};
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
pub use config::AppConfig;
pub use errors::{ImitatorError, Result as ImitatorResult};

/// 测试工具 - 仅在测试环境下可用
#[cfg(test)]
pub use test_utils::TestHelper;
