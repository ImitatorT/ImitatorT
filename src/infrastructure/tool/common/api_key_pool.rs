//! API Key 池管理
//!
//! 支持多账号轮询和密钥管理

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use dashmap::DashMap;
use std::time::Instant;

/// API Key 池
pub struct ApiKeyPool {
    /// API Keys 列表
    keys: Arc<Vec<String>>,
    /// 当前索引
    current_index: AtomicUsize,
    /// 每个 key 的使用计数
    usage_counts: DashMap<String, AtomicUsize>,
    /// 每个 key 的冷却时间（如果达到限制）
    cooldowns: DashMap<String, Instant>,
    /// 单个 key 的最大使用次数（0 表示无限制）
    max_usage_per_key: usize,
}

impl ApiKeyPool {
    /// 创建新的 API Key 池
    pub fn new(keys: Vec<String>) -> Self {
        let usage_counts = DashMap::new();
        for key in &keys {
            usage_counts.insert(key.clone(), AtomicUsize::new(0));
        }

        Self {
            keys: Arc::new(keys),
            current_index: AtomicUsize::new(0),
            usage_counts,
            cooldowns: DashMap::new(),
            max_usage_per_key: 0,
        }
    }

    /// 创建带使用限制的 API Key 池
    pub fn with_usage_limit(keys: Vec<String>, max_usage: usize) -> Self {
        let mut pool = Self::new(keys);
        pool.max_usage_per_key = max_usage;
        pool
    }

    /// 从逗号分隔的字符串创建
    pub fn from_csv(csv: &str) -> Self {
        let keys: Vec<String> = csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self::new(keys)
    }

    /// 获取下一个可用的 API Key（轮询）
    pub fn get_next_key(&self) -> Option<String> {
        if self.keys.is_empty() {
            return None;
        }

        let mut attempts = 0;
        let num_keys = self.keys.len();

        while attempts < num_keys {
            let index = self.current_index.fetch_add(1, Ordering::SeqCst) % num_keys;
            let key = &self.keys[index];

            // 检查是否在冷却期
            if let Some(cooldown_end) = self.cooldowns.get(key) {
                if Instant::now() < *cooldown_end {
                    attempts += 1;
                    continue;
                } else {
                    self.cooldowns.remove(key);
                }
            }

            // 检查使用限制
            if self.max_usage_per_key > 0 {
                if let Some(usage) = self.usage_counts.get(key) {
                    if usage.load(Ordering::Relaxed) >= self.max_usage_per_key {
                        self.cooldowns.insert(
                            key.clone(),
                            Instant::now() + std::time::Duration::from_secs(3600),
                        );
                        attempts += 1;
                        continue;
                    }
                }
            }

            // 记录使用
            if let Some(usage) = self.usage_counts.get(key) {
                usage.fetch_add(1, Ordering::Relaxed);
            }

            return Some(key.clone());
        }

        None
    }

    /// 将某个 key 置于冷却期
    pub fn cooldown_key(&self, key: &str, duration: std::time::Duration) {
        self.cooldowns
            .insert(key.to_string(), Instant::now() + duration);
    }

    /// 重置使用计数
    pub fn reset_usage(&self) {
        for entry in self.usage_counts.iter() {
            entry.value().store(0, Ordering::Relaxed);
        }
    }

    /// 获取密钥数量
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// 获取所有密钥（只读）
    pub fn get_keys(&self) -> Vec<String> {
        self.keys.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_round_robin() {
        let pool = ApiKeyPool::new(vec!["key1".to_string(), "key2".to_string(), "key3".to_string()]);

        assert_eq!(pool.get_next_key(), Some("key1".to_string()));
        assert_eq!(pool.get_next_key(), Some("key2".to_string()));
        assert_eq!(pool.get_next_key(), Some("key3".to_string()));
        assert_eq!(pool.get_next_key(), Some("key1".to_string()));
    }

    #[test]
    fn test_empty_pool() {
        let pool = ApiKeyPool::new(vec![]);
        assert_eq!(pool.get_next_key(), None);
    }

    #[test]
    fn test_from_csv() {
        let pool = ApiKeyPool::from_csv("key1,key2,key3");
        assert_eq!(pool.len(), 3);
    }

    #[test]
    fn test_cooldown() {
        let pool = ApiKeyPool::new(vec!["key1".to_string(), "key2".to_string()]);

        pool.cooldown_key("key1", Duration::from_secs(1));

        assert_eq!(pool.get_next_key(), Some("key2".to_string()));
    }
}
