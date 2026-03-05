//! LDAP 同步服务
//!
//! 负责同步用户和组织架构到 LDAP 服务器

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

use super::client::{LdapClient, LdapUser, LdapGroup};
use super::config::LdapConfig;
use crate::core::store::Store;
use crate::domain::user::{User, Position};
use crate::domain::{Organization, Agent, Department};

/// LDAP 同步服务
pub struct LdapSyncService {
    client: LdapClient,
    store: Arc<dyn Store>,
}

impl LdapSyncService {
    /// 创建新的 LDAP 同步服务
    pub fn new(client: LdapClient, store: Arc<dyn Store>) -> Self {
        Self { client, store }
    }

    /// 同步所有用户到 LDAP
    pub async fn sync_all_users(&self) -> Result<()> {
        info!("Starting LDAP user sync...");

        // 从存储加载所有用户
        match self.store.load_users().await {
            Ok(users) => {
                info!("Syncing {} users to LDAP", users.len());

                let mut created = 0;
                let mut updated = 0;
                let mut errors = 0;

                for user in &users {
                    match self.sync_user_to_ldap(user).await {
                        Ok(created_new) => {
                            if created_new {
                                created += 1;
                            } else {
                                updated += 1;
                            }
                        }
                        Err(e) => {
                            error!("Failed to sync user {}: {}", user.username, e);
                            errors += 1;
                        }
                    }
                }

                info!(
                    "User sync completed: {} created, {} updated, {} errors",
                    created, updated, errors
                );
            }
            Err(e) => {
                error!("Failed to load users: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// 同步单个用户到 LDAP
    /// 返回 true 表示创建了新用户，false 表示更新了现有用户
    pub async fn sync_user_to_ldap(&self, user: &User) -> Result<bool> {
        debug!("Syncing user {} to LDAP", user.username);

        // 将 User 转换为 LdapUser
        let ldap_user = self.user_to_ldap_user(user);

        // 查找或创建用户
        let created = self.client.find_or_create_user(&ldap_user).await?;

        if created {
            info!("Created LDAP user: {}", ldap_user.uid);
        } else {
            debug!("Updated LDAP user: {}", ldap_user.uid);
        }

        Ok(created)
    }

    /// 将 User 转换为 LdapUser
    fn user_to_ldap_user(&self, user: &User) -> LdapUser {
        let position_str = match user.position {
            Position::Chairman => "Chairman",
            Position::Management => "Management",
            Position::Employee => "Employee",
        };

        LdapUser {
            uid: user.username.clone(),
            cn: user.name.clone(),
            mail: user.email.clone(),
            employee_id: user.employee_id.clone(),
            department: user.department.clone(),
            position: position_str.to_string(),
            display_name: user.name.clone(),
            user_password: Some(user.password_hash.clone()),
        }
    }

    /// 同步组织架构到 LDAP
    pub async fn sync_organization(&self) -> Result<()> {
        info!("Starting LDAP organization sync...");

        // 从存储加载组织架构
        match self.store.load_organization().await {
            Ok(org) => {
                info!(
                    "Syncing organization: {} departments, {} agents",
                    org.departments.len(),
                    org.agents.len()
                );

                // 同步部门（作为 LDAP 组）
                for dept in &org.departments {
                    if let Err(e) = self.sync_department_to_ldap(dept, &org).await {
                        warn!("Failed to sync department {}: {}", dept.name, e);
                    }
                }

                // 同步 Agent
                for agent in &org.agents {
                    if let Err(e) = self.sync_agent_to_ldap(agent).await {
                        warn!("Failed to sync agent {}: {}", agent.name, e);
                    }
                }

                info!("Organization sync completed");
            }
            Err(e) => {
                error!("Failed to load organization: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// 同步部门到 LDAP（作为组）
    async fn sync_department_to_ldap(&self, dept: &Department, org: &Organization) -> Result<()> {
        debug!("Syncing department {} to LDAP", dept.name);

        // 获取部门成员
        let members = org
            .agents
            .iter()
            .filter(|a| a.department_id.as_ref() == Some(&dept.id))
            .collect::<Vec<_>>();

        // 构建组成员 DN 列表
        let member_dns: Vec<String> = members
            .iter()
            .map(|a| format!("uid={},{}", a.id, self.client.config().user_base_dn))
            .collect();

        // 检查组是否存在
        match self.client.find_group(&dept.name).await {
            Ok(Some(_existing)) => {
                // 组存在，更新成员
                self.client
                    .update_group_members(&dept.name, &member_dns)
                    .await?;
                debug!("Updated LDAP group: {}", dept.name);
            }
            Ok(None) => {
                // 组不存在，创建
                let ldap_group = LdapGroup {
                    cn: dept.name.clone(),
                    members: member_dns,
                    description: Some(format!("Department: {}", dept.name)),
                };
                self.client.create_group(&ldap_group).await?;
                info!("Created LDAP group: {}", dept.name);
            }
            Err(e) => {
                warn!("Failed to check group {}: {}", dept.name, e);
            }
        }

        Ok(())
    }

    /// 同步 Agent 到 LDAP
    async fn sync_agent_to_ldap(&self, agent: &Agent) -> Result<()> {
        debug!("Syncing agent {} to LDAP", agent.name);

        // 将 Agent 转换为 LdapUser
        let ldap_user = LdapUser {
            uid: agent.id.clone(),
            cn: agent.name.clone(),
            mail: None,
            employee_id: agent.id.clone(),
            department: agent.department_id.clone().unwrap_or_default(),
            position: agent.role.title.clone(),
            display_name: agent.name.clone(),
            user_password: None,
        };

        // 查找或创建
        self.client.find_or_create_user(&ldap_user).await?;

        Ok(())
    }

    /// 添加用户到部门组
    pub async fn add_user_to_department(&self, user_id: &str, dept_name: &str) -> Result<()> {
        let user_dn = format!("uid={},{}", user_id, self.client.config().user_base_dn);
        self.client.add_member_to_group(dept_name, &user_dn).await
    }

    /// 从部门组移除用户
    pub async fn remove_user_from_department(&self, user_id: &str, dept_name: &str) -> Result<()> {
        let user_dn = format!("uid={},{}", user_id, self.client.config().user_base_dn);
        self.client.remove_member_from_group(dept_name, &user_dn).await
    }

    /// 清理所有测试数据
    pub async fn cleanup_test_data(&self) -> Result<()> {
        info!("Cleaning up LDAP test data...");

        // 删除所有组
        match self.client.list_groups().await {
            Ok(groups) => {
                for group in &groups {
                    if let Err(e) = self.client.delete_group(&group.cn).await {
                        warn!("Failed to delete group {}: {}", group.cn, e);
                    } else {
                        info!("Deleted group: {}", group.cn);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to list groups: {}", e);
            }
        }

        // 删除所有用户
        match self.client.list_users().await {
            Ok(users) => {
                for user in &users {
                    if let Err(e) = self.client.delete_user(&user.uid).await {
                        warn!("Failed to delete user {}: {}", user.uid, e);
                    } else {
                        info!("Deleted user: {}", user.uid);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to list users: {}", e);
            }
        }

        info!("LDAP test data cleanup completed");
        Ok(())
    }
}
