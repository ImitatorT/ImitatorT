//! Invitation Code Related Models

use serde::{Deserialize, Serialize};

/// Invitation Code Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationCode {
    pub id: String,
    pub code: String,
    pub created_by: String, // Creator ID
    pub expiry_time: i64,   // Expiration timestamp (seconds)
    pub is_used: bool,      // Whether it has been used
    pub max_usage: u32,     // Maximum usage count, default 1
    pub current_usage: u32, // Current usage count
    pub created_at: i64,
}

impl InvitationCode {
    pub fn new(created_by: String, max_usage: Option<u32>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            code: Self::generate_code(),
            created_by,
            expiry_time: Self::calculate_expiry_time(), // Expires after 1 day
            is_used: false,
            max_usage: max_usage.unwrap_or(1), // Default can only be used once
            current_usage: 0,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Generate random invitation code (16-character alphanumeric combination)
    fn generate_code() -> String {
        use rand::{distributions::Alphanumeric, Rng};
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect()
    }

    /// Calculate expiration time (current time + 1 day)
    fn calculate_expiry_time() -> i64 {
        chrono::Utc::now().timestamp() + 24 * 60 * 60 // 24 hours = 86400 seconds
    }

    /// Check if invitation code is valid (not expired and not reached maximum usage)
    pub fn is_valid(&self) -> bool {
        let current_time = chrono::Utc::now().timestamp();
        !self.is_used && current_time < self.expiry_time && self.current_usage < self.max_usage
    }

    /// Use invitation code (increase usage count, mark as used if reached maximum)
    pub fn use_code(&mut self) {
        self.current_usage += 1;
        if self.current_usage >= self.max_usage {
            self.is_used = true;
        }
    }
}
