//! User Authentication Related Models

use serde::{Deserialize, Serialize};

/// User Position Enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum Position {
    /// Group Chairman
    Chairman,
    /// Management
    Management,
    /// Regular Employee
    #[default]
    Employee,
}

/// User Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub name: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub employee_id: String, // Employee ID
    pub position: Position,  // Position
    pub department: String,  // Department
    pub created_at: i64,
}

impl User {
    pub fn new_chairman(
        username: String,
        name: String,
        password_hash: String,
        email: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            name,
            email,
            password_hash,
            employee_id: "00001".to_string(), // Fixed employee ID for Group Chairman
            position: Position::Chairman,
            department: "Corporate Office".to_string(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn new_management(
        username: String,
        name: String,
        password_hash: String,
        employee_seq: u32,
        email: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            name,
            email,
            password_hash,
            employee_id: format!("{:05}", employee_seq), // Management employee ID starts from 00002
            position: Position::Management,
            department: "General Management Department".to_string(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn new_employee(
        username: String,
        name: String,
        password_hash: String,
        employee_seq: u32,
        department: String,
        email: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            name,
            email,
            password_hash,
            employee_id: format!("1{:04}", employee_seq), // Regular employee ID starts with 1
            position: Position::Employee,
            department,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}
