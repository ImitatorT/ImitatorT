//! LDAP 集成模块
//!
//! 提供与 LDAP (lldap) 服务器的连接、同步和管理功能
//!
//! # 模块结构
//!
//! - `config` - LDAP 连接配置
//! - `client` - LDAP 客户端，提供 CRUD 操作
//! - `sync` - LDAP 同步服务，同步用户和组织架构
//! - `bootstrap` - LDAP 初始化引导

pub mod config;
pub mod client;
pub mod sync;
pub mod bootstrap;

pub use config::LdapConfig;
pub use client::{LdapClient, LdapUser, LdapGroup};
pub use sync::LdapSyncService;
pub use bootstrap::{LdapBootstrap, initialize_ldap};
