//! LDAP 客户端
//!
//! 封装 ldap3 库和 GraphQL API，提供异步 LDAP 操作
//!
//! 注意：lldap 不支持通过 LDAP 协议进行组成员修改操作，
//! 组成员管理需要使用 GraphQL API

use anyhow::{Context, Result};
use ldap3::{
    Ldap, LdapConnAsync, Mod, Scope, SearchEntry, drive,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{debug, info, warn};

use super::config::LdapConfig;

/// LDAP 用户条目
#[derive(Debug, Clone)]
pub struct LdapUser {
    pub uid: String,
    pub cn: String,
    pub mail: Option<String>,
    pub employee_id: String,
    pub department: String,
    pub position: String,
    pub display_name: String,
    pub user_password: Option<String>,
}

/// LDAP 组条目
#[derive(Debug, Clone)]
pub struct LdapGroup {
    pub cn: String,
    pub members: Vec<String>,
    pub description: Option<String>,
}

/// GraphQL API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphQlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphQlError {
    message: String,
}

/// GraphQL 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphQlUser {
    id: String,
    email: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

/// GraphQL 组信息（简化版，用于列表）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphQlGroupSimple {
    id: i64,
    #[serde(rename = "displayName")]
    display_name: String,
}

/// LDAP 客户端 - 线程安全封装
///
/// 使用 LDAP 协议进行用户/组查询和创建
/// 使用 GraphQL API 进行组成员管理（lldap 限制）
#[derive(Clone)]
pub struct LdapClient {
    config: LdapConfig,
    http_client: Client,
}

impl LdapClient {
    /// 创建新的 LDAP 客户端
    pub fn new(config: LdapConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
        }
    }

    /// 获取配置引用
    pub fn config(&self) -> &LdapConfig {
        &self.config
    }

    /// 执行 GraphQL 查询
    async fn graphql_query<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
    ) -> Result<T> {
        let graphql_url = self.config.graphql_url();

        // 首先登录获取 session
        let login_query = r#"
            mutation Login {
                login(input: {username: "admin", password: "gdsc90zzy"}) {
                    redirect
                }
            }
        "#;

        let login_response = self.http_client
            .post(&graphql_url)
            .json(&serde_json::json!({
                "query": login_query
            }))
            .send()
            .await
            .context("Failed to send login request")?;

        // 获取 cookie
        let cookies = login_response
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // 执行实际查询
        let response = self.http_client
            .post(&graphql_url)
            .json(&serde_json::json!({
                "query": query
            }))
            .header(
                reqwest::header::COOKIE,
                cookies.unwrap_or_default(),
            )
            .send()
            .await
            .context("Failed to send GraphQL request")?;

        let graphql_response: GraphQlResponse<T> = response
            .json()
            .await
            .context("Failed to parse GraphQL response")?;

        if let Some(errors) = graphql_response.errors {
            let error_msgs: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
            return Err(anyhow::anyhow!("GraphQL errors: {:?}", error_msgs));
        }

        graphql_response
            .data
            .ok_or_else(|| anyhow::anyhow!("GraphQL response has no data"))
    }

    /// 创建并绑定连接 - 返回 Ldap handle
    async fn connect(&self) -> Result<Ldap> {
        let (conn, mut ldap) = LdapConnAsync::new(&self.config.url)
            .await
            .context("Failed to connect to LDAP server")?;

        drive!(conn);

        let result = ldap
            .simple_bind(&self.config.bind_dn, &self.config.password)
            .await
            .context("Failed to bind to LDAP")?;

        result
            .success()
            .context("LDAP bind failed")?;

        Ok(ldap)
    }

    /// 查找用户（通过 employee_id，实际使用 uid 匹配，因为 lldap 不支持 employeeID 属性）
    pub async fn find_user_by_employee_id(&self, employee_id: &str) -> Result<Option<LdapUser>> {
        // 使用 uid 进行搜索（lldap 不支持 employeeID 属性）
        self.find_user_by_uid(employee_id).await
    }

    /// 查找用户（通过 uid/username）
    pub async fn find_user_by_uid(&self, uid: &str) -> Result<Option<LdapUser>> {
        let mut ldap = self.connect().await?;

        let filter = format!("(uid={})", uid);

        let (rs, _) = ldap
            .search(
                &self.config.user_base_dn,
                Scope::Subtree,
                &filter,
                vec!["uid", "cn", "mail", "employeeID", "department", "title", "displayName"],
            )
            .await
            .context("LDAP search failed")?
            .success()
            .context("LDAP search result error")?;

        ldap.unbind().await.ok();

        if rs.is_empty() {
            return Ok(None);
        }

        let entry = SearchEntry::construct(rs[0].clone());
        Ok(Some(self.search_entry_to_ldap_user(&entry)))
    }

    /// 将 SearchEntry 转换为 LdapUser
    fn search_entry_to_ldap_user(&self, entry: &SearchEntry) -> LdapUser {
        let attrs = &entry.attrs;
        LdapUser {
            uid: self.get_attr(attrs, "uid").unwrap_or_default(),
            cn: self.get_attr(attrs, "cn").unwrap_or_default(),
            mail: self.get_attr(attrs, "mail"),
            employee_id: self.get_attr(attrs, "employeeID").unwrap_or_default(),
            department: self.get_attr(attrs, "department").unwrap_or_default(),
            position: self.get_attr(attrs, "title").unwrap_or_default(),
            display_name: self.get_attr(attrs, "displayName").unwrap_or_default(),
            user_password: None,
        }
    }

    /// 从属性 Map 中获取字符串值
    fn get_attr(&self, attrs: &std::collections::HashMap<String, Vec<String>>, key: &str) -> Option<String> {
        attrs
            .get(key)
            .and_then(|v| v.first())
            .cloned()
    }

    /// 创建用户
    pub async fn create_user(&self, user: &LdapUser) -> Result<()> {
        let mut ldap = self.connect().await?;

        let user_dn = format!("uid={},{}", user.uid, self.config.user_base_dn);

        let mut attrs: Vec<(&str, HashSet<&str>)> = Vec::new();

        // lldap 使用特定的 objectClass，不支持 "top"、"organizationalPerson" 等
        let mut obj_classes: HashSet<&str> = HashSet::new();
        obj_classes.insert("person");
        obj_classes.insert("inetOrgPerson");
        attrs.push(("objectClass", obj_classes));

        let mut uid_set: HashSet<&str> = HashSet::new();
        uid_set.insert(&user.uid);
        attrs.push(("uid", uid_set));

        let mut cn_set: HashSet<&str> = HashSet::new();
        cn_set.insert(&user.cn);
        attrs.push(("cn", cn_set));

        let mut sn_set: HashSet<&str> = HashSet::new();
        sn_set.insert(&user.cn);
        attrs.push(("sn", sn_set));

        // mail 属性在 lldap 中必须唯一，如果未提供则使用 uid 生成
        let mail_to_use = user.mail.clone().unwrap_or_else(|| format!("{}@local", user.uid));
        let mut mail_set: HashSet<&str> = HashSet::new();
        mail_set.insert(&mail_to_use);
        attrs.push(("mail", mail_set));

        // lldap 不支持 userPassword 通过 LDAP 协议设置，需要使用 GraphQL API 或管理界面
        // 不支持 displayName、department、title 等属性

        let result = ldap
            .add(&user_dn, attrs)
            .await
            .context("LDAP add failed")?;

        result.success().context("LDAP add result error")?;

        ldap.unbind().await.ok();

        info!("Created LDAP user: {}", user_dn);
        Ok(())
    }

    /// 更新用户
    pub async fn update_user(&self, user: &LdapUser) -> Result<()> {
        let mut ldap = self.connect().await?;

        let user_dn = format!("uid={},{}", user.uid, self.config.user_base_dn);

        let mut mods: Vec<Mod<&str>> = Vec::new();

        let mut cn_set: HashSet<&str> = HashSet::new();
        cn_set.insert(&user.cn);
        mods.push(Mod::Replace("cn", cn_set));

        let mut dn_set: HashSet<&str> = HashSet::new();
        dn_set.insert(&user.display_name);
        mods.push(Mod::Replace("displayName", dn_set));

        let mut dept_set: HashSet<&str> = HashSet::new();
        dept_set.insert(&user.department);
        mods.push(Mod::Replace("department", dept_set));

        // lldap 不支持 employeeID 属性，跳过

        if let Some(ref mail) = user.mail {
            let mut mail_set: HashSet<&str> = HashSet::new();
            mail_set.insert(mail.as_str());
            mods.push(Mod::Replace("mail", mail_set));
        }

        if !user.position.is_empty() {
            let mut title_set: HashSet<&str> = HashSet::new();
            title_set.insert(&user.position);
            mods.push(Mod::Replace("title", title_set));
        }

        let result = ldap
            .modify(&user_dn, mods)
            .await
            .context("LDAP modify failed")?;

        result.success().context("LDAP modify result error")?;

        ldap.unbind().await.ok();

        info!("Updated LDAP user: {}", user_dn);
        Ok(())
    }

    /// 查找或创建用户
    pub async fn find_or_create_user(&self, user: &LdapUser) -> Result<bool> {
        if let Some(_existing) = self.find_user_by_employee_id(&user.employee_id).await? {
            self.update_user(user).await?;
            Ok(false)
        } else {
            self.create_user(user).await?;
            Ok(true)
        }
    }

    /// 删除用户
    pub async fn delete_user(&self, uid: &str) -> Result<()> {
        let mut ldap = self.connect().await?;

        let user_dn = format!("uid={},{}", uid, self.config.user_base_dn);

        let result = ldap
            .delete(&user_dn)
            .await
            .context("LDAP delete failed")?;

        result.success().context("LDAP delete result error")?;

        ldap.unbind().await.ok();

        info!("Deleted LDAP user: {}", user_dn);
        Ok(())
    }

    /// 查找组
    pub async fn find_group(&self, group_name: &str) -> Result<Option<LdapGroup>> {
        let mut ldap = self.connect().await?;

        let filter = format!("(cn={})", group_name);

        // lldap 使用 "uniqueMember" 属性
        let (rs, _) = ldap
            .search(
                &self.config.group_base_dn,
                Scope::Subtree,
                &filter,
                vec!["cn", "uniqueMember", "description"],
            )
            .await
            .context("LDAP search failed")?
            .success()
            .context("LDAP search result error")?;

        ldap.unbind().await.ok();

        if rs.is_empty() {
            return Ok(None);
        }

        let entry = SearchEntry::construct(rs[0].clone());
        let attrs = &entry.attrs;

        let members = attrs
            .get("uniqueMember")
            .cloned()
            .unwrap_or_default();

        Ok(Some(LdapGroup {
            cn: self.get_attr(attrs, "cn").unwrap_or_default(),
            members,
            description: self.get_attr(attrs, "description"),
        }))
    }

    /// 创建组
    pub async fn create_group(&self, group: &LdapGroup) -> Result<()> {
        let mut ldap = self.connect().await?;

        let group_dn = format!("cn={},{}", group.cn, self.config.group_base_dn);

        let mut attrs: Vec<(&str, HashSet<&str>)> = Vec::new();

        // lldap 仅支持 groupOfUniqueNames objectClass，不支持 "top"
        let mut obj_classes: HashSet<&str> = HashSet::new();
        obj_classes.insert("groupOfUniqueNames");
        attrs.push(("objectClass", obj_classes));

        let mut cn_set: HashSet<&str> = HashSet::new();
        cn_set.insert(&group.cn);
        attrs.push(("cn", cn_set));

        // lldap 不支持 description 属性

        if !group.members.is_empty() {
            // lldap 使用 "uniqueMember" 属性
            let member_set: HashSet<&str> = group.members.iter().map(|s| s.as_str()).collect();
            attrs.push(("uniqueMember", member_set));
        }

        let result = ldap
            .add(&group_dn, attrs)
            .await
            .context("LDAP add failed")?;

        result.success().context("LDAP add result error")?;

        ldap.unbind().await.ok();

        info!("Created LDAP group: {}", group_dn);
        Ok(())
    }

    /// 更新组成员（使用 GraphQL API，因为 lldap 不支持 LDAP modify 操作）
    pub async fn update_group_members(&self, group_name: &str, members: &[String]) -> Result<()> {
        // 首先获取组 ID
        let groups = self.list_groups_graphql().await?;
        let group = groups
            .iter()
            .find(|g| g.display_name == group_name)
            .ok_or_else(|| anyhow::anyhow!("Group not found: {}", group_name))?;

        let group_id = group.id;

        // 获取所有用户 ID
        let users = self.list_users_graphql().await?;

        // 添加新成员
        for member_dn in members {
            // 从 DN 提取 uid (格式：uid=xxx,ou=people,dc=example,dc=com)
            let uid = member_dn
                .strip_prefix("uid=")
                .and_then(|s| s.split(',').next())
                .ok_or_else(|| anyhow::anyhow!("Invalid member DN format: {}", member_dn))?;

            if let Some(user) = users.iter().find(|u| u.id == uid) {
                self.add_user_to_group_graphql(&user.id, group_id).await?;
            }
        }

        info!("Updated LDAP group members: {}", group_name);
        Ok(())
    }

    /// 添加成员到组（使用 GraphQL API）
    pub async fn add_member_to_group(&self, group_name: &str, member_dn: &str) -> Result<()> {
        // 获取组 ID
        let groups = self.list_groups_graphql().await?;
        let group = groups
            .iter()
            .find(|g| g.display_name == group_name)
            .ok_or_else(|| anyhow::anyhow!("Group not found: {}", group_name))?;

        let group_id = group.id;

        // 从 DN 提取 uid
        let uid = member_dn
            .strip_prefix("uid=")
            .and_then(|s| s.split(',').next())
            .ok_or_else(|| anyhow::anyhow!("Invalid member DN format: {}", member_dn))?;

        // 获取用户 ID
        let users = self.list_users_graphql().await?;
        let user = users
            .iter()
            .find(|u| u.id == uid)
            .ok_or_else(|| anyhow::anyhow!("User not found: {}", uid))?;

        self.add_user_to_group_graphql(&user.id, group_id).await?;

        info!("Added member {} to group {}", member_dn, group_name);
        Ok(())
    }

    /// 从组中移除成员（使用 GraphQL API）
    pub async fn remove_member_from_group(&self, group_name: &str, member_dn: &str) -> Result<()> {
        // 获取组 ID
        let groups = self.list_groups_graphql().await?;
        let group = groups
            .iter()
            .find(|g| g.display_name == group_name)
            .ok_or_else(|| anyhow::anyhow!("Group not found: {}", group_name))?;

        let group_id = group.id;

        // 从 DN 提取 uid
        let uid = member_dn
            .strip_prefix("uid=")
            .and_then(|s| s.split(',').next())
            .ok_or_else(|| anyhow::anyhow!("Invalid member DN format: {}", member_dn))?;

        // 获取用户 ID
        let users = self.list_users_graphql().await?;
        let user = users
            .iter()
            .find(|u| u.id == uid)
            .ok_or_else(|| anyhow::anyhow!("User not found: {}", uid))?;

        self.remove_user_from_group_graphql(&user.id, group_id).await?;

        info!("Removed member {} from group {}", member_dn, group_name);
        Ok(())
    }

    /// 删除组
    pub async fn delete_group(&self, group_name: &str) -> Result<()> {
        let mut ldap = self.connect().await?;

        let group_dn = format!("cn={},{}", group_name, self.config.group_base_dn);

        let result = ldap
            .delete(&group_dn)
            .await
            .context("LDAP delete failed")?;

        result.success().context("LDAP delete result error")?;

        ldap.unbind().await.ok();

        info!("Deleted LDAP group: {}", group_dn);
        Ok(())
    }

    /// 列出所有组
    pub async fn list_groups(&self) -> Result<Vec<LdapGroup>> {
        let mut ldap = self.connect().await?;

        let filter = "(objectClass=groupOfUniqueNames)";

        // lldap 使用 "uniqueMember" 属性
        let (rs, _) = ldap
            .search(
                &self.config.group_base_dn,
                Scope::Subtree,
                filter,
                vec!["cn", "uniqueMember", "description"],
            )
            .await
            .context("LDAP search failed")?
            .success()
            .context("LDAP search result error")?;

        ldap.unbind().await.ok();

        let mut groups = Vec::new();
        for result in rs {
            let entry = SearchEntry::construct(result);
            let attrs = &entry.attrs;

            let members = attrs
                .get("uniqueMember")
                .cloned()
                .unwrap_or_default();

            groups.push(LdapGroup {
                cn: self.get_attr(attrs, "cn").unwrap_or_default(),
                members,
                description: self.get_attr(attrs, "description"),
            });
        }

        Ok(groups)
    }

    /// 列出所有用户
    pub async fn list_users(&self) -> Result<Vec<LdapUser>> {
        let mut ldap = self.connect().await?;

        let filter = "(objectClass=inetOrgPerson)";

        let (rs, _) = ldap
            .search(
                &self.config.user_base_dn,
                Scope::Subtree,
                filter,
                vec!["uid", "cn", "mail", "employeeID", "department", "title", "displayName"],
            )
            .await
            .context("LDAP search failed")?
            .success()
            .context("LDAP search result error")?;

        ldap.unbind().await.ok();

        let mut users = Vec::new();
        for result in rs {
            let entry = SearchEntry::construct(result);
            users.push(self.search_entry_to_ldap_user(&entry));
        }

        Ok(users)
    }

    // ==================== GraphQL API 方法 ====================

    /// 通过 GraphQL API 列出所有组（简化版，不包含成员）
    async fn list_groups_graphql(&self) -> Result<Vec<GraphQlGroupSimple>> {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct GroupsData {
            groups: Vec<GraphQlGroupSimple>,
        }

        let query = r#"
            query GetGroups {
                groups {
                    id
                    displayName
                }
            }
        "#;

        let response: GraphQlResponse<GroupsData> = self.execute_graphql(query).await?;
        Ok(response.data.map(|d| d.groups).unwrap_or_default())
    }

    /// 通过 GraphQL API 列出所有用户
    async fn list_users_graphql(&self) -> Result<Vec<GraphQlUser>> {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct UsersData {
            users: Vec<GraphQlUser>,
        }

        let query = r#"
            query GetUsers {
                users {
                    id
                    email
                    displayName
                }
            }
        "#;

        let response: GraphQlResponse<UsersData> = self.execute_graphql(query).await?;
        Ok(response.data.map(|d| d.users).unwrap_or_default())
    }

    /// 通过 GraphQL API 获取用户详情（包含组成员关系）
    async fn get_user_graphql(&self, user_id: &str) -> Result<Option<GraphQlUser>> {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct UserData {
            user: GraphQlUserWithGroups,
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct GraphQlUserWithGroups {
            id: String,
            email: Option<String>,
            #[serde(rename = "displayName")]
            display_name: Option<String>,
            groups: Vec<GraphQlGroupSimple>,
        }

        let query = format!(
            r#"
            query GetUser {{
                user(userId: "{user_id}") {{
                    id
                    email
                    displayName
                    groups {{
                        id
                        displayName
                    }}
                }}
            }}
            "#
        );

        let response: GraphQlResponse<UserData> = self.execute_graphql(&query).await?;
        Ok(response.data.map(|d| {
            GraphQlUser {
                id: d.user.id,
                email: d.user.email,
                display_name: d.user.display_name,
            }
        }))
    }

    /// 通过 GraphQL API 添加用户到组
    async fn add_user_to_group_graphql(&self, user_id: &str, group_id: i64) -> Result<()> {
        let query = format!(
            r#"
            mutation AddUserToGroup {{
                addUserToGroup(userId: "{user_id}", groupId: {group_id}) {{
                    __typename
                }}
            }}
            "#
        );

        let _response: GraphQlResponse<serde_json::Value> = self.execute_graphql(&query).await?;
        Ok(())
    }

    /// 通过 GraphQL API 从组中移除用户
    async fn remove_user_from_group_graphql(&self, user_id: &str, group_id: i64) -> Result<()> {
        let query = format!(
            r#"
            mutation RemoveUserFromGroup {{
                removeUserFromGroup(userId: "{user_id}", groupId: {group_id}) {{
                    __typename
                }}
            }}
            "#
        );

        let _response: GraphQlResponse<serde_json::Value> = self.execute_graphql(&query).await?;
        Ok(())
    }

    /// 执行 GraphQL 查询（带登录）
    async fn execute_graphql<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
    ) -> Result<GraphQlResponse<T>> {
        let graphql_url = self.config.graphql_url();
        let auth_url = format!("http://{}:{}/auth/simple/login", self.config.http_host, self.config.http_port);

        // 使用 REST API 登录获取 JWT token
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct LoginResponse {
            token: String,
            #[serde(rename = "refreshToken")]
            #[serde(default)]
            refresh_token: Option<String>,
        }

        // 先获取原始响应以便调试
        let login_resp = self
            .http_client
            .post(&auth_url)
            .json(&serde_json::json!({
                "username": "admin",
                "password": self.config.password
            }))
            .send()
            .await
            .context("Failed to send login request")?;

        let login_status = login_resp.status();
        let login_text = login_resp.text().await
            .context("Failed to read login response text")?;

        debug!("Login response status: {}, body: {}", login_status, &login_text[..login_text.len().min(200)]);

        // 解析 JSON
        let login_data: serde_json::Value = serde_json::from_str(&login_text)
            .with_context(|| format!("Failed to parse login JSON: {}", &login_text[..login_text.len().min(200)]))?;

        let token = login_data.get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No token in login response: {}", &login_text[..login_text.len().min(200)]))?;

        // 使用 Bearer token 执行 GraphQL 查询
        let response = self
            .http_client
            .post(&graphql_url)
            .json(&serde_json::json!({"query": query}))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token),
            )
            .send()
            .await
            .context("Failed to send GraphQL request")?;

        let graphql_response: GraphQlResponse<T> = response
            .json()
            .await
            .context("Failed to parse GraphQL response")?;

        if let Some(errors) = graphql_response.errors {
            let error_msgs: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
            return Err(anyhow::anyhow!("GraphQL errors: {:?}", error_msgs));
        }

        Ok(graphql_response)
    }
}
