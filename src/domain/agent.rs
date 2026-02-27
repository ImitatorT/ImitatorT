//! Agent Domain Entity
//!
//! Basic definition of virtual company employees

use serde::{Deserialize, Serialize};

/// Agent Unique Identifier
pub type AgentId = String;


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
    /// List of monitored tool IDs
    pub watched_tools: Vec<String>,
    /// Trigger condition configuration
    pub trigger_conditions: Vec<TriggerCondition>,
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
            watched_tools: vec![],
            trigger_conditions: vec![],
        }
    }

    /// Create a new Agent with tool watching capability
    pub fn new_with_watching(
        id: impl Into<String>,
        name: impl Into<String>,
        role: Role,
        llm_config: LLMConfig,
        watched_tools: Vec<String>,
        trigger_conditions: Vec<TriggerCondition>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role,
            department_id: None,
            llm_config,
            watched_tools,
            trigger_conditions,
        }
    }

    /// Set department
    pub fn with_department(mut self, dept_id: impl Into<String>) -> Self {
        self.department_id = Some(dept_id.into());
        self
    }

    /// Set watched tools
    pub fn with_watched_tools(mut self, watched_tools: Vec<String>) -> Self {
        self.watched_tools = watched_tools;
        self
    }

    /// Set trigger conditions
    pub fn with_trigger_conditions(mut self, trigger_conditions: Vec<TriggerCondition>) -> Self {
        self.trigger_conditions = trigger_conditions;
        self
    }

    /// Add a tool to watch
    pub fn add_watched_tool(mut self, tool_id: impl Into<String>) -> Self {
        self.watched_tools.push(tool_id.into());
        self
    }

    /// Add a trigger condition
    pub fn add_trigger_condition(mut self, condition: TriggerCondition) -> Self {
        self.trigger_conditions.push(condition);
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
