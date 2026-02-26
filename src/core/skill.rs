//! Skill 管理器
//!
//! 提供技能的注册、绑定和权限管理功能

use crate::domain::{Skill, Tool, SkillToolBinding, ToolAccessType};
use crate::core::tool::ToolRegistry;
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct SkillManager {
    /// 技能存储
    skills: DashMap<String, Skill>,
    /// 技能-工具绑定关系
    skill_tool_bindings: DashMap<String, Vec<SkillToolBinding>>, // skill_id -> bindings
    /// 工具-技能绑定关系
    tool_skill_bindings: DashMap<String, Vec<String>>, // tool_id -> skill_ids
    /// 工具访问控制
    tool_access_control: DashMap<String, ToolAccessType>,
    /// 工具注册表引用
    tool_registry: Arc<ToolRegistry>,
}

impl SkillManager {
    pub fn new(tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            skills: DashMap::new(),
            skill_tool_bindings: DashMap::new(),
            tool_skill_bindings: DashMap::new(),
            tool_access_control: DashMap::new(),
            tool_registry,
        }
    }

    /// 注册技能
    pub fn register_skill(&self, skill: Skill) -> Result<()> {
        if self.skills.contains_key(&skill.id) {
            return Err(anyhow::anyhow!("Skill already registered: {}", skill.id));
        }

        self.skills.insert(skill.id.clone(), skill);
        Ok(())
    }

    /// 设置工具访问类型
    pub fn set_tool_access(&self, tool_id: &str, access_type: ToolAccessType) -> Result<()> {
        if !self.tool_registry.contains(tool_id) {
            return Err(anyhow::anyhow!("Tool not found: {}", tool_id));
        }

        self.tool_access_control.insert(tool_id.to_string(), access_type);
        Ok(())
    }

    /// 绑定技能和工具
    pub fn bind_skill_tool(&self, binding: SkillToolBinding) -> Result<()> {
        // 验证技能和工具是否存在
        if !self.skills.contains_key(&binding.skill_id) {
            return Err(anyhow::anyhow!("Skill not found: {}", binding.skill_id));
        }
        if !self.tool_registry.contains(&binding.tool_id) {
            return Err(anyhow::anyhow!("Tool not found: {}", binding.tool_id));
        }

        // 添加到技能-工具映射
        self.skill_tool_bindings
            .entry(binding.skill_id.clone())
            .or_insert_with(Vec::new)
            .push(binding.clone());

        // 添加到工具-技能映射
        self.tool_skill_bindings
            .entry(binding.tool_id.clone())
            .or_insert_with(Vec::new)
            .push(binding.skill_id.clone());

        // 自动将工具设为私有（如果尚未设置）
        if !self.tool_access_control.contains_key(&binding.tool_id) {
            self.tool_access_control
                .insert(binding.tool_id.clone(), ToolAccessType::Private);
        }

        Ok(())
    }

    /// 获取技能可用的工具列表
    pub fn get_skill_tools(&self, skill_id: &str) -> Vec<Tool> {
        let mut tools = Vec::new();

        if let Some(bindings) = self.skill_tool_bindings.get(skill_id) {
            for binding in bindings.value() {
                if let Some(tool) = self.tool_registry.get(&binding.tool_id) {
                    tools.push(tool);
                }
            }
        }

        tools
    }

    /// 检查工具是否可以被调用（基于技能绑定）
    pub fn can_call_tool(&self, tool_id: &str, caller_skills: &[String]) -> bool {
        // 检查工具是否存在
        if !self.tool_registry.contains(tool_id) {
            return false;
        }

        // 检查访问控制
        let access_type = self.tool_access_control
            .get(tool_id)
            .map(|v| v.value().clone())
            .unwrap_or(ToolAccessType::Public); // 默认为公共工具

        match access_type {
            ToolAccessType::Public => true,
            ToolAccessType::Private => {
                // 检查调用者是否具有绑定该工具的技能
                if let Some(allowed_skills) = self.tool_skill_bindings.get(tool_id) {
                    caller_skills.iter().any(|skill| allowed_skills.value().contains(skill))
                } else {
                    false
                }
            }
        }
    }

    /// 获取技能列表
    pub fn get_skills(&self) -> Vec<Skill> {
        self.skills.iter().map(|kv| kv.value().clone()).collect()
    }

    /// 获取技能
    pub fn get_skill(&self, skill_id: &str) -> Option<Skill> {
        self.skills.get(skill_id).map(|s| s.clone())
    }

    /// 获取工具的绑定技能
    pub fn get_tool_bound_skills(&self, tool_id: &str) -> Vec<String> {
        self.tool_skill_bindings
            .get(tool_id)
            .map(|skills| skills.value().clone())
            .unwrap_or_default()
    }

    /// 获取技能的绑定工具
    pub fn get_skill_bound_tools(&self, skill_id: &str) -> Vec<String> {
        self.skill_tool_bindings
            .get(skill_id)
            .map(|bindings| {
                bindings.value()
                    .iter()
                    .map(|binding| binding.tool_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取所有技能ID
    pub fn get_skill_ids(&self) -> Vec<String> {
        self.skills.iter().map(|kv| kv.key().clone()).collect()
    }

    /// 获取所有工具ID
    pub fn get_tool_ids(&self) -> Vec<String> {
        self.tool_registry.list_all().iter().map(|t| t.id.clone()).collect()
    }
}