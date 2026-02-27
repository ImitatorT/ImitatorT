//! Agent Runtime
//!
//! Responsible for interacting with LLM and executing decisions

use anyhow::Result;
use crate::domain::{Agent, Message, MessageTarget};
use crate::infrastructure::llm::OpenAIClient;

/// Agent Runtime - Responsible for thinking and executing
pub struct AgentRuntime {
    agent: Agent,
    llm: OpenAIClient,
}

impl AgentRuntime {
    /// Create a new Agent Runtime
    pub async fn new(agent: Agent) -> Result<Self> {
        let llm = OpenAIClient::new_with_base_url(
            agent.llm_config.api_key.clone(),
            agent.llm_config.model.clone(),
            agent.llm_config.base_url.clone(),
        );

        Ok(Self {
            agent,
            llm,
        })
    }

    /// Get Agent ID
    pub fn id(&self) -> &str {
        &self.agent.id
    }

    /// Get Agent Name
    pub fn name(&self) -> &str {
        &self.agent.name
    }

    /// Get Agent reference
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    /// Think and make decisions
    pub async fn think(&self, context: Context) -> Result<Decision> {
        // Build prompt
        let prompt = self.build_thinking_prompt(&context);

        // Call LLM
        let response = self.llm.complete(&prompt).await?;

        // Parse decision
        let decision = self.parse_decision(&response)?;

        Ok(decision)
    }

    /// Execute a specific task
    pub async fn execute_task(&self, task: &str) -> Result<String> {
        let prompt = format!(
            "You are {}, {}. Your task is: {}\n\nResponse:",
            self.agent.name,
            self.agent.system_prompt(),
            task
        );

        let response = self.llm.complete(&prompt).await?;
        Ok(response.trim().to_string())
    }

    /// Build thinking prompt
    fn build_thinking_prompt(&self, context: &Context) -> String {
        let mut prompt = format!(
            "You are {}, {}. ",
            self.agent.name, self.agent.system_prompt()
        );

        if let Some(org_info) = &context.organization_info {
            prompt.push_str(&format!("Organization info: {}\n", org_info));
        }

        if let Some(task) = &context.current_task {
            prompt.push_str(&format!("Your current task is: {}\n", task));
        }

        if !context.unread_messages.is_empty() {
            prompt.push_str("\nRecent messages:\n");
            for msg in &context.unread_messages {
                let target_desc = match &msg.to {
                    MessageTarget::Direct(id) => format!("Direct to {}", id),
                    MessageTarget::Group(id) => format!("Group {}", id),
                };
                prompt.push_str(&format!(
                    "- From {}: {} (to: {})\n",
                    msg.from,
                    msg.content,
                    target_desc
                ));
            }
        }

        prompt.push_str("\nBased on this information, what is your decision or action? Respond in JSON format with fields: 'action' (one of 'send_message', 'create_group', 'execute_task', 'wait'), 'target' (for messages), 'content' (for messages), 'group_name' (for groups), 'members' (for groups), 'task' (for execute_task).");
        prompt
    }

    /// Parse decision from LLM response
    fn parse_decision(&self, response: &str) -> Result<Decision> {
        // Simplified parsing - in a real implementation, this would be more robust
        // For now, default to ExecuteTask as a general-purpose action
        Ok(Decision::ExecuteTask {
            task: response.to_string(),
        })
    }
}

/// Context for Agent thinking
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Unread messages
    pub unread_messages: Vec<Message>,
    /// Current task
    pub current_task: Option<String>,
    /// Organization information
    pub organization_info: Option<String>,
}

impl Context {
    /// Add context
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.unread_messages = messages;
        self
    }

    /// Add task
    pub fn with_task(mut self, task: impl Into<String>) -> Self {
        self.current_task = Some(task.into());
        self
    }
}

/// Decision made by agent
#[derive(Debug, Clone)]
pub enum Decision {
    SendMessage {
        target: MessageTarget,
        content: String,
    },
    CreateGroup {
        name: String,
        members: Vec<String>,
    },
    ExecuteTask {
        task: String,
    },
    Wait,
    Error(String),
}