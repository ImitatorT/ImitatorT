//! 核心层
//!
//! 提供Agent运行时和消息通信能力

pub mod agent;
pub mod config;
pub mod messaging;
pub mod skill;
pub mod state;
pub mod store;
pub mod tool;
pub mod tool_provider;
pub mod watchdog;
pub mod watchdog_agent;

// Re-export important types from watchdog for easy access
pub use watchdog::TriggerCondition;
