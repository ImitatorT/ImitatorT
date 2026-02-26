//! 应用程序配置
//!
//! 管理所有可配置的参数和默认值

use serde::{Deserialize, Serialize};
use std::env;

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 数据库路径
    pub db_path: String,

    /// Web 服务器绑定地址
    pub web_bind: String,

    /// 输出模式 (cli 或 web)
    pub output_mode: String,

    /// 消息广播通道容量
    pub message_channel_capacity: usize,

    /// 默认 API 基础 URL
    pub default_api_base_url: String,

    /// 默认模型名称
    pub default_model: String,

    /// 日志级别
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            db_path: get_env_or_default("DB_PATH", "imitatort.db".to_string()),
            web_bind: get_env_or_default("WEB_BIND", "0.0.0.0:8080".to_string()),
            output_mode: get_env_or_default("OUTPUT_MODE", "cli".to_string()),
            message_channel_capacity: get_env_or_default("MESSAGE_CHANNEL_CAPACITY", 1000usize),
            default_api_base_url: get_env_or_default("DEFAULT_API_BASE_URL", "https://api.openai.com/v1".to_string()),
            default_model: get_env_or_default("DEFAULT_MODEL", "gpt-4o-mini".to_string()),
            log_level: get_env_or_default("LOG_LEVEL", "info".to_string()),
        }
    }
}

impl AppConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self::default()
    }

    /// 从配置文件加载配置
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 获取配置，优先从文件加载，如果失败则使用环境变量或默认值
    pub fn load(config_path: Option<&str>) -> Self {
        match config_path {
            Some(path) => Self::from_file(path).unwrap_or_else(|_| Self::from_env()),
            None => Self::from_env(),
        }
    }
}

/// 辅助函数：从环境变量获取值，如果不存在则返回默认值
fn get_env_or_default<T: std::str::FromStr + Default>(key: &str, default: T) -> T
where
    T: std::str::FromStr,
{
    env::var(key)
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(default)
}