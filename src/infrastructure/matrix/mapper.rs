//! Matrix ID 映射器
//!
//! 负责内部 ID 与 Matrix ID 之间的双向映射

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

use super::config::MatrixConfig;

/// 用户映射记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMapping {
    /// 内部用户 ID
    pub internal_id: String,
    /// Matrix 用户 ID (如：@_ceo:localhost)
    pub matrix_user_id: String,
    /// 显示名称
    pub display_name: Option<String>,
    /// 是否为虚拟用户
    pub is_virtual: bool,
}

/// 房间映射记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMapping {
    /// 内部组/房间 ID
    pub internal_id: String,
    /// Matrix 房间 ID (如：!room123:localhost)
    pub matrix_room_id: String,
    /// Matrix 房间别名 (如：#company-general:localhost)
    pub matrix_alias: Option<String>,
    /// 房间名称
    pub name: Option<String>,
}

/// ID 映射器
pub struct Mapper {
    config: MatrixConfig,
    /// 用户 ID 映射：internal_id -> UserMapping
    user_mappings: HashMap<String, UserMapping>,
    /// Matrix 用户 ID 反向映射：matrix_user_id -> internal_id
    matrix_user_reverse: HashMap<String, String>,
    /// 房间 ID 映射：internal_id -> RoomMapping
    room_mappings: HashMap<String, RoomMapping>,
    /// Matrix 房间 ID 反向映射：matrix_room_id -> internal_id
    matrix_room_reverse: HashMap<String, String>,
}

impl Mapper {
    /// 创建新的映射器
    pub fn new(config: &MatrixConfig) -> Self {
        Self {
            config: config.clone(),
            user_mappings: HashMap::new(),
            matrix_user_reverse: HashMap::new(),
            room_mappings: HashMap::new(),
            matrix_room_reverse: HashMap::new(),
        }
    }

    /// 从存储加载映射
    pub async fn load_from_store(
        &mut self,
        store: &dyn crate::core::store::Store,
    ) -> Result<()> {
        // 加载用户映射
        self.load_user_mappings(store).await?;

        // 加载房间映射
        self.load_room_mappings(store).await?;

        Ok(())
    }

    /// 加载用户映射
    async fn load_user_mappings(
        &mut self,
        store: &dyn crate::core::store::Store,
    ) -> Result<()> {
        // 从组织架构加载所有 Agent
        match store.load_organization().await {
            Ok(org) => {
                for agent in &org.agents {
                    let matrix_user_id = self
                        .config
                        .generate_user_id(&agent.name.to_lowercase().replace(' ', "_"));
                    let mapping = UserMapping {
                        internal_id: agent.id.clone(),
                        matrix_user_id: matrix_user_id.clone(),
                        display_name: Some(agent.name.clone()),
                        is_virtual: true,
                    };
                    self.user_mappings.insert(agent.id.clone(), mapping);
                    self.matrix_user_reverse
                        .insert(matrix_user_id.clone(), agent.id.clone());
                    debug!(
                        "Loaded user mapping: {} -> {}",
                        agent.id, matrix_user_id
                    );
                }
            }
            Err(e) => {
                info!("Failed to load organization for user mappings: {}", e);
            }
        }

        // 从存储加载所有用户
        match store.load_users().await {
            Ok(users) => {
                for user in users {
                    let matrix_user_id = self.config.generate_user_id(&format!(
                        "user_{}",
                        user.username.to_lowercase()
                    ));
                    let mapping = UserMapping {
                        internal_id: user.id.clone(),
                        matrix_user_id: matrix_user_id.clone(),
                        display_name: Some(user.name.clone()),
                        is_virtual: false,
                    };
                    self.user_mappings.insert(user.id.clone(), mapping);
                    self.matrix_user_reverse
                        .insert(matrix_user_id.clone(), user.id.clone());
                    debug!(
                        "Loaded user mapping: {} -> {}",
                        user.id, matrix_user_id
                    );
                }
            }
            Err(e) => {
                info!("Failed to load users for user mappings: {}", e);
            }
        }

        Ok(())
    }

    /// 加载房间映射
    async fn load_room_mappings(
        &mut self,
        store: &dyn crate::core::store::Store,
    ) -> Result<()> {
        // 从组织架构加载所有部门作为房间
        match store.load_organization().await {
            Ok(org) => {
                for dept in &org.departments {
                    let alias = format!("company_{}", dept.name.to_lowercase().replace(' ', "_"));
                    let matrix_alias = self.config.generate_room_alias(&alias);
                    let mapping = RoomMapping {
                        internal_id: dept.id.clone(),
                        matrix_room_id: matrix_alias.clone(), // 初始时使用别名，后续会更新为实际房间 ID
                        matrix_alias: Some(matrix_alias),
                        name: Some(dept.name.clone()),
                    };
                    self.room_mappings.insert(dept.id.clone(), mapping);
                }
            }
            Err(e) => {
                info!("Failed to load organization for room mappings: {}", e);
            }
        }

        Ok(())
    }

    /// 获取用户的 Matrix ID
    pub fn get_matrix_user_id(&self, internal_id: &str) -> Option<&str> {
        self.user_mappings
            .get(internal_id)
            .map(|m| m.matrix_user_id.as_str())
    }

    /// 获取 Matrix 用户 ID 对应的内部 ID
    pub fn get_internal_user_id(&self, matrix_user_id: &str) -> Option<&str> {
        self.matrix_user_reverse
            .get(matrix_user_id)
            .map(|id| id.as_str())
    }

    /// 获取房间的 Matrix ID
    pub fn get_matrix_room_id(&self, internal_id: &str) -> Option<&str> {
        self.room_mappings
            .get(internal_id)
            .map(|m| m.matrix_room_id.as_str())
    }

    /// 获取 Matrix 房间 ID 对应的内部 ID
    pub fn get_internal_room_id(&self, matrix_room_id: &str) -> Option<&str> {
        self.matrix_room_reverse
            .get(matrix_room_id)
            .map(|id| id.as_str())
    }

    /// 注册新的用户映射
    pub fn register_user(&mut self, internal_id: &str, display_name: &str) -> String {
        let matrix_user_id = self.config.generate_user_id(&format!(
            "user_{}",
            display_name.to_lowercase().replace(' ', "_")
        ));

        let mapping = UserMapping {
            internal_id: internal_id.to_string(),
            matrix_user_id: matrix_user_id.clone(),
            display_name: Some(display_name.to_string()),
            is_virtual: false,
        };

        self.user_mappings
            .insert(internal_id.to_string(), mapping);
        self.matrix_user_reverse
            .insert(matrix_user_id.clone(), internal_id.to_string());

        info!(
            "Registered user mapping: {} -> {}",
            internal_id, matrix_user_id
        );

        matrix_user_id
    }

    /// 注册新的房间映射
    pub fn register_room(
        &mut self,
        internal_id: &str,
        matrix_room_id: &str,
        name: Option<&str>,
    ) {
        let matrix_alias = if matrix_room_id.starts_with('#') {
            Some(matrix_room_id.to_string())
        } else {
            None
        };

        let mapping = RoomMapping {
            internal_id: internal_id.to_string(),
            matrix_room_id: matrix_room_id.to_string(),
            matrix_alias,
            name: name.map(String::from),
        };

        self.room_mappings
            .insert(internal_id.to_string(), mapping);
        self.matrix_room_reverse
            .insert(matrix_room_id.to_string(), internal_id.to_string());

        info!(
            "Registered room mapping: {} -> {}",
            internal_id, matrix_room_id
        );
    }

    /// 更新房间的实际 Matrix ID（从别名解析后）
    pub fn update_room_matrix_id(&mut self, internal_id: &str, matrix_room_id: &str) {
        if let Some(mapping) = self.room_mappings.get_mut(internal_id) {
            // 移除旧的反向映射
            self.matrix_room_reverse.remove(&mapping.matrix_room_id);

            // 更新映射
            mapping.matrix_room_id = matrix_room_id.to_string();

            // 添加新的反向映射
            self.matrix_room_reverse
                .insert(matrix_room_id.to_string(), internal_id.to_string());

            info!(
                "Updated room matrix_id: {} -> {}",
                internal_id, matrix_room_id
            );
        }
    }

    /// 获取所有用户映射
    pub fn get_all_user_mappings(&self) -> Vec<&UserMapping> {
        self.user_mappings.values().collect()
    }

    /// 获取所有房间映射
    pub fn get_all_room_mappings(&self) -> Vec<&RoomMapping> {
        self.room_mappings.values().collect()
    }

    /// 检查用户是否是虚拟用户
    pub fn is_virtual_user(&self, internal_id: &str) -> bool {
        self.user_mappings
            .get(internal_id)
            .map(|m| m.is_virtual)
            .unwrap_or(false)
    }

    /// 生成 Agent 的 Matrix 用户 ID
    pub fn generate_agent_user_id(&self, agent_name: &str) -> String {
        self.config
            .generate_user_id(&agent_name.to_lowercase().replace(' ', "_"))
    }

    /// 生成部门房间的别名
    pub fn generate_department_alias(&self, department: &str) -> String {
        self.config
            .generate_room_alias(&format!("dept_{}", department.to_lowercase().replace(' ', "_")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_mapping() {
        let config = MatrixConfig {
            homeserver_url: "http://localhost:5141".to_string(),
            server_name: "localhost".to_string(),
            as_token: "test".to_string(),
            hs_token: "test".to_string(),
            sender_localpart: "_bot".to_string(),
            appservice_port: 9000,
        };

        let mut mapper = Mapper::new(&config);

        // 注册用户
        let matrix_id = mapper.register_user("user123", "John Doe");
        assert!(matrix_id.starts_with("@_user_john"));
        assert!(matrix_id.ends_with(":localhost"));

        // 查询映射
        assert_eq!(
            mapper.get_matrix_user_id("user123"),
            Some(matrix_id.as_str())
        );
        assert_eq!(
            mapper.get_internal_user_id(&matrix_id),
            Some("user123")
        );
    }

    #[test]
    fn test_room_mapping() {
        let config = MatrixConfig {
            homeserver_url: "http://localhost:5141".to_string(),
            server_name: "localhost".to_string(),
            as_token: "test".to_string(),
            hs_token: "test".to_string(),
            sender_localpart: "_bot".to_string(),
            appservice_port: 9000,
        };

        let mut mapper = Mapper::new(&config);

        // 注册房间
        mapper.register_room("dept_engineering", "!room123:localhost", Some("Engineering"));

        // 查询映射
        assert_eq!(
            mapper.get_matrix_room_id("dept_engineering"),
            Some("!room123:localhost")
        );
        assert_eq!(
            mapper.get_internal_room_id("!room123:localhost"),
            Some("dept_engineering")
        );
    }
}
