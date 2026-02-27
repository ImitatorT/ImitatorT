//! Application Configuration
//!
//! Manages all configurable parameters and default values

use serde::{Deserialize, Serialize};
use std::env;

/// Application Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Database path
    pub db_path: String,

    /// Web server binding address
    pub web_bind: String,

    /// Output mode (cli or web)
    pub output_mode: String,

    /// Message broadcast channel capacity
    pub message_channel_capacity: usize,

    /// Default API base URL
    pub default_api_base_url: String,

    /// Default model name
    pub default_model: String,

    /// Log level
    pub log_level: String,

    /// Whether to run Agent autonomous loops (in web mode)
    pub run_agent_loops: bool,
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
            run_agent_loops: get_env_or_default("RUN_AGENT_LOOPS", true), // Default to run agent loops, maintaining backward compatibility
        }
    }
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self::default()
    }

    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Get configuration, prioritizing file loading, falling back to environment variables or defaults if failed
    pub fn load(config_path: Option<&str>) -> Self {
        match config_path {
            Some(path) => Self::from_file(path).unwrap_or_else(|_| Self::from_env()),
            None => Self::from_env(),
        }
    }
}

/// Helper function: get value from environment variable, return default if not exists
fn get_env_or_default<T: std::str::FromStr + Default>(key: &str, default: T) -> T
where
    T: std::str::FromStr,
{
    env::var(key)
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(default)
}