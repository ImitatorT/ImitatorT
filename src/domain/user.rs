//! 用户认证相关模型

use serde::{Deserialize, Serialize};

/// 用户模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub name: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub is_director: bool,
    pub created_at: i64,
}

impl User {
    pub fn new(username: String, name: String, password_hash: String, email: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            name,
            email,
            password_hash,
            is_director: false,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}