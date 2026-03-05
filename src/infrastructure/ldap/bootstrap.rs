//! LDAP 引导模块
//!
//! 负责应用启动时的 LDAP 初始化和同步

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn, error};

use super::client::LdapClient;
use super::config::LdapConfig;
use super::sync::LdapSyncService;
use crate::core::store::Store;
use crate::infrastructure::matrix::{MatrixConfig, TuwunelClient};

/// LDAP 引导服务
pub struct LdapBootstrap {
    client: LdapClient,
    sync_service: LdapSyncService,
    store: Arc<dyn Store>,
}

impl LdapBootstrap {
    /// 创建新的 LDAP 引导服务
    pub fn new(config: LdapConfig, store: Arc<dyn Store>) -> Self {
        let client = LdapClient::new(config.clone());
        let sync_service = LdapSyncService::new(client.clone(), store.clone());

        Self {
            client,
            sync_service,
            store,
        }
    }

    /// 初始化 LDAP 同步
    ///
    /// 执行流程：
    /// 1. 从存储加载所有用户
    /// 2. 从存储加载组织架构
    /// 3. 对每个用户：检查 LDAP 是否存在（通过 employee_id + 组织架构匹配）
    /// 4. 不存在则创建，字段不一致则更新
    /// 5. 同步组织架构（部门 -> LDAP Groups）
    pub async fn initialize(&self) -> Result<()> {
        info!("🔄 Initializing LDAP synchronization...");

        // 检查 LDAP 是否已配置
        if !LdapConfig::is_configured() {
            warn!("LDAP not configured, skipping initialization");
            return Ok(());
        }

        // 1. 同步所有用户
        if let Err(e) = self.sync_service.sync_all_users().await {
            error!("Failed to sync users: {}", e);
            return Err(e);
        }

        // 2. 同步组织架构
        if let Err(e) = self.sync_service.sync_organization().await {
            error!("Failed to sync organization: {}", e);
            return Err(e);
        }

        // 3. 同步到 Tuwunel Matrix（如果已配置）
        if MatrixConfig::is_configured() {
            info!("🔄 Syncing to Tuwunel Matrix...");
            if let Err(e) = self.sync_to_tuwunel().await {
                error!("Failed to sync to Tuwunel: {}", e);
                // 不阻断主流程，仅记录错误
            }
        }

        info!("✅ LDAP and Matrix synchronization completed successfully");
        Ok(())
    }

    /// 同步单个用户（用于运行时新增用户）
    pub async fn sync_user(&self, user: &crate::domain::user::User) -> Result<bool> {
        self.sync_service.sync_user_to_ldap(user).await
    }

    /// 清理测试数据
    pub async fn cleanup_test_data(&self) -> Result<()> {
        info!("🧹 Cleaning up LDAP test data...");
        self.sync_service.cleanup_test_data().await
    }

    /// 获取 LDAP 客户端
    pub fn client(&self) -> &LdapClient {
        &self.client
    }

    /// 获取同步服务
    pub fn sync_service(&self) -> &LdapSyncService {
        &self.sync_service
    }

    /// 同步所有 Agent 到 Tuwunel Matrix
    async fn sync_to_tuwunel(&self) -> Result<()> {
        let matrix_config = MatrixConfig::from_env()?;
        let tuwunel_client = TuwunelClient::new(&matrix_config);

        // 加载组织架构
        let org = self.store.load_organization().await?;

        // 为每个 Agent 注册虚拟用户
        for agent in &org.agents {
            let localpart = format!("agent_{}", agent.id);
            tuwunel_client
                .register_virtual_user(&localpart, &agent.name)
                .await?;
        }

        info!("✅ Synced {} agents to Tuwunel Matrix", org.agents.len());
        Ok(())
    }
}

/// 快速初始化 LDAP
pub async fn initialize_ldap(store: Arc<dyn Store>) -> Result<Option<LdapBootstrap>> {
    if !LdapConfig::is_configured() {
        info!("LDAP not configured, skipping initialization");
        return Ok(None);
    }

    let config = LdapConfig::from_env();
    let bootstrap = LdapBootstrap::new(config, store);
    bootstrap.initialize().await?;

    Ok(Some(bootstrap))
}
