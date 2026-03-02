//! 标准化错误处理
//!
//! 定义项目专用的错误类型

use thiserror::Error;

/// 项目主要错误类型
#[derive(Error, Debug)]
pub enum ImitatorError {
    /// 存储相关错误
    #[error("Storage error: {0}")]
    StorageError(String),

    /// 消息传递错误
    #[error("Messaging error: {0}")]
    MessagingError(String),

    /// Agent 相关错误
    #[error("Agent error: {0}")]
    AgentError(String),

    /// 配置错误
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// 网络请求错误
    #[error("Network error: {0}")]
    NetworkError(String),

    /// LLM 服务错误
    #[error("LLM service error: {0}")]
    LlmError(String),

    /// 工具执行错误
    #[error("Tool execution error: {0}")]
    ToolError(String),

    /// 功能执行错误
    #[error("Capability execution error: {0}")]
    CapabilityError(String),

    /// 输入验证错误
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// 权限错误
    #[error("Permission error: {0}")]
    PermissionError(String),

    /// 未知错误
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<anyhow::Error> for ImitatorError {
    fn from(err: anyhow::Error) -> Self {
        ImitatorError::Unknown(err.to_string())
    }
}

impl From<std::io::Error> for ImitatorError {
    fn from(err: std::io::Error) -> Self {
        ImitatorError::StorageError(err.to_string())
    }
}

impl From<serde_json::Error> for ImitatorError {
    fn from(err: serde_json::Error) -> Self {
        ImitatorError::ConfigError(err.to_string())
    }
}

impl From<serde_yaml::Error> for ImitatorError {
    fn from(err: serde_yaml::Error) -> Self {
        ImitatorError::ConfigError(err.to_string())
    }
}

/// 项目结果类型别名
pub type Result<T> = std::result::Result<T, ImitatorError>;
