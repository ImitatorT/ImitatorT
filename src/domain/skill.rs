//! Skill 领域实体
//!
//! 技能系统的核心业务定义，支持技能与工具的多对多绑定

use serde::{Deserialize, Serialize};

/// 技能实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub author: String,
    pub metadata: std::collections::HashMap<String, serde_json::Value>, // 扩展元数据
}

impl Skill {
    /// 创建新技能
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: impl Into<String>,
        version: impl Into<String>,
        author: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            category: category.into(),
            version: version.into(),
            author: author.into(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// 设置元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// 技能与工具的关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillToolBinding {
    pub skill_id: String,
    pub tool_id: String,
    pub binding_type: BindingType,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl SkillToolBinding {
    /// 创建新的绑定
    pub fn new(
        skill_id: impl Into<String>,
        tool_id: impl Into<String>,
        binding_type: BindingType,
    ) -> Self {
        Self {
            skill_id: skill_id.into(),
            tool_id: tool_id.into(),
            binding_type,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// 设置绑定元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// 绑定类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BindingType {
    /// 强绑定：技能必须有此工具才能正常工作
    Required,
    /// 可选绑定：技能可以利用此工具增强功能
    Optional,
}

/// 工具访问类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolAccessType {
    /// 公共工具：任何人都可以调用
    Public,
    /// 私有工具：需要特定技能才能调用
    Private,
}