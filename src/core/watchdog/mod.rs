//! Watchdog框架 - 通用监控和事件触发系统
//!
//! 提供统一的工具执行监控和事件触发能力

use std::sync::Arc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{debug, error, info};

use crate::domain::tool::ToolCallContext;

pub mod client;
pub mod condition;
pub mod rule;

/// 轮询配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingConfig {
    /// 轮询间隔（毫秒）
    pub interval_ms: u64,
    /// 是否启用轮询
    pub enabled: bool,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            interval_ms: 5000, // 默认5秒轮询一次
            enabled: true,
            timeout_ms: 30000, // 默认30秒超时
        }
    }
}

/// 触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// 数值范围条件：当工具返回数值在指定范围内时触发
    NumericRange {
        min: f64,
        max: f64,
    },
    /// 字符串匹配条件：当工具返回字符串包含指定内容时触发
    StringContains {
        content: String,
    },
    /// 状态匹配条件：当工具返回特定状态时触发
    StatusMatches {
        expected_status: String,
    },
    /// 自定义表达式条件：使用表达式语言定义复杂条件
    CustomExpression {
        expression: String,
    },
}

/// 工具执行事件
#[derive(Debug, Clone)]
pub enum ToolExecutionEvent {
    /// 工具执行前
    PreExecute {
        tool_id: String,
        params: serde_json::Value,
        context: ToolCallContext,
    },
    /// 工具执行后（成功）
    PostExecute {
        tool_id: String,
        result: serde_json::Value,
        context: ToolCallContext,
    },
    /// 工具执行错误
    Error {
        tool_id: String,
        error: String,
        context: ToolCallContext,
    },
}

/// 可轮询的监控规则
#[derive(Debug, Clone)]
pub struct PollableWatchdogRule {
    /// 基础规则
    pub base_rule: WatchdogRule,
    /// 轮询配置
    pub polling_config: PollingConfig,
    /// 上次检查时间
    pub last_check: Option<tokio::time::Instant>,
}

/// 监控规则
#[derive(Debug, Clone)]
pub struct WatchdogRule {
    /// 规则ID
    pub id: String,
    /// 监控的工具ID
    pub tool_id: String,
    /// 触发条件
    pub condition: TriggerCondition,
    /// 关联的Agent ID（触发时通知该Agent）
    pub target_agent_id: String,
    /// 规则是否启用
    pub enabled: bool,
    /// 用户定义的标签
    pub tags: Vec<String>,
}

impl WatchdogRule {
    /// 创建新的监控规则
    pub fn new(
        id: impl Into<String>,
        tool_id: impl Into<String>,
        condition: TriggerCondition,
        target_agent_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            tool_id: tool_id.into(),
            condition,
            target_agent_id: target_agent_id.into(),
            enabled: true,
            tags: vec![],
        }
    }

    /// 添加标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 检查规则是否应该触发
    pub fn should_trigger(&self, event: &ToolExecutionEvent) -> bool {
        if !self.enabled {
            return false;
        }

        match event {
            ToolExecutionEvent::PostExecute { tool_id, result, .. } if tool_id == &self.tool_id => {
                self.evaluate_condition(result)
            }
            _ => false,
        }
    }

    /// 评估条件
    fn evaluate_condition(&self, result: &serde_json::Value) -> bool {
        // 使用condition模块中的评估器
        let evaluator = condition::ConditionEvaluator;
        evaluator.evaluate_condition(&self.condition, result)
    }
}

/// 事件处理器
pub trait EventHandler: Send + Sync {
    fn handle_event(&self, event: &ToolExecutionEvent) -> Result<()>;
}

/// 默认事件处理器
pub struct DefaultEventHandler;

impl EventHandler for DefaultEventHandler {
    fn handle_event(&self, event: &ToolExecutionEvent) -> Result<()> {
        match event {
            ToolExecutionEvent::PostExecute { tool_id, result, context: _ } => {
                debug!("Tool {} executed successfully with result: {:?}", tool_id, result);
                // 这里可以添加具体的处理逻辑
            }
            ToolExecutionEvent::Error { tool_id, error, context: _ } => {
                error!("Tool {} execution failed: {}", tool_id, error);
            }
            _ => {}
        }
        Ok(())
    }
}

/// 事件分发器
pub struct EventDispatcher {
    handlers: DashMap<String, Arc<dyn EventHandler>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }

    /// 注册事件处理器
    pub fn register_handler(&self, name: impl Into<String>, handler: Arc<dyn EventHandler>) {
        self.handlers.insert(name.into(), handler);
    }

    /// 分发事件
    pub async fn dispatch(&self, event: &ToolExecutionEvent) {
        for handler in self.handlers.iter() {
            if let Err(e) = handler.value().handle_event(event) {
                error!("Error in event handler: {}", e);
            }
        }
    }
}

/// Watchdog框架核心
pub struct WatchdogFramework {
    /// 监控规则存储
    rules: DashMap<String, WatchdogRule>,
    /// 事件分发器
    event_dispatcher: Arc<EventDispatcher>,
    /// 全局启用状态
    enabled: Arc<RwLock<bool>>,
}

impl WatchdogFramework {
    /// 创建新的Watchdog框架
    pub fn new() -> Self {
        Self {
            rules: DashMap::new(),
            event_dispatcher: Arc::new(EventDispatcher::new()),
            enabled: Arc::new(RwLock::new(true)),
        }
    }

    /// 注册监控规则
    pub fn register_rule(&self, rule: WatchdogRule) -> Result<()> {
        // 直接在框架的存储中注册规则
        self.rules.insert(rule.id.clone(), rule);
        Ok(())
    }

    /// 移除监控规则
    pub fn remove_rule(&self, rule_id: &str) -> Option<WatchdogRule> {
        // 这里需要改进，目前使用简单实现
        self.rules.remove(rule_id).map(|(_, rule)| rule)
    }

    /// 获取监控规则
    pub fn get_rule(&self, rule_id: &str) -> Option<WatchdogRule> {
        self.rules.get(rule_id).map(|r| r.clone())
    }

    /// 检查规则是否存在
    pub fn has_rule(&self, rule_id: &str) -> bool {
        self.rules.contains_key(rule_id)
    }

    /// 获取所有规则
    pub fn list_rules(&self) -> Vec<WatchdogRule> {
        self.rules.iter().map(|r| r.clone()).collect()
    }

    /// 启用/禁用规则
    pub fn set_rule_enabled(&self, rule_id: &str, enabled: bool) -> bool {
        if let Some(mut rule) = self.rules.get_mut(rule_id) {
            rule.enabled = enabled;
            info!("Rule {} is now {}", rule_id, if enabled { "enabled" } else { "disabled" });
            true
        } else {
            false
        }
    }

    /// 检查框架是否启用
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// 启用/禁用整个框架
    pub async fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().await = enabled;
        info!("Watchdog framework is now {}", if enabled { "enabled" } else { "disabled" });
    }

    /// 处理工具执行事件
    pub async fn process_event(&self, event: &ToolExecutionEvent) -> Result<Vec<String>> {
        if !*self.enabled.read().await {
            return Ok(vec![]);
        }

        // 分发事件到所有处理器
        self.event_dispatcher.dispatch(event).await;

        // 检查事件是否匹配任何规则
        let matched_rule_ids: Vec<String> = self.rules
            .iter()
            .filter(|rule| rule.should_trigger(event))
            .map(|rule| rule.id.clone())
            .collect();

        // 收集被触发的Agent ID
        let mut triggered_agents = Vec::new();
        for rule_id in matched_rule_ids {
            if let Some(rule) = self.rules.get(&rule_id) {
                triggered_agents.push(rule.target_agent_id.clone());
                info!("Rule {} triggered for agent {}", rule.id, rule.target_agent_id);
            }
        }

        Ok(triggered_agents)
    }

    /// 获取事件分发器引用
    pub fn event_dispatcher(&self) -> Arc<EventDispatcher> {
        self.event_dispatcher.clone()
    }
}

impl Default for WatchdogFramework {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_watchdog_rule_numeric_range() {
        let rule = WatchdogRule::new(
            "test_rule",
            "test_tool",
            TriggerCondition::NumericRange { min: 10.0, max: 20.0 },
            "test_agent",
        );

        // 测试在范围内的数值
        assert!(rule.should_trigger(&ToolExecutionEvent::PostExecute {
            tool_id: "test_tool".to_string(),
            result: json!(15.0),
            context: ToolCallContext::new("test_caller".to_string()),
        }));

        // 测试超出范围的数值
        assert!(!rule.should_trigger(&ToolExecutionEvent::PostExecute {
            tool_id: "test_tool".to_string(),
            result: json!(25.0),
            context: ToolCallContext::new("test_caller".to_string()),
        }));
    }

    #[tokio::test]
    async fn test_watchdog_rule_string_contains() {
        let rule = WatchdogRule::new(
            "test_rule",
            "test_tool",
            TriggerCondition::StringContains { content: "success".to_string() },
            "test_agent",
        );

        // 测试包含目标字符串
        assert!(rule.should_trigger(&ToolExecutionEvent::PostExecute {
            tool_id: "test_tool".to_string(),
            result: json!("operation was successful"),
            context: ToolCallContext::new("test_caller".to_string()),
        }));

        // 测试不包含目标字符串
        assert!(!rule.should_trigger(&ToolExecutionEvent::PostExecute {
            tool_id: "test_tool".to_string(),
            result: json!("operation failed"),
            context: ToolCallContext::new("test_caller".to_string()),
        }));
    }

    #[tokio::test]
    async fn test_watchdog_framework() {
        let framework = WatchdogFramework::new();

        let rule = WatchdogRule::new(
            "test_rule",
            "test_tool",
            TriggerCondition::NumericRange { min: 10.0, max: 20.0 },
            "test_agent",
        );

        framework.register_rule(rule).unwrap();

        // 测试事件处理
        let triggered = framework.process_event(&ToolExecutionEvent::PostExecute {
            tool_id: "test_tool".to_string(),
            result: json!(15.0),
            context: ToolCallContext::new("test_caller".to_string()),
        }).await.unwrap();

        assert_eq!(triggered, vec!["test_agent"]);
    }
}