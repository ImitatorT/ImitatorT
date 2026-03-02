use imitatort::domain::invitation_code::InvitationCode;
use imitatort::domain::user::{Position, User};
use imitatort::infrastructure::auth::PasswordService;

#[test]
fn test_user_position_and_employee_id() {
    // Test corporate chairman user
    let password_hash = PasswordService::hash_password("password123").unwrap();
    let chairman_user = User::new_chairman(
        "chairman_user".to_string(),
        "Corporate Chairman".to_string(),
        password_hash,
        None,
    );

    assert_eq!(chairman_user.employee_id, "00001");
    assert!(matches!(chairman_user.position, Position::Chairman));
    assert_eq!(chairman_user.department, "Corporate Office");

    // 测试管理层用户
    let password_hash = PasswordService::hash_password("password123").unwrap();
    let management_user = User::new_management(
        "manager_user".to_string(),
        "Manager".to_string(),
        password_hash,
        2, // Employee ID 00002
        None,
    );

    assert_eq!(management_user.employee_id, "00002");
    assert!(matches!(management_user.position, Position::Management));
    assert_eq!(management_user.department, "General Management Department");

    // Test regular employee user
    let password_hash = PasswordService::hash_password("password123").unwrap();
    let employee_user = User::new_employee(
        "employee_user".to_string(),
        "Employee".to_string(),
        password_hash,
        1, // Employee ID 10001
        "Sales Department".to_string(),
        None,
    );

    assert_eq!(employee_user.employee_id, "10001");
    assert!(matches!(employee_user.position, Position::Employee));
    assert_eq!(employee_user.department, "Sales Department");

    println!("✓ 用户职位和工号测试通过");
}

#[test]
fn test_invitation_code_functionality() {
    // 测试邀请码生成和验证
    let invitation_code = InvitationCode::new("test_user".to_string(), Some(1));

    assert!(invitation_code.is_valid());
    assert_eq!(invitation_code.current_usage, 0);
    assert_eq!(invitation_code.max_usage, 1);
    assert_eq!(invitation_code.created_by, "test_user");

    // 测试使用邀请码
    let mut code_to_use = invitation_code;
    code_to_use.use_code();

    assert_eq!(code_to_use.current_usage, 1);
    assert!(!code_to_use.is_valid()); // 因为已达到最大使用次数

    println!("✓ 邀请码功能测试通过");
}

#[test]
fn test_invitation_code_multiple_usage() {
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

    println!("✓ 邀请码多次使用测试通过");
}
