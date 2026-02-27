//! Watchdog规则管理
//!
//! 提供规则的高级管理功能

use dashmap::DashMap;
use anyhow::Result;
use tracing::info;

use super::{WatchdogRule, ToolExecutionEvent};

/// 规则管理器
pub struct RuleManager {
    rules: DashMap<String, WatchdogRule>,
    /// 规则标签索引
    tag_index: DashMap<String, Vec<String>>,
}

impl RuleManager {
    /// 创建新的规则管理器
    pub fn new() -> Self {
        Self {
            rules: DashMap::new(),
            tag_index: DashMap::new(),
        }
    }

    /// 注册规则
    pub fn register_rule(&self, rule: WatchdogRule) -> Result<()> {
        let rule_id = rule.id.clone();

        // 检查规则是否已存在
        if self.rules.contains_key(&rule_id) {
            return Err(anyhow::anyhow!("Rule already exists: {}", rule_id));
        }

        // 存储规则
        self.rules.insert(rule_id.clone(), rule.clone());

        // 更新标签索引
        for tag in &rule.tags {
            let mut tag_rules = self.tag_index.entry(tag.clone()).or_insert_with(Vec::new);
            tag_rules.push(rule_id.clone());
        }

        info!("Registered rule: {} with tags: {:?}", rule_id, rule.tags);
        Ok(())
    }

    /// 获取规则
    pub fn get_rule(&self, rule_id: &str) -> Option<WatchdogRule> {
        self.rules.get(rule_id).map(|r| r.clone())
    }

    /// 移除规则
    pub fn remove_rule(&self, rule_id: &str) -> Option<WatchdogRule> {
        if let Some((_, rule)) = self.rules.remove(rule_id) {
            // 从标签索引中移除
            for tag in &rule.tags {
                if let Some(mut tag_rules) = self.tag_index.get_mut(tag) {
                    tag_rules.retain(|id| id != rule_id);
                }
            }

            info!("Removed rule: {}", rule_id);
            Some(rule)
        } else {
            None
        }
    }

    /// 列出所有规则
    pub fn list_rules(&self) -> Vec<WatchdogRule> {
        self.rules.iter().map(|r| r.clone()).collect()
    }

    /// 根据标签查找规则
    pub fn find_rules_by_tag(&self, tag: &str) -> Vec<WatchdogRule> {
        let rule_ids = self.tag_index.get(tag).map(|ids| ids.clone()).unwrap_or_default();

        rule_ids
            .iter()
            .filter_map(|id| self.rules.get(id).map(|r| r.clone()))
            .collect()
    }

    /// 根据工具ID查找规则
    pub fn find_rules_by_tool(&self, tool_id: &str) -> Vec<WatchdogRule> {
        self.rules
            .iter()
            .filter(|r| r.tool_id == tool_id)
            .map(|r| r.clone())
            .collect()
    }

    /// 根据目标Agent查找规则
    pub fn find_rules_by_target_agent(&self, agent_id: &str) -> Vec<WatchdogRule> {
        self.rules
            .iter()
            .filter(|r| r.target_agent_id == agent_id)
            .map(|r| r.clone())
            .collect()
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

    /// 检查规则是否启用
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        self.rules.get(rule_id).map(|r| r.enabled).unwrap_or(false)
    }

    /// 更新规则
    pub fn update_rule(&self, rule: WatchdogRule) -> bool {
        if self.rules.contains_key(&rule.id) {
            // 先从旧标签索引中移除
            if let Some(old_rule) = self.rules.get(&rule.id) {
                for tag in &old_rule.tags {
                    if let Some(mut tag_rules) = self.tag_index.get_mut(tag) {
                        tag_rules.retain(|id| id != &rule.id);
                    }
                }
            }

            // 更新规则
            self.rules.insert(rule.id.clone(), rule.clone());

            // 添加到新标签索引
            for tag in &rule.tags {
                let mut tag_rules = self.tag_index.entry(tag.clone()).or_insert_with(Vec::new);
                if !tag_rules.contains(&rule.id) {
                    tag_rules.push(rule.id.clone());
                }
            }

            info!("Updated rule: {}", rule.id);
            true
        } else {
            false
        }
    }

    /// 获取规则统计信息
    pub fn get_stats(&self) -> RuleStats {
        let total = self.rules.len();
        let enabled = self.rules.iter().filter(|r| r.enabled).count();
        let disabled = total - enabled;

        RuleStats {
            total,
            enabled,
            disabled,
        }
    }

    /// 检查事件是否匹配任何规则
    pub fn check_event(&self, event: &ToolExecutionEvent) -> Vec<String> {
        let mut matched_rules = Vec::new();

        for rule in self.rules.iter() {
            if rule.should_trigger(event) {
                matched_rules.push(rule.id.clone());
            }
        }

        matched_rules
    }

    /// 获取所有标签
    pub fn get_all_tags(&self) -> Vec<String> {
        self.tag_index.iter().map(|entry| entry.key().clone()).collect()
    }
}

/// 规则统计信息
#[derive(Debug)]
pub struct RuleStats {
    pub total: usize,
    pub enabled: usize,
    pub disabled: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::watchdog::TriggerCondition; // 添加正确的导入
    use serde_json::json;

    #[test]
    fn test_rule_management() {
        let manager = RuleManager::new();

        let rule = WatchdogRule::new(
            "test_rule",
            "test_tool",
            TriggerCondition::NumericRange { min: 10.0, max: 20.0 },
            "test_agent",
        ).with_tags(vec!["critical".to_string(), "monitoring".to_string()]);

        // 测试规则注册
        assert!(manager.register_rule(rule).is_ok());

        // 测试规则获取
        assert!(manager.get_rule("test_rule").is_some());

        // 测试按标签查找
        let critical_rules = manager.find_rules_by_tag("critical");
        assert_eq!(critical_rules.len(), 1);
        assert_eq!(critical_rules[0].id, "test_rule");

        // 测试按工具查找
        let tool_rules = manager.find_rules_by_tool("test_tool");
        assert_eq!(tool_rules.len(), 1);

        // 测试启用/禁用
        assert!(manager.set_rule_enabled("test_rule", false));
        assert!(!manager.is_rule_enabled("test_rule"));

        // 测试规则移除
        assert!(manager.remove_rule("test_rule").is_some());
        assert!(manager.get_rule("test_rule").is_none());
    }

    #[test]
    fn test_event_checking() {
        let manager = RuleManager::new();

        let rule = WatchdogRule::new(
            "numeric_rule",
            "test_tool",
            TriggerCondition::NumericRange { min: 10.0, max: 20.0 },
            "test_agent",
        );

        manager.register_rule(rule).unwrap();

        // 测试事件匹配
        let matched = manager.check_event(&ToolExecutionEvent::PostExecute {
            tool_id: "test_tool".to_string(),
            result: json!(15.0),
            context: crate::domain::tool::ToolCallContext::new("test_caller".to_string()),
        });

        assert_eq!(matched, vec!["numeric_rule"]);

        // 测试不匹配的事件
        let unmatched = manager.check_event(&ToolExecutionEvent::PostExecute {
            tool_id: "test_tool".to_string(),
            result: json!(25.0),
            context: crate::domain::tool::ToolCallContext::new("test_caller".to_string()),
        });

        assert_eq!(unmatched.len(), 0);
    }
}