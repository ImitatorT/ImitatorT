use std::sync::Arc;

use tokio;
use serde_json::json;

use imitatort::domain::user::User;
use imitatort::domain::invitation_code::InvitationCode;
use imitatort::core::store::Store;
use imitatort::infrastructure::store::SqliteStore;
use imitatort::infrastructure::auth::{JwtService, PasswordService};

#[tokio::test]
async fn test_user_registration_flow() {
    let store = Arc::new(SqliteStore::new_in_memory().unwrap());
    let jwt_service = JwtService::new("test_secret");

    // 1. Test first registered user becomes corporate chairman
    let password_hash = PasswordService::hash_password("password123").unwrap();
    let chairman_user = User::new_chairman(
        "chairman_user".to_string(),
        "Corporate Chairman".to_string(),
        password_hash,
        None,
    );

    assert_eq!(chairman_user.employee_id, "00001");
    assert!(matches!(chairman_user.position, imitatort::domain::user::Position::Chairman));

    // 2. 测试邀请码生成和验证
    let invitation_code = InvitationCode::new("chairman_user".to_string(), Some(1));
    assert!(invitation_code.is_valid());

    // 3. 测试管理层用户创建
    let management_password_hash = PasswordService::hash_password("password123").unwrap();
    let management_user = User::new_management(
        "manager_user".to_string(),
        "Manager".to_string(),
        management_password_hash,
        2, // Employee ID 00002
        None,
    );

    assert_eq!(management_user.employee_id, "00002");
    assert!(matches!(management_user.position, imitatort::domain::user::Position::Management));

    // 4. Test regular employee user creation
    let employee_password_hash = PasswordService::hash_password("password123").unwrap();
    let employee_user = User::new_employee(
        "employee_user".to_string(),
        "Employee".to_string(),
        employee_password_hash,
        1, // Employee ID 10001
        "Sales Department".to_string(),
        None,
    );

    assert_eq!(employee_user.employee_id, "10001");
    assert!(matches!(employee_user.position, imitatort::domain::user::Position::Employee));

    println!("All user registration flow tests passed!");
}

#[tokio::test]
async fn test_invitation_code_lifecycle() {
    let store = Arc::new(SqliteStore::new_in_memory().unwrap());

    // Create invitation code
    let mut invitation_code = InvitationCode::new("test_user".to_string(), Some(1));
    assert!(invitation_code.is_valid());
    assert_eq!(invitation_code.current_usage, 0);
    assert_eq!(invitation_code.max_usage, 1);

    // Use invitation code
    invitation_code.use_code();
    assert_eq!(invitation_code.current_usage, 1);
    assert!(!invitation_code.is_valid()); // Because maximum usage has been reached

    // Test multiple uses of invitation code
    let mut invitation_code_multi = InvitationCode::new("test_user".to_string(), Some(2));
    assert!(invitation_code_multi.is_valid());

    // First use
    invitation_code_multi.use_code();
    assert_eq!(invitation_code_multi.current_usage, 1);
    assert!(invitation_code_multi.is_valid()); // Still valid, because maximum usage hasn't been reached yet

    // Second use
    invitation_code_multi.use_code();
    assert_eq!(invitation_code_multi.current_usage, 2);
    assert!(!invitation_code_multi.is_valid()); // Now should be invalid

    println!("All invitation code lifecycle tests passed!");
}