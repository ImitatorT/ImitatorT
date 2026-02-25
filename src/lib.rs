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
    pub mod store;
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
}

// 重新导出主要类型
pub use application::framework::{CompanyBuilder, VirtualCompany, DEFAULT_DB_PATH};
pub use core::config::CompanyConfig;
pub use core::store::{MessageFilter, Store};
pub use domain::{Agent, Department, Group, LLMConfig, Message, MessageTarget, Organization, Role};
pub use infrastructure::store::SqliteStore;
