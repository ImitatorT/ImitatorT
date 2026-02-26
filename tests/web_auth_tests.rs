use imitatort_stateless_company::infrastructure::auth::{JwtService, PasswordService};

#[tokio::test]
async fn test_password_hashing() {
    let password = "test_password";
    let hashed = PasswordService::hash_password(password).expect("Failed to hash password");
    assert!(PasswordService::verify_password(&hashed, password).expect("Failed to verify password"));
    assert!(!PasswordService::verify_password(&hashed, "wrong_password").expect("Failed to verify password"));
}

#[tokio::test]
async fn test_jwt_generation() {
    let jwt_service = JwtService::new("secret_key_for_testing");
    let user = imitatort_stateless_company::infrastructure::auth::UserInfo {
        id: "user123".to_string(),
        username: "testuser".to_string(),
        name: "Test User".to_string(),
        email: Some("test@example.com".to_string()),
        is_director: false,
    };

    let token = jwt_service.generate_token(&user).expect("Failed to generate token");
    let decoded_user = jwt_service.validate_token(&token).expect("Failed to validate token");
    assert_eq!(decoded_user.id, user.id);
    assert_eq!(decoded_user.username, user.username);
}