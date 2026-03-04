//! 请求频率限制器
//!
//! 使用简单的固定窗口限流算法

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// 频率限制器配置
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// 每秒允许的请求数
    pub requests_per_second: u64,
    /// 桶容量（突发请求）
    pub bucket_capacity: u64,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 1,
            bucket_capacity: 5,
        }
    }
}

/// 频率限制器
pub struct RateLimiter {
    config: RateLimiterConfig,
    tokens: AtomicU64,
    last_refill: Mutex<Instant>,
}

impl RateLimiter {
    /// 创建新的频率限制器
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            tokens: AtomicU64::new(config.bucket_capacity),
            last_refill: Mutex::new(Instant::now()),
            config,
        }
    }

    /// 等待并获取令牌
    pub async fn acquire(&self) {
        loop {
            if self.try_acquire() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// 尝试获取令牌（非阻塞）
    pub fn try_acquire(&self) -> bool {
        let current_tokens = self.tokens.load(Ordering::SeqCst);
        if current_tokens > 0 {
            let result = self.tokens.compare_exchange(
                current_tokens,
                current_tokens - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
            return result.is_ok();
        }
        false
    }
}

/// 简单的固定窗口限流器
pub struct SimpleRateLimiter {
    requests_per_second: u64,
    last_request: Mutex<Instant>,
}

impl SimpleRateLimiter {
    /// 创建简单的限流器
    pub fn new(requests_per_second: u64) -> Self {
        Self {
            requests_per_second,
            last_request: Mutex::new(Instant::now()),
        }
    }

    /// 等待直到可以发送下一个请求
    pub async fn wait(&self) {
        let min_interval = Duration::from_secs(1) / self.requests_per_second as u32;

        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();

        if elapsed < min_interval {
            tokio::time::sleep(min_interval - elapsed).await;
        }

        *last = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_rate_limiter() {
        let limiter = SimpleRateLimiter::new(2);

        // 第一次请求
        limiter.wait().await;

        // 第二次请求应该等待约 500ms
        let start = Instant::now();
        limiter.wait().await;
        let elapsed = start.elapsed();

        // 验证等待了至少 300ms（放宽条件以适应系统延迟）
        assert!(elapsed >= Duration::from_millis(300), "elapsed = {:?}", elapsed);
    }
}
