//! 测试通用工具
//!
//! 提供测试辅助函数和通用测试工具

use std::sync::Once;

static INIT: Once = Once::new();

/// 初始化测试环境
pub fn setup() {
    INIT.call_once(|| {
        // 设置测试日志
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .try_init();
    });
}

/// 生成唯一的测试 ID
pub fn generate_test_id() -> String {
    format!("test-{}", uuid::Uuid::new_v4())
}

/// 测试超时包装器（用于异步测试）
#[allow(dead_code)]
pub async fn with_timeout<F, T>(duration: std::time::Duration, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(duration, f)
        .await
        .expect("Test timed out")
}

/// 常用的测试超时时间
pub const TEST_TIMEOUT_SHORT: std::time::Duration = std::time::Duration::from_secs(5);
pub const TEST_TIMEOUT_MEDIUM: std::time::Duration = std::time::Duration::from_secs(30);
pub const TEST_TIMEOUT_LONG: std::time::Duration = std::time::Duration::from_secs(60);
