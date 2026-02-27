//! Agent Domain Entity
//!
//! Basic definition of virtual company employees

use serde::{Deserialize, Serialize};

/// Agent Unique Identifier
pub type AgentId = String;

/// Agent Mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMode {
    /// Active mode: Actively monitor specific tools and work autonomously based on results
    Active {
        // List of monitored tool IDs
        watched_tools: Vec<String>,
        // Trigger condition configuration
        trigger_conditions: Vec<TriggerCondition>,
    },
    /// Passive mode: Only work when mentioned or receives messages
    Passive,
}

/// Trigger Condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Numeric range condition: Trigger when tool returns value within specified range
    NumericRange {
        min: f64,
        max: f64,
    },
    /// String matching condition: Trigger when tool returns string containing specified content
    StringContains {
        content: String,
    },
    /// Status matching condition: Trigger when tool returns specific status
    StatusMatches {
        expected_status: String,
    },
    /// Custom expression condition: Define complex conditions using expression language
    CustomExpression {
        expression: String,
    },
}

/// Agent Entity - Unified Agent definition, single source of truth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub role: Role,
    pub department_id: Option<String>,
    pub llm_config: LLMConfig,
    /// Agent mode, defaults to passive mode
    pub mode: AgentMode,
}

impl Agent {
    /// Create a new Agent
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        role: Role,
        llm_config: LLMConfig,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role,
            department_id: None,
            llm_config,
            mode: AgentMode::Passive, // Default to passive mode
        }
    }

    /// Create a new Agent with mode
    pub fn new_with_mode(
        id: impl Into<String>,
        name: impl Into<String>,
        role: Role,
        llm_config: LLMConfig,
        mode: AgentMode,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role,
            department_id: None,
            llm_config,
            mode,
        }
    }

    /// Set department
    pub fn with_department(mut self, dept_id: impl Into<String>) -> Self {
        self.department_id = Some(dept_id.into());
        self
    }

    /// Set mode
    pub fn with_mode(mut self, mode: AgentMode) -> Self {
        self.mode = mode;
        self
    }

    /// Generate system prompt
    pub fn system_prompt(&self) -> String {
        self.role.system_prompt.clone()
    }
}

/// Role Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub title: String,
    pub responsibilities: Vec<String>,
    pub expertise: Vec<String>,
    pub system_prompt: String,
}

impl Role {
    /// Create simple role
    pub fn simple(title: impl Into<String>, system_prompt: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            responsibilities: vec![],
            expertise: vec![],
            system_prompt: system_prompt.into(),
        }
    }

    /// Add responsibilities
    pub fn with_responsibilities(mut self, items: Vec<String>) -> Self {
        self.responsibilities = items;
        self
    }

    /// Add expertise areas
    pub fn with_expertise(mut self, items: Vec<String>) -> Self {
        self.expertise = items;
        self
    }
}

/// LLM Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub model: String,
    pub api_key: String,
    pub base_url: String,
}

impl LLMConfig {
    /// Use OpenAI default configuration
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self {
            model: "gpt-4o-mini".to_string(),
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set base URL (for custom endpoints)
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}
