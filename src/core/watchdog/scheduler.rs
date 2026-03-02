//! 定时调度器模块
//!
//! 支持基于时间间隔和 cron 表达式的定时任务触发

use anyhow::Result;
use chrono::{DateTime, Utc, Datelike, Timelike};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};


/// 定时任务 tick 事件
#[derive(Debug, Clone)]
pub struct ScheduleTick {
    pub rule_id: String,
    pub target_agent_id: String,
    pub triggered_at: DateTime<Utc>,
}

/// 定时任务类型
#[derive(Debug, Clone)]
pub enum ScheduleType {
    /// 固定时间间隔（秒）
    Interval { seconds: u64 },
    /// Cron 表达式（格式：分 时 日 月 星期）
    Cron { cron_expression: String },
}

/// 定时任务规则
#[derive(Debug, Clone)]
pub struct ScheduleRule {
    /// 规则 ID
    pub id: String,
    /// 定时任务类型
    pub schedule_type: ScheduleType,
    /// 目标 Agent ID
    pub target_agent_id: String,
    /// 规则是否启用
    pub enabled: bool,
    /// 上次触发时间
    pub last_triggered: Option<DateTime<Utc>>,
    /// 用户定义的标签
    pub tags: Vec<String>,
}

impl ScheduleRule {
    /// 创建新的定时任务规则（间隔模式）
    pub fn new_interval(
        id: impl Into<String>,
        seconds: u64,
        target_agent_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            schedule_type: ScheduleType::Interval { seconds },
            target_agent_id: target_agent_id.into(),
            enabled: true,
            last_triggered: None,
            tags: vec![],
        }
    }

    /// 创建新的定时任务规则（Cron 模式）
    pub fn new_cron(
        id: impl Into<String>,
        cron_expression: impl Into<String>,
        target_agent_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            schedule_type: ScheduleType::Cron {
                cron_expression: cron_expression.into(),
            },
            target_agent_id: target_agent_id.into(),
            enabled: true,
            last_triggered: None,
            tags: vec![],
        }
    }

    /// 添加标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 检查是否应该触发
    pub fn should_trigger(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }

        match &self.schedule_type {
            ScheduleType::Interval { seconds } => {
                match self.last_triggered {
                    Some(last) => {
                        let elapsed = now.signed_duration_since(last).num_seconds() as u64;
                        elapsed >= *seconds
                    }
                    None => true, // 从未触发过，立即触发
                }
            }
            ScheduleType::Cron { cron_expression } => {
                // 简单实现：检查当前时间是否匹配 cron 表达式
                // 格式：分 时 日 月 星期
                Self::matches_cron(now, cron_expression)
            }
        }
    }

    /// 检查时间是否匹配 cron 表达式
    fn matches_cron(now: DateTime<Utc>, cron_expression: &str) -> bool {
        let parts: Vec<&str> = cron_expression.split_whitespace().collect();
        if parts.len() != 5 {
            warn!("Invalid cron expression: {}", cron_expression);
            return false;
        }

        let minute = now.minute();
        let hour = now.hour();
        let day = now.day();
        let month = now.month();
        let weekday = now.weekday().num_days_from_sunday();

        let matches_field = |field: &str, value: u32| -> bool {
            if field == "*" {
                return true;
            }

            // 处理列表 (e.g., "1,3,5")
            if field.contains(',') {
                return field
                    .split(',')
                    .filter_map(|s| s.parse::<u32>().ok())
                    .any(|v| v == value);
            }

            // 处理范围 (e.g., "1-5")
            if field.contains('-') {
                if let Some((start, end)) = field.split_once('-') {
                    if let (Ok(start), Ok(end)) = (start.parse::<u32>(), end.parse::<u32>()) {
                        return value >= start && value <= end;
                    }
                }
            }

            // 处理步长 (e.g., "*/5")
            if field.starts_with("*/") {
                if let Some(step) = field.strip_prefix("*/").and_then(|s| s.parse::<u32>().ok()) {
                    if step > 0 {
                        return value % step == 0;
                    }
                }
            }

            // 精确匹配
            field.parse::<u32>() == Ok(value)
        };

        matches_field(parts[0], minute)
            && matches_field(parts[1], hour)
            && matches_field(parts[2], day)
            && matches_field(parts[3], month)
            && matches_field(parts[4], weekday)
    }

    /// 更新触发时间
    pub fn mark_triggered(&mut self) {
        self.last_triggered = Some(Utc::now());
    }
}

/// 定时任务管理器
pub struct ScheduleManager {
    /// 定时任务规则存储
    rules: DashMap<String, ScheduleRule>,
    /// 事件发送器
    tx: broadcast::Sender<ScheduleTick>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl ScheduleManager {
    /// 创建新的定时任务管理器
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            rules: DashMap::new(),
            tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 注册定时任务规则
    pub fn register_rule(&self, rule: ScheduleRule) -> Result<()> {
        let rule_id = rule.id.clone();
        self.rules.insert(rule.id.clone(), rule);
        info!("Registered schedule rule: {}", rule_id);
        Ok(())
    }

    /// 移除定时任务规则
    pub fn remove_rule(&self, rule_id: &str) -> Option<ScheduleRule> {
        self.rules.remove(rule_id).map(|(_, rule)| rule)
    }

    /// 获取定时任务规则
    pub fn get_rule(&self, rule_id: &str) -> Option<ScheduleRule> {
        self.rules.get(rule_id).map(|r| r.clone())
    }

    /// 检查规则是否存在
    pub fn has_rule(&self, rule_id: &str) -> bool {
        self.rules.contains_key(rule_id)
    }

    /// 获取所有规则
    pub fn list_rules(&self) -> Vec<ScheduleRule> {
        self.rules.iter().map(|r| r.clone()).collect()
    }

    /// 启用/禁用规则
    pub fn set_rule_enabled(&self, rule_id: &str, enabled: bool) -> bool {
        if let Some(mut rule) = self.rules.get_mut(rule_id) {
            rule.enabled = enabled;
            info!(
                "Schedule rule {} is now {}",
                rule_id,
                if enabled { "enabled" } else { "disabled" }
            );
            true
        } else {
            false
        }
    }

    /// 获取事件接收器
    pub fn subscribe(&self) -> broadcast::Receiver<ScheduleTick> {
        self.tx.subscribe()
    }

    /// 启动后台调度任务
    pub fn start(&self) {
        let rules = self.rules.clone();
        let tx = self.tx.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            *running.write().await = true;
            info!("Schedule manager started");

            let mut check_interval = interval(Duration::from_secs(1));

            loop {
                check_interval.tick().await;

                if !*running.read().await {
                    break;
                }

                let now = Utc::now();
                let mut triggered_count = 0;

                // 检查所有规则
                for mut rule_ref in rules.iter_mut() {
                    let rule = rule_ref.value_mut();

                    if rule.should_trigger(now) {
                        // 发送触发事件
                        let tick = ScheduleTick {
                            rule_id: rule.id.clone(),
                            target_agent_id: rule.target_agent_id.clone(),
                            triggered_at: now,
                        };

                        if let Err(e) = tx.send(tick) {
                            error!("Failed to send schedule tick: {}", e);
                        } else {
                            debug!("Schedule rule triggered: {}", rule.id);
                            rule.mark_triggered();
                            triggered_count += 1;
                        }
                    }
                }

                if triggered_count > 0 {
                    debug!("Triggered {} schedule rules", triggered_count);
                }
            }

            info!("Schedule manager stopped");
        });
    }

    /// 停止后台调度任务
    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("Schedule manager stopping...");
    }

    /// 检查是否运行中
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

impl Default for ScheduleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_rule_should_trigger() {
        let mut rule = ScheduleRule::new_interval("test", 60, "agent1");

        // 从未触发过，应该立即触发
        assert!(rule.should_trigger(Utc::now()));

        // 手动设置上次触发时间
        rule.last_triggered = Some(Utc::now());
        assert!(!rule.should_trigger(Utc::now()));

        // 禁用后不应该触发
        rule.enabled = false;
        assert!(!rule.should_trigger(Utc::now()));
    }

    #[test]
    fn test_cron_expression_matching() {
        let now = Utc::now();

        // 每分钟
        assert!(ScheduleRule::matches_cron(now, "* * * * *"));

        // 每小时
        let cron_hourly = format!("{} * * * *", now.minute());
        assert!(ScheduleRule::matches_cron(now, &cron_hourly));

        // 精确时间匹配
        let cron_exact = format!("{} {} * * *", now.minute(), now.hour());
        assert!(ScheduleRule::matches_cron(now, &cron_exact));
    }

    #[tokio::test]
    async fn test_schedule_manager() {
        let manager = ScheduleManager::new();

        // 注册规则
        let rule = ScheduleRule::new_interval("test_interval", 1, "test_agent");
        manager.register_rule(rule).unwrap();

        // 订阅事件
        let mut rx = manager.subscribe();

        // 启动调度器
        manager.start();

        // 等待触发
        let tick = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("Timeout waiting for schedule tick")
            .expect("Channel closed");

        assert_eq!(tick.rule_id, "test_interval");
        assert_eq!(tick.target_agent_id, "test_agent");

        // 停止调度器
        manager.stop().await;
    }
}
