//! LDAP 配置
//!
//! lldap 连接和认证配置

use serde::{Deserialize, Serialize};
use std::env;

/// LDAP 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapConfig {
    /// LDAP 服务器 URL（如：ldap://localhost:3890）
    pub url: String,

    /// 基础 DN（如：dc=example,dc=com）
    pub base_dn: String,

    /// 管理员绑定 DN（如：cn=admin,dc=example,dc=com）
    pub bind_dn: String,

    /// 管理员密码
    pub password: String,

    /// 用户存储 DN（如：ou=people,dc=example,dc=com）
    pub user_base_dn: String,

    /// 组存储 DN（如：ou=groups,dc=example,dc=com）
    pub group_base_dn: String,

    /// HTTP API 端口（用于 GraphQL 操作，lldap 默认 17170）
    pub http_port: u16,

    /// HTTP API 主机（用于 GraphQL 操作）
    pub http_host: String,
}

impl LdapConfig {
    /// 从环境变量加载 LDAP 配置
    pub fn from_env() -> Self {
        Self {
            url: env::var("LDAP_URL").unwrap_or_else(|_| "ldap://localhost:3890".to_string()),
            base_dn: env::var("LDAP_BASE_DN").unwrap_or_else(|_| "dc=example,dc=com".to_string()),
            bind_dn: env::var("LDAP_BIND_DN")
                .unwrap_or_else(|_| "cn=admin,dc=example,dc=com".to_string()),
            password: env::var("LDAP_PASSWORD").unwrap_or_else(|_| "admin".to_string()),
            user_base_dn: env::var("LDAP_USER_BASE_DN")
                .unwrap_or_else(|_| "ou=people,dc=example,dc=com".to_string()),
            group_base_dn: env::var("LDAP_GROUP_BASE_DN")
                .unwrap_or_else(|_| "ou=groups,dc=example,dc=com".to_string()),
            http_port: env::var("LDAP_HTTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(17170),
            http_host: env::var("LDAP_HTTP_HOST")
                .unwrap_or_else(|_| "localhost".to_string()),
        }
    }

    /// 获取 GraphQL API URL
    pub fn graphql_url(&self) -> String {
        format!("http://{}:{}/api/graphql", self.http_host, self.http_port)
    }

    /// 检查是否已配置 LDAP（URL 非默认值或有 LDAP_URL 环境变量）
    pub fn is_configured() -> bool {
        env::var("LDAP_URL").is_ok()
    }
}

impl Default for LdapConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
