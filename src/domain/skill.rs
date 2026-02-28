//! Skill Domain Entity
//!
//! Core business definition for skill system, supporting many-to-many binding between skills and tools

use serde::{Deserialize, Serialize};

/// Skill Entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub author: String,
    pub metadata: std::collections::HashMap<String, serde_json::Value>, // Extended metadata
}

impl Skill {
    /// Create new skill
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

    /// Set metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Relationship between Skill and Tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillToolBinding {
    pub skill_id: String,
    pub tool_id: String,
    pub binding_type: BindingType,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl SkillToolBinding {
    /// Create new binding
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

    /// Set binding metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Binding Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BindingType {
    /// Required binding: Skill must have this tool to work properly
    Required,
    /// Optional binding: Skill can enhance functionality using this tool
    Optional,
}

/// Tool Access Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolAccessType {
    /// Public tool: Anyone can call
    Public,
    /// Private tool: Requires specific skill to call
    Private,
}
