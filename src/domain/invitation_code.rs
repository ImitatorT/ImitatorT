//! 邀请码相关模型

use serde::{Deserialize, Serialize};

/// 邀请码模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationCode {
    pub id: String,
    pub code: String,
    pub created_by: String,      // 创建者ID
    pub expiry_time: i64,        // 过期时间戳（秒）
    pub is_used: bool,           // 是否已使用
    pub max_usage: u32,          // 最大使用次数，默认1
    pub current_usage: u32,      // 当前使用次数
    pub created_at: i64,
}

impl InvitationCode {
    pub fn new(created_by: String, max_usage: Option<u32>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            code: Self::generate_code(),
            created_by,
            expiry_time: Self::calculate_expiry_time(),  // 1天后过期
            is_used: false,
            max_usage: max_usage.unwrap_or(1),  // 默认只能使用1次
            current_usage: 0,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// 生成随机邀请码（16位字母数字组合）
    fn generate_code() -> String {
        use rand::{distributions::Alphanumeric, Rng};
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect()
    }

    /// 计算过期时间（当前时间+1天）
    fn calculate_expiry_time() -> i64 {
        chrono::Utc::now().timestamp() + 24 * 60 * 60  // 24小时 = 86400秒
    }

    /// 检查邀请码是否有效（未过期且未达到最大使用次数）
    pub fn is_valid(&self) -> bool {
        let current_time = chrono::Utc::now().timestamp();
        !self.is_used &&
        current_time < self.expiry_time &&
        self.current_usage < self.max_usage
    }

    /// 使用邀请码（增加使用次数，如果达到最大次数则标记为已使用）
    pub fn use_code(&mut self) {
        self.current_usage += 1;
        if self.current_usage >= self.max_usage {
            self.is_used = true;
        }
    }
}