use std::sync::Arc;

use tokio;
use serde_json::json;

use imitatort_stateless_company::domain::user::User;
use imitatort_stateless_company::domain::invitation_code::InvitationCode;
use imitatort_stateless_company::core::store::{Store, MemoryStore};
use imitatort_stateless_company::infrastructure::auth::{JwtService, PasswordService};

#[tokio::test]
async fn test_user_registration_flow() {
    let store = Arc::new(MemoryStore::new());
    let jwt_service = JwtService::new("test_secret");

    // 1. 测试首位注册用户成为集团主席
    let password_hash = PasswordService::hash_password("password123").unwrap();
    let chairman_user = User::new_chairman(
        "chairman_user".to_string(),
        "集团主席".to_string(),
        password_hash,
        None,
    );

    assert_eq!(chairman_user.employee_id, "00001");
    assert!(matches!(chairman_user.position, imitatort_stateless_company::domain::user::Position::Chairman));

    // 2. 测试邀请码生成和验证
    let invitation_code = InvitationCode::new("chairman_user".to_string(), Some(1));
    assert!(invitation_code.is_valid());

    // 3. 测试管理层用户创建
    let management_password_hash = PasswordService::hash_password("password123").unwrap();
    let management_user = User::new_management(
        "manager_user".to_string(),
        "经理".to_string(),
        management_password_hash,
        2, // 工号00002
        None,
    );

    assert_eq!(management_user.employee_id, "00002");
    assert!(matches!(management_user.position, imitatort_stateless_company::domain::user::Position::Management));

    // 4. 测试普通员工用户创建
    let employee_password_hash = PasswordService::hash_password("password123").unwrap();
    let employee_user = User::new_employee(
        "employee_user".to_string(),
        "员工".to_string(),
        employee_password_hash,
        1, // 工号10001
        "销售部".to_string(),
        None,
    );

    assert_eq!(employee_user.employee_id, "10001");
    assert!(matches!(employee_user.position, imitatort_stateless_company::domain::user::Position::Employee));

    println!("All user registration flow tests passed!");
}

#[tokio::test]
async fn test_invitation_code_lifecycle() {
    let store = Arc::new(MemoryStore::new());

    // 创建邀请码
    let mut invitation_code = InvitationCode::new("test_user".to_string(), Some(1));
    assert!(invitation_code.is_valid());
    assert_eq!(invitation_code.current_usage, 0);
    assert_eq!(invitation_code.max_usage, 1);

    // 使用邀请码
    invitation_code.use_code();
    assert_eq!(invitation_code.current_usage, 1);
    assert!(!invitation_code.is_valid()); // 因为已达到最大使用次数

    // 测试多次使用邀请码
    let mut invitation_code_multi = InvitationCode::new("test_user".to_string(), Some(2));
    assert!(invitation_code_multi.is_valid());

    // 第一次使用
    invitation_code_multi.use_code();
    assert_eq!(invitation_code_multi.current_usage, 1);
    assert!(invitation_code_multi.is_valid()); // 仍然有效，因为还没达到最大使用次数

    // 第二次使用
    invitation_code_multi.use_code();
    assert_eq!(invitation_code_multi.current_usage, 2);
    assert!(!invitation_code_multi.is_valid()); // 现在应该无效了

    println!("All invitation code lifecycle tests passed!");
}