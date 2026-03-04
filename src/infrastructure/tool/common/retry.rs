//! 重试工具函数
//!
//! 提供指数退避重试机制

use std::time::Duration;

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 初始延迟（毫秒）
    pub initial_delay_ms: u64,
    /// 最大延迟（毫秒）
    pub max_delay_ms: u64,
    /// 退避乘数
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 10000,
            multiplier: 2.0,
        }
    }
}

/// 执行带重试的异步操作
pub async fn retry_async<F, Fut, T>(
    config: &RetryConfig,
    mut operation: F,
) -> Result<T, anyhow::Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, anyhow::Error>>,
{
    let mut last_error = None;
    let mut delay_ms = config.initial_delay_ms;

    for attempt in 0..config.max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < config.max_retries - 1 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = std::cmp::min(
                        (delay_ms as f64 * config.multiplier) as u64,
                        config.max_delay_ms,
                    );
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_retry_success() {
        let config = RetryConfig::default();
        let call_count = Arc::new(Mutex::new(0));

        let result = retry_async(&config, || {
            let call_count = Arc::clone(&call_count);
            async move {
                let mut count = call_count.lock().await;
                *count += 1;
                if *count < 2 {
                    Err(anyhow::anyhow!("Temporary error"))
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
    }
}
