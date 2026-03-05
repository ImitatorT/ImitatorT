//! Matrix 配置模块
//!
//! 提供 Matrix Homeserver 连接配置

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// Matrix 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    /// Homeserver URL (如：http://localhost:5141)
    pub homeserver_url: String,
    /// 服务器名称 (如：localhost)
    pub server_name: String,
    /// Appservice Token (as_token)
    pub as_token: String,
    /// Homeserver Token (hs_token)
    pub hs_token: String,
    /// Sender Localpart (如：_imitator_bot)
    pub sender_localpart: String,
    /// Appservice 监听端口
    pub appservice_port: u16,
}

impl MatrixConfig {
    /// 检查 Matrix 是否已配置
    pub fn is_configured() -> bool {
        env::var("MATRIX_HOMESERVER_URL").is_ok()
            && env::var("MATRIX_SERVER_NAME").is_ok()
            && env::var("MATRIX_AS_TOKEN").is_ok()
    }

    /// 从环境变量加载配置
    ///
    /// 必需的环境变量：
    /// - MATRIX_HOMESERVER_URL: Homeserver URL
    /// - MATRIX_SERVER_NAME: 服务器名称
    /// - MATRIX_AS_TOKEN: Appservice Token
    /// - MATRIX_HS_TOKEN: Homeserver Token
    /// - MATRIX_APPSERVICE_PORT: Appservice 端口 (可选，默认 9000)
    pub fn from_env() -> Result<Self> {
        let homeserver_url = env::var("MATRIX_HOMESERVER_URL")
            .context("MATRIX_HOMESERVER_URL not set")?;

        let server_name = env::var("MATRIX_SERVER_NAME")
            .context("MATRIX_SERVER_NAME not set")?;

        let as_token = env::var("MATRIX_AS_TOKEN")
            .context("MATRIX_AS_TOKEN not set")?;

        let hs_token = env::var("MATRIX_HS_TOKEN")
            .context("MATRIX_HS_TOKEN not set")?;

        let sender_localpart = env::var("MATRIX_SENDER_LOCALPART")
            .unwrap_or_else(|_| "_imitator_bot".to_string());

        let appservice_port = env::var("MATRIX_APPSERVICE_PORT")
            .unwrap_or_else(|_| "9000".to_string())
            .parse::<u16>()
            .context("Invalid MATRIX_APPSERVICE_PORT")?;

        Ok(Self {
            homeserver_url,
            server_name,
            as_token,
            hs_token,
            sender_localpart,
            appservice_port,
        })
    }

    /// 获取完整的用户 ID 前缀
    ///
    /// 例如：sender_localpart = "_imitator_bot", server_name = "localhost"
    /// 返回：@_imitator_bot:localhost
    pub fn sender_user_id(&self) -> String {
        format!("@{}:{}", self.sender_localpart, self.server_name)
    }

    /// 生成虚拟用户 ID
    ///
    /// 例如：localpart = "ceo", server_name = "localhost"
    /// 返回：@_ceo:localhost
    pub fn generate_user_id(&self, localpart: &str) -> String {
        let full_localpart = if localpart.starts_with('_') {
            localpart.to_string()
        } else {
            format!("_{}", localpart)
        };
        format!("@{}:{}", full_localpart, self.server_name)
    }

    /// 生成房间别名
    ///
    /// 例如：alias = "company-general"
    /// 返回：#company-general:localhost
    pub fn generate_room_alias(&self, alias: &str) -> String {
        format!("#{}:{}", alias, self.server_name)
    }

    /// 获取 Homeserver API 基础 URL
    pub fn api_base_url(&self) -> String {
        format!("{}/_matrix/client/v3", self.homeserver_url.trim_end_matches('/'))
    }

    /// 获取 Appservice API 基础 URL
    pub fn appservice_api_base_url(&self) -> String {
        format!("{}/_matrix/app/v1", self.homeserver_url.trim_end_matches('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_user_id() {
        let config = MatrixConfig {
            homeserver_url: "http://localhost:5141".to_string(),
            server_name: "localhost".to_string(),
            as_token: "test_as".to_string(),
            hs_token: "test_hs".to_string(),
            sender_localpart: "_imitator_bot".to_string(),
            appservice_port: 9000,
        };

        assert_eq!(config.generate_user_id("ceo"), "@_ceo:localhost");
        assert_eq!(config.generate_user_id("_cto"), "@_cto:localhost");
    }

    #[test]
    fn test_generate_room_alias() {
        let config = MatrixConfig {
            homeserver_url: "http://localhost:5141".to_string(),
            server_name: "localhost".to_string(),
            as_token: "test_as".to_string(),
            hs_token: "test_hs".to_string(),
            sender_localpart: "_imitator_bot".to_string(),
            appservice_port: 9000,
        };

        assert_eq!(config.generate_room_alias("company-general"), "#company-general:localhost");
    }
}
