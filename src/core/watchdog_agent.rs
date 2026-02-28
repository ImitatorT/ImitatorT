//! Watchdog Agent - System agent that monitors tool executions and triggers other agents

use anyhow::Result;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::domain::{tool::ToolCallContext, Agent, TriggerCondition};

/// 工具执行事件
#[derive(Debug, Clone)]
pub enum ToolExecutionEvent {
    /// 工具执行前
    PreExecute {
        tool_id: String,
        params: Value,
        context: ToolCallContext,
    },
    /// 工具执行后（成功）
    PostExecute {
        tool_id: String,
        result: Value,
        context: ToolCallContext,
    },
    /// 工具执行错误
    Error {
        tool_id: String,
        error: String,
        context: ToolCallContext,
    },
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
            ToolExecutionEvent::PostExecute {
                tool_id, result, ..
            } if tool_id == &self.tool_id => self.evaluate_condition(result),
            _ => false,
        }
    }

    /// 评估条件
    fn evaluate_condition(&self, result: &Value) -> bool {
        // 评估触发条件
        match &self.condition {
            TriggerCondition::NumericRange { min, max } => {
                if let Some(num_val) = result.as_f64() {
                    num_val >= *min && num_val <= *max
                } else {
                    false
                }
            }
            TriggerCondition::StringContains { content } => {
                if let Some(str_val) = result.as_str() {
                    str_val.contains(content.as_str())
                } else {
                    false
                }
            }
            TriggerCondition::StatusMatches { expected_status } => {
                if let Some(status_val) = result.as_str() {
                    status_val == expected_status
                } else {
                    false
                }
            }
            TriggerCondition::CustomExpression { .. } => {
                // 简单实现，实际应用中可能需要更复杂的表达式解析
                false
            }
        }
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
            ToolExecutionEvent::PostExecute {
                tool_id,
                result,
                context: _,
            } => {
                debug!(
                    "Tool {} executed successfully with result: {:?}",
                    tool_id, result
                );
                // 这里可以添加具体的处理逻辑
            }
            ToolExecutionEvent::Error {
                tool_id,
                error,
                context: _,
            } => {
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

impl Default for EventDispatcher {
    fn default() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self::default()
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

/// Watchdog Agent - 作为系统内置Agent永久运行
pub struct WatchdogAgent {
    /// 监控规则存储
    rules: DashMap<String, WatchdogRule>,
    /// 事件分发器
    event_dispatcher: Arc<EventDispatcher>,
    /// 全局启用状态
    enabled: Arc<RwLock<bool>>,
    /// Agent实例
    agent: Agent,
}

impl WatchdogAgent {
    /// 创建新的Watchdog Agent
    pub fn new(agent: Agent) -> Self {
        Self {
            rules: DashMap::new(),
            event_dispatcher: Arc::new(EventDispatcher::new()),
            enabled: Arc::new(RwLock::new(true)),
            agent,
        }
    }

    /// 注册监控规则
    pub fn register_rule(&self, rule: WatchdogRule) -> Result<()> {
        self.rules.insert(rule.id.clone(), rule);
        Ok(())
    }

    /// 移除监控规则
    pub fn remove_rule(&self, rule_id: &str) -> Option<WatchdogRule> {
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
            info!(
                "Rule {} is now {}",
                rule_id,
                if enabled { "enabled" } else { "disabled" }
            );
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
        info!(
            "Watchdog agent is now {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }

    /// 处理工具执行事件
    pub async fn process_event(&self, event: &ToolExecutionEvent) -> Result<Vec<String>> {
        if !*self.enabled.read().await {
            return Ok(vec![]);
        }

        // 分发事件到所有处理器
        self.event_dispatcher.dispatch(event).await;

        // 检查事件是否匹配任何规则
        let matched_rule_ids: Vec<String> = self
            .rules
            .iter()
            .filter(|rule| rule.should_trigger(event))
            .map(|rule| rule.id.clone())
            .collect();

        // 收集被触发的Agent ID
        let mut triggered_agents = Vec::new();
        for rule_id in matched_rule_ids {
            if let Some(rule) = self.rules.get(&rule_id) {
                triggered_agents.push(rule.target_agent_id.clone());
                info!(
                    "Rule {} triggered for agent {}",
                    rule.id, rule.target_agent_id
                );
            }
        }

        Ok(triggered_agents)
    }

    /// 获取事件分发器引用
    pub fn event_dispatcher(&self) -> Arc<EventDispatcher> {
        self.event_dispatcher.clone()
    }

    /// 获取Agent实例
    pub fn agent(&self) -> &Agent {
        &self.agent
    }
}

impl WatchdogAgent {
    /// 为Agent注册私聊唤醒事件
    pub fn register_direct_message_watcher(&self, agent_id: &str) -> Result<()> {
        let rule = WatchdogRule::new(
            format!("direct_msg_{}", agent_id),
            "message.send_direct".to_string(), // 私聊消息工具
            TriggerCondition::StringContains {
                content: format!("\"target\":\"{}\"", agent_id), // 当消息目标包含agent_id时触发
            },
            agent_id.to_string(),
        );

        self.register_rule(rule)
    }

    /// 为Agent注册艾特(@)唤醒事件
    pub fn register_mention_watcher(&self, agent_id: &str) -> Result<()> {
        let rule = WatchdogRule::new(
            format!("mention_{}", agent_id),
            "message.send_group".to_string(), // 群聊消息工具
            TriggerCondition::StringContains {
                content: format!("\"mention_agent_ids\":[\"{}\"", agent_id), // 当艾特列表包含agent_id时触发
            },
            agent_id.to_string(),
        );

        self.register_rule(rule)
    }

    /// 为Agent注册默认唤醒事件（私聊和艾特）
    pub fn register_default_watchers(&self, agent_id: &str) -> Result<()> {
        self.register_direct_message_watcher(agent_id)?;
        self.register_mention_watcher(agent_id)?;
        Ok(())
    }
}

impl Clone for WatchdogAgent {
    fn clone(&self) -> Self {
        Self {
            rules: self.rules.clone(),
            event_dispatcher: self.event_dispatcher.clone(),
            enabled: self.enabled.clone(),
            agent: self.agent.clone(),
        }
    }
}

/// Client for interacting with the watchdog system
pub struct WatchdogClient {
    watchdog_agent: Arc<WatchdogAgent>,
}

impl WatchdogClient {
    /// Create a new client
    pub fn new(watchdog_agent: Arc<WatchdogAgent>) -> Self {
        Self { watchdog_agent }
    }

    /// Register a rule to watch a specific tool
    pub fn register_tool_watcher(
        &self,
        agent_id: &str,
        tool_id: &str,
        condition: TriggerCondition,
    ) -> anyhow::Result<()> {
        let rule = WatchdogRule::new(
            format!("rule_{}_{}", agent_id, tool_id),
            tool_id.to_string(),
            condition,
            agent_id.to_string(),
        );

        self.watchdog_agent.register_rule(rule)
    }
}
