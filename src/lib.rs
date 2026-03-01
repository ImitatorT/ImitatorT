//! # ImitatorT - Spring Boot Style Multi-Agent Framework
//!
//! A modern, ready-to-use multi-Agent system framework with Spring Boot design philosophy,
//! providing auto-configuration and convention-over-configuration features, allowing developers to quickly build AI Agent collaboration systems.
//!
//! ## 快速开始
//!
//! ```ignore
//! use imitatort::{quick_start, VirtualCompany, CompanyBuilder, CompanyConfig};
//!
//! // Method 1: Quick start using auto-configuration
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Auto-configure and start multi-Agent system and web service
//!     imitatort::quick_start().await?;
//!     Ok(())
//! }
//! ```
//!
//! ```ignore
//! // Method 2: Manual configuration startup
//! use imitatort::{VirtualCompany, CompanyBuilder, CompanyConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create company instance from configuration
//!     let config = CompanyConfig::test_config(); // or load from YAML file
//!     let company = CompanyBuilder::from_config(config)?
//!         .build_and_save()
//!         .await?;
//!
//!     // 启动多 Agent 系统
//!     company.run().await?;
//!
//!     Ok(())
//! }
//! ```

/// 框架引导模块 - 提供 Spring Boot 风格的自动配置启动功能
pub mod bootstrap;

/// 应用程序配置管理 - 管理运行时设置如数据库路径、网络绑定等
pub mod config;

/// 跨层错误类型定义 - 提供统一的错误处理机制
pub mod errors;

/// 领域层 - 定义核心业务实体
pub mod domain;

/// 核心层 - 提供运行时能力和基础服务
pub mod core {
    pub mod agent;
    pub mod capability;
    pub mod capability_provider;
    pub mod config;
    pub mod messaging;
    pub mod skill;
    pub mod store;
    pub mod tool;
    pub mod tool_provider;
    pub mod watchdog;
    pub mod watchdog_agent;
}

/// 应用层 - 业务逻辑编排
pub mod application {
    pub mod autonomous;
    pub mod company_runtime;
    pub mod framework;
    pub mod organization;
}

/// 基础设施层 - 外部集成和服务
pub mod infrastructure {
    pub mod auth;
    pub mod capability;
    pub mod llm;
    pub mod logger;
    pub mod store;
    pub mod tool;
    pub mod web;
}

// ================================
// 核心框架 API - 主要入口点
// ================================

/// 虚拟公司框架 - 框架的主要入口点，管理多 Agent 系统
pub use application::framework::{CompanyBuilder, VirtualCompany};

/// 公司配置 - 定义 Agent 组织架构和设置
pub use core::config::CompanyConfig;

/// 快速启动函数 - 自动配置并启动框架
pub use bootstrap::{quick_start, start_with_config, FrameworkLauncher};

// ================================
// Web 服务 API - 内置 Web 功能
// ================================

/// 启动内置 Web 服务器 - 提供 REST API 和 WebSocket 服务
pub use infrastructure::web::start_web_server;

// ================================
// 核心实体定义 - 领域模型
// ================================

/// Agent Entity - Core representation of virtual employees
pub use domain::{
    Agent, AgentId, Department, Group, GroupVisibility, LLMConfig, Message, MessageTarget,
    Organization, Role, TriggerCondition,
};

/// 用户实体 - 系统用户
pub use domain::user::User;

// ================================
// 工具和能力系统 - 扩展功能
// ================================

pub use core::tool::ToolRegistry;
pub use core::tool_provider::{CompositeToolProvider, FrameworkToolProvider, RegistryToolProvider};
/// 工具系统相关类型
pub use domain::tool::{
    CategoryNodeInfo, CategoryPath, JsonSchema, MatchType, ReturnType, Tool, ToolCallContext,
    ToolProvider,
};
pub use infrastructure::tool::{
    FrameworkToolExecutor, ToolEnvironment, ToolExecutor, ToolExecutorRegistry, ToolResult,
};

pub use core::capability::CapabilityRegistry;
pub use core::capability_provider::{
    CompositeCapabilityProvider, FrameworkCapabilityProvider, RegistryCapabilityProvider,
};
/// 能力系统相关类型
pub use domain::capability::{
    Capability, CapabilityAccessType, CapabilityCallContext, CapabilityPath, CapabilityProvider,
    SkillCapabilityBinding,
};
pub use infrastructure::capability::{
    CapabilityExecutor, CapabilityExecutorRegistry, CapabilityResult, McpClient,
    McpProtocolHandler, McpServer,
};

pub use core::skill::SkillManager;
/// 技能系统相关类型
pub use domain::skill::{BindingType, Skill, SkillToolBinding, ToolAccessType};
pub use domain::skill::{
    BindingType as DomainBindingType, Skill as DomainSkill,
    SkillToolBinding as DomainSkillToolBinding, ToolAccessType as DomainToolAccessType,
};

// ================================
// 存储和基础设施
// ================================

/// 存储系统抽象和实现
pub use core::store::{MessageFilter, Store};
pub use infrastructure::store::SqliteStore;

/// 应用程序配置 - 框架运行时配置
pub use config::AppConfig;

/// Watchdog Agent相关类型
pub use core::watchdog_agent::{ToolExecutionEvent, WatchdogAgent, WatchdogClient, WatchdogRule};

/// 错误类型定义
pub use errors::{ImitatorError, Result as ImitatorResult};
