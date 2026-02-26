//! 用户认证相关模型

use serde::{Deserialize, Serialize};

/// 用户职位枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Position {
    /// 集团主席
    Chairman,
    /// 管理层
    Management,
    /// 普通员工
    Employee,
}

impl Default for Position {
    fn default() -> Self {
        Position::Employee
    }
}

/// 用户模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub name: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub employee_id: String,  // 工号
    pub position: Position,   // 职位
    pub department: String,   // 部门
    pub created_at: i64,
}

impl User {
    pub fn new_chairman(username: String, name: String, password_hash: String, email: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            name,
            email,
            password_hash,
            employee_id: "00001".to_string(),  // 集团主席固定工号
            position: Position::Chairman,
            department: "集团办公室".to_string(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn new_management(username: String, name: String, password_hash: String, employee_seq: u32, email: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            name,
            email,
            password_hash,
            employee_id: format!("{:05}", employee_seq),  // 管理层工号从00002开始
            position: Position::Management,
            department: "综合管理部".to_string(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn new_employee(username: String, name: String, password_hash: String, employee_seq: u32, department: String, email: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            name,
            email,
            password_hash,
            employee_id: format!("1{:04}", employee_seq),  // 普通员工工号以1开头
            position: Position::Employee,
            department,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}