//! 轮询监听模块
//!
//! 支持主动轮询工具执行并检查结果触发条件

use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};

use crate::infrastructure::tool::ToolExecutor;
use crate::domain::tool::ToolCallContext;
use crate::domain::TriggerCondition;

/// 轮询事件
#[derive(Debug, Clone)]
pub struct PollingTick {
    pub rule_id: String,
    pub target_agent_id: String,
    pub tool_result: Value,
    pub triggered_at: DateTime<Utc>,
}

/// 轮询规则
#[derive(Debug, Clone)]
pub struct PollingRule {
    /// 规则 ID
    pub id: String,
    /// 轮询的工具 ID
    pub tool_id: String,
    /// 工具参数
    pub params: Value,
    /// 轮询间隔（秒）
    pub interval_secs: u64,
    /// 触发条件
    pub condition: TriggerCondition,
    /// 目标 Agent ID
    pub target_agent_id: String,
    /// 规则是否启用
    pub enabled: bool,
    /// 上次轮询时间
    pub last_polled: Option<DateTime<Utc>>,
    /// 上次轮询结果
    pub last_result: Option<Value>,
    /// 用户定义的标签
    pub tags: Vec<String>,
}

impl PollingRule {
    /// 创建新的轮询规则
    pub fn new(
        id: impl Into<String>,
        tool_id: impl Into<String>,
        params: Value,
        interval_secs: u64,
        condition: TriggerCondition,
        target_agent_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            tool_id: tool_id.into(),
            params,
            interval_secs,
            condition,
            target_agent_id: target_agent_id.into(),
            enabled: true,
            last_polled: None,
            last_result: None,
            tags: vec![],
        }
    }

    /// 添加标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 检查是否应该轮询
    pub fn should_poll(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }

        match self.last_polled {
            Some(last) => {
                let elapsed = now.signed_duration_since(last).num_seconds() as u64;
                elapsed >= self.interval_secs
            }
            None => true, // 从未轮询过，立即轮询
        }
    }

    /// 检查结果是否匹配触发条件
    pub fn matches_condition(&self, result: &Value) -> bool {
        match &self.condition {
            TriggerCondition::NumericRange { min, max } => {
                if let Some(num_val) = result.as_f64() {
                    num_val >= *min && num_val <= *max
                } else {
                    false
                }
            }
            TriggerCondition::StringContains { content } => {
                // 检查字符串值
                if let Some(str_val) = result.as_str() {
                    return str_val.contains(content.as_str());
                }

                // 检查 JSON 对象中是否包含指定内容
                if let Value::Object(obj) = result {
                    for (_, val) in obj {
                        if let Some(str_val) = val.as_str() {
                            if str_val.contains(content) {
                                return true;
                            }
                        }

                        // 如果值是数组，检查数组元素
                        if let Value::Array(arr) = val {
                            for arr_val in arr {
                                if let Some(arr_str) = arr_val.as_str() {
                                    if arr_str.contains(content) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }

                // 将整个 JSON 序列化为字符串再检查
                let json_str = result.to_string();
                json_str.contains(content)
            }
            TriggerCondition::StatusMatches { expected_status } => {
                if let Some(status_val) = result.as_str() {
                    status_val == expected_status
                } else {
                    false
                }
            }
            TriggerCondition::CustomExpression { .. }
            | TriggerCondition::ScheduleInterval { .. }
            | TriggerCondition::ScheduleCron { .. } => {
                // 轮询不支持这些条件类型
                false
            }
        }
    }

    /// 更新轮询时间和结果
    pub fn mark_polled(&mut self, result: Value) {
        self.last_polled = Some(Utc::now());
        self.last_result = Some(result);
    }
}

/// 轮询管理器
pub struct PollingManager {
    /// 轮询规则存储
    rules: DashMap<String, PollingRule>,
    /// 工具执行器
    tool_executor: Arc<dyn ToolExecutor>,
    /// 事件发送器
    tx: broadcast::Sender<PollingTick>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl PollingManager {
    /// 创建新的轮询管理器
    pub fn new(tool_executor: Arc<dyn ToolExecutor>) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            rules: DashMap::new(),
            tool_executor,
            tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 注册轮询规则
    pub fn register_rule(&self, rule: PollingRule) -> Result<()> {
        let rule_id = rule.id.clone();
        self.rules.insert(rule.id.clone(), rule);
        info!("Registered polling rule: {}", rule_id);
        Ok(())
    }

    /// 移除轮询规则
    pub fn remove_rule(&self, rule_id: &str) -> Option<PollingRule> {
        self.rules.remove(rule_id).map(|(_, rule)| rule)
    }

    /// 获取轮询规则
    pub fn get_rule(&self, rule_id: &str) -> Option<PollingRule> {
        self.rules.get(rule_id).map(|r| r.clone())
    }

    /// 检查规则是否存在
    pub fn has_rule(&self, rule_id: &str) -> bool {
        self.rules.contains_key(rule_id)
    }

    /// 获取所有规则
    pub fn list_rules(&self) -> Vec<PollingRule> {
        self.rules.iter().map(|r| r.clone()).collect()
    }

    /// 启用/禁用规则
    pub fn set_rule_enabled(&self, rule_id: &str, enabled: bool) -> bool {
        if let Some(mut rule) = self.rules.get_mut(rule_id) {
            rule.enabled = enabled;
            info!(
                "Polling rule {} is now {}",
                rule_id,
                if enabled { "enabled" } else { "disabled" }
            );
            true
        } else {
            false
        }
    }

    /// 获取事件接收器
    pub fn subscribe(&self) -> broadcast::Receiver<PollingTick> {
        self.tx.subscribe()
    }

    /// 启动后台轮询任务
    pub fn start(&self) {
        let rules = self.rules.clone();
        let tool_executor = self.tool_executor.clone();
        let tx = self.tx.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            *running.write().await = true;
            info!("Polling manager started");

            let mut check_interval = interval(Duration::from_secs(1));

            loop {
                check_interval.tick().await;

                if !*running.read().await {
                    break;
                }

                let now = Utc::now();
                let mut polled_count = 0;

                // 检查所有规则
                for mut rule_ref in rules.iter_mut() {
                    let rule = rule_ref.value_mut();

                    if rule.should_poll(now) {
                        // 执行工具
                        let context = ToolCallContext::new("polling_manager".to_string());

                        match tool_executor
                            .execute(&rule.tool_id, rule.params.clone(), &context)
                            .await
                        {
                            Ok(result) => {
                                debug!(
                                    "Polling rule {} tool {} executed successfully",
                                    rule.id, rule.tool_id
                                );

                                // 检查结果是否匹配条件
                                if rule.matches_condition(&result) {
                                    // 发送触发事件
                                    let tick = PollingTick {
                                        rule_id: rule.id.clone(),
                                        target_agent_id: rule.target_agent_id.clone(),
                                        tool_result: result.clone(),
                                        triggered_at: now,
                                    };

                                    if let Err(e) = tx.send(tick) {
                                        error!("Failed to send polling tick: {}", e);
                                    } else {
                                        debug!("Polling rule triggered: {}", rule.id);
                                        polled_count += 1;
                                    }
                                }

                                // 更新轮询状态
                                rule.mark_polled(result);
                            }
                            Err(e) => {
                                error!(
                                    "Polling rule {} tool {} execution failed: {}",
                                    rule.id, rule.tool_id, e
                                );
                                // 即使失败也更新轮询时间，避免频繁重试
                                rule.last_polled = Some(Utc::now());
                            }
                        }
                    }
                }

                if polled_count > 0 {
                    debug!("Triggered {} polling rules", polled_count);
                }
            }

            info!("Polling manager stopped");
        });
    }

    /// 停止后台轮询任务
    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("Polling manager stopping...");
    }

    /// 检查是否运行中
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

impl Default for PollingManager {
    fn default() -> Self {
        // 创建一个空的工具执行器用于默认实现
        // 实际使用时应该使用 new() 方法传入真实的工具执行器
        unimplemented!("Default implementation not available. Use new() instead.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 模拟工具执行器用于测试
    struct MockToolExecutor {
        return_value: Value,
    }

    impl MockToolExecutor {
        fn new(return_value: Value) -> Self {
            Self { return_value }
        }
    }

    #[async_trait::async_trait]
    impl ToolExecutor for MockToolExecutor {
        async fn execute(
            &self,
            _tool_id: &str,
            _params: Value,
            _context: &ToolCallContext,
        ) -> Result<Value> {
            Ok(self.return_value.clone())
        }

        fn can_execute(&self, _tool_id: &str) -> bool {
            true
        }

        fn supported_tools(&self) -> Vec<String> {
            vec!["mock_tool".to_string()]
        }
    }

    #[test]
    fn test_polling_rule_should_poll() {
        let mut rule = PollingRule::new(
            "test",
            "mock_tool",
            json!({}),
            60,
            TriggerCondition::NumericRange {
                min: 0.0,
                max: 100.0,
            },
            "agent1",
        );

        // 从未轮询过，应该立即轮询
        assert!(rule.should_poll(Utc::now()));

        // 手动设置上次轮询时间
        rule.last_polled = Some(Utc::now());
        assert!(!rule.should_poll(Utc::now()));

        // 禁用后不应该轮询
        rule.enabled = false;
        assert!(!rule.should_poll(Utc::now()));
    }

    #[test]
    fn test_polling_rule_matches_condition() {
        let rule = PollingRule::new(
            "test",
            "mock_tool",
            json!({}),
            60,
            TriggerCondition::NumericRange {
                min: 20.0,
                max: 30.0,
            },
            "agent1",
        );

        // 在范围内的值应该匹配
        assert!(rule.matches_condition(&json!(25.0)));
        assert!(rule.matches_condition(&json!(20.0)));
        assert!(rule.matches_condition(&json!(30.0)));

        // 超出范围的值不应该匹配
        assert!(!rule.matches_condition(&json!(10.0)));
        assert!(!rule.matches_condition(&json!(40.0)));
    }

    #[test]
    fn test_polling_rule_string_contains() {
        let rule = PollingRule::new(
            "test",
            "mock_tool",
            json!({}),
            60,
            TriggerCondition::StringContains {
                content: "hello".to_string(),
            },
            "agent1",
        );

        // 包含 "hello" 的字符串应该匹配
        assert!(rule.matches_condition(&json!("hello world")));
        assert!(rule.matches_condition(&json!("say hello")));

        // 不包含 "hello" 的字符串不应该匹配
        assert!(!rule.matches_condition(&json!("goodbye")));
    }

    #[tokio::test]
    async fn test_polling_manager() {
        let executor = Arc::new(MockToolExecutor::new(json!(25.0)));
        let manager = PollingManager::new(executor);

        // 注册规则
        let rule = PollingRule::new(
            "test_poll",
            "mock_tool",
            json!({}),
            1, // 1 秒间隔
            TriggerCondition::NumericRange {
                min: 20.0,
                max: 30.0,
            },
            "test_agent",
        );
        manager.register_rule(rule).unwrap();

        // 订阅事件
        let mut rx = manager.subscribe();

        // 启动轮询器
        manager.start();

        // 等待触发
        let tick = tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .expect("Timeout waiting for polling tick")
            .expect("Channel closed");

        assert_eq!(tick.rule_id, "test_poll");
        assert_eq!(tick.target_agent_id, "test_agent");

        // 停止轮询器
        manager.stop().await;
    }
}
