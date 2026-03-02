//! 认证和授权模块
//!
//! 提供JWT令牌和密码哈希服务

use anyhow::Result;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: String,
    pub username: String,
    pub name: String,
    pub email: Option<String>,
    pub is_director: bool,
    pub employee_id: String,
    pub position: String,
    pub department: String,
    pub exp: usize,
}

#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub name: String,
    pub email: Option<String>,
    pub is_director: bool,
    pub employee_id: String,
    pub position: String,
    pub department: String,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        let secret = secret.as_bytes();
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            algorithm: Algorithm::HS256,
        }
    }

    pub fn generate_token(&self, user_info: &UserInfo) -> Result<String> {
        let expiration = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
            + 3600 * 24 * 7) as usize; // 7天过期

        let claims = Claims {
            id: user_info.id.clone(),
            username: user_info.username.clone(),
            name: user_info.name.clone(),
            email: user_info.email.clone(),
            is_director: user_info.is_director,
            employee_id: user_info.employee_id.clone(),
            position: user_info.position.clone(),
            department: user_info.department.clone(),
            exp: expiration,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;
        Ok(token)
    }

    pub fn validate_token(&self, token: &str) -> Result<UserInfo> {
        let mut validation = Validation::new(self.algorithm);
        validation.validate_exp = true;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        let claims = token_data.claims;

        Ok(UserInfo {
            id: claims.id,
            username: claims.username,
            name: claims.name,
            email: claims.email,
            is_director: claims.is_director,
            employee_id: claims.employee_id,
            position: claims.position,
            department: claims.department,
        })
    }
}

pub struct PasswordService;

impl PasswordService {
    pub fn hash_password(password: &str) -> Result<String> {
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        Ok(hash)
    }

    pub fn verify_password(hash: &str, password: &str) -> Result<bool> {
        let valid = bcrypt::verify(password, hash)?;
        Ok(valid)
    }
}
