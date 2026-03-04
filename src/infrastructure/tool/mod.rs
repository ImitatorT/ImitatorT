//! Tool 执行器实现
//!
//! 包含具体工具执行器的实现，如 FrameworkToolExecutor、web、code、file 等

// 重新导出 core 层的接口和类型（向后兼容）
pub use crate::core::tool::{ToolExecutor, ToolResult, ToolExecutorRegistry, FnToolExecutor};

pub mod framework_tools;
pub mod common;
pub mod web;
pub mod code;
pub mod file;

pub use framework_tools::{FrameworkToolExecutor, ToolEnvironment};
pub use web::{execute_web_search, execute_web_visit, execute_wikipedia, execute_reddit, execute_twitter, execute_bloomberg, execute_polymarket};
pub use code::execute_rust;
pub use file::execute_simple_file;
