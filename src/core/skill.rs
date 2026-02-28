//! Skill 管理器
//!
//! 提供技能的注册、绑定和权限管理功能

use crate::domain::{Skill, Tool, SkillToolBinding, ToolAccessType};
use crate::core::tool::ToolRegistry;
use crate::domain::capability::{Capability, CapabilityAccessType, SkillCapabilityBinding};
use crate::core::capability::CapabilityRegistry;
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
    /// 技能-功能绑定关系
    skill_capability_bindings: DashMap<String, Vec<SkillCapabilityBinding>>, // skill_id -> bindings
    /// 功能-技能绑定关系
    capability_skill_bindings: DashMap<String, Vec<String>>, // capability_id -> skill_ids
    /// 功能访问控制
    capability_access_control: DashMap<String, CapabilityAccessType>,
    /// 工具注册表引用
    tool_registry: Arc<ToolRegistry>,
    /// 功能注册表引用
    capability_registry: Arc<CapabilityRegistry>,
}

impl SkillManager {
    pub fn new(tool_registry: Arc<ToolRegistry>, capability_registry: Arc<CapabilityRegistry>) -> Self {
        Self {
            skills: DashMap::new(),
            skill_tool_bindings: DashMap::new(),
            tool_skill_bindings: DashMap::new(),
            tool_access_control: DashMap::new(),
            skill_capability_bindings: DashMap::new(),
            capability_skill_bindings: DashMap::new(),
            capability_access_control: DashMap::new(),
            tool_registry,
            capability_registry,
        }
    }

    /// 创建仅支持工具的 SkillManager（用于向后兼容）
    pub fn new_with_tool_registry(tool_registry: Arc<ToolRegistry>) -> Self {
        let capability_registry = Arc::new(CapabilityRegistry::new());
        Self::new(tool_registry, capability_registry)
    }

    /// 创建支持工具和功能的 SkillManager
    pub fn new_with_registries(tool_registry: Arc<ToolRegistry>, capability_registry: Arc<CapabilityRegistry>) -> Self {
        Self::new(tool_registry, capability_registry)
    }

    /// 创建仅支持功能的 SkillManager（用于仅能力场景）
    pub fn new_with_capability_registry(capability_registry: Arc<CapabilityRegistry>) -> Self {
        let tool_registry = Arc::new(ToolRegistry::new());
        Self::new(tool_registry, capability_registry)
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

    /// 设置功能访问类型
    pub fn set_capability_access(&self, capability_id: &str, access_type: CapabilityAccessType) -> Result<()> {
        if !self.capability_registry.contains(capability_id) {
            return Err(anyhow::anyhow!("Capability not found: {}", capability_id));
        }

        self.capability_access_control.insert(capability_id.to_string(), access_type);
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
            .or_default()
            .push(binding.clone());

        // 添加到工具-技能映射
        self.tool_skill_bindings
            .entry(binding.tool_id.clone())
            .or_default()
            .push(binding.skill_id.clone());

        // 自动将工具设为私有（如果尚未设置）
        if !self.tool_access_control.contains_key(&binding.tool_id) {
            self.tool_access_control
                .insert(binding.tool_id.clone(), ToolAccessType::Private);
        }

        Ok(())
    }

    /// 绑定技能和功能
    pub fn bind_skill_capability(&self, binding: SkillCapabilityBinding) -> Result<()> {
        // 验证技能和功能是否存在
        if !self.skills.contains_key(&binding.skill_id) {
            return Err(anyhow::anyhow!("Skill not found: {}", binding.skill_id));
        }
        if !self.capability_registry.contains(&binding.capability_id) {
            return Err(anyhow::anyhow!("Capability not found: {}", binding.capability_id));
        }

        // 添加到技能-功能映射
        self.skill_capability_bindings
            .entry(binding.skill_id.clone())
            .or_default()
            .push(binding.clone());

        // 添加到功能-技能映射
        self.capability_skill_bindings
            .entry(binding.capability_id.clone())
            .or_default()
            .push(binding.skill_id.clone());

        // 自动将功能设为私有（如果尚未设置）
        if !self.capability_access_control.contains_key(&binding.capability_id) {
            self.capability_access_control
                .insert(binding.capability_id.clone(), CapabilityAccessType::Private);
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

    /// 获取技能可用的功能列表
    pub fn get_skill_capabilities(&self, skill_id: &str) -> Vec<Capability> {
        let mut capabilities = Vec::new();

        if let Some(bindings) = self.skill_capability_bindings.get(skill_id) {
            for binding in bindings.value() {
                if let Some(capability) = self.capability_registry.get(&binding.capability_id) {
                    capabilities.push(capability);
                }
            }
        }

        capabilities
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

    /// 检查功能是否可以被调用（基于技能绑定）
    pub fn can_call_capability(&self, capability_id: &str, caller_skills: &[String]) -> bool {
        // 检查功能是否存在
        if !self.capability_registry.contains(capability_id) {
            return false;
        }

        // 检查访问控制
        let access_type = self.capability_access_control
            .get(capability_id)
            .map(|v| v.value().clone())
            .unwrap_or(CapabilityAccessType::Public); // 默认为公共功能

        match access_type {
            CapabilityAccessType::Public => true,
            CapabilityAccessType::Private => {
                // 检查调用者是否具有绑定该功能的技能
                if let Some(allowed_skills) = self.capability_skill_bindings.get(capability_id) {
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

    /// 获取功能的绑定技能
    pub fn get_capability_bound_skills(&self, capability_id: &str) -> Vec<String> {
        self.capability_skill_bindings
            .get(capability_id)
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

    /// 获取技能的绑定功能
    pub fn get_skill_bound_capabilities(&self, skill_id: &str) -> Vec<String> {
        self.skill_capability_bindings
            .get(skill_id)
            .map(|bindings| {
                bindings.value()
                    .iter()
                    .map(|binding| binding.capability_id.clone())
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

    /// 获取所有功能ID
    pub fn get_capability_ids(&self) -> Vec<String> {
        self.capability_registry.list_all().iter().map(|c| c.id.clone()).collect()
    }
}