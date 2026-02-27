//! Watchdog客户端 - Agent与Watchdog框架的接口
//!
//! 提供Agent注册监控规则和接收触发事件的能力

use std::sync::Arc;
use anyhow::Result;
use tracing::info;

use super::{WatchdogFramework, WatchdogRule, TriggerCondition, ToolExecutionEvent};

/// Watchdog客户端
pub struct WatchdogClient {
    framework: Arc<WatchdogFramework>,
    agent_id: String,
}

impl WatchdogClient {
    /// 创建新的Watchdog客户端
    pub fn new(framework: Arc<WatchdogFramework>, agent_id: impl Into<String>) -> Self {
        Self {
            framework,
            agent_id: agent_id.into(),
        }
    }

    /// 注册监控规则
    pub async fn register_rule(
        &self,
        rule_id: impl Into<String>,
        tool_id: impl Into<String>,
        condition: TriggerCondition,
    ) -> Result<()> {
        let rule = WatchdogRule::new(
            rule_id,
            tool_id,
            condition,
            self.agent_id.clone(),
        );

        self.framework.register_rule(rule)?;
        info!("Agent {} registered watchdog rule", self.agent_id);
        Ok(())
    }

    /// 注销监控规则
    pub async fn unregister_rule(&self, rule_id: &str) -> bool {
        self.framework.remove_rule(rule_id).is_some()
    }

    /// 启用/禁用规则
    pub async fn set_rule_enabled(&self, rule_id: &str, enabled: bool) -> bool {
        self.framework.set_rule_enabled(rule_id, enabled)
    }

    /// 检查规则是否存在
    pub async fn has_rule(&self, rule_id: &str) -> bool {
        self.framework.has_rule(rule_id)
    }

    /// 获取所有规则
    pub async fn list_rules(&self) -> Vec<WatchdogRule> {
        self.framework.list_rules()
    }

    /// 获取框架引用
    pub fn framework(&self) -> &Arc<WatchdogFramework> {
        &self.framework
    }

    /// 处理工具执行事件（通常由框架调用）
    pub async fn handle_event(&self, event: &ToolExecutionEvent) -> Result<Vec<String>> {
        self.framework.process_event(event).await
    }
}