//! Agent Runtime
//!
//! Responsible for interacting with LLM and executing decisions

use anyhow::Result;
use crate::domain::{Agent, Message, MessageTarget};
use crate::infrastructure::llm::OpenAIClient;
use serde_json;

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

        Ok(Self { agent, llm })
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

    /// Build thinking prompt
    fn build_thinking_prompt(&self, context: &Context) -> String {
        let mut prompt = format!(
            "{}\n\nCurrent situation:\n",
            self.agent.system_prompt()
        );

        // Add unread messages
        if !context.unread_messages.is_empty() {
            prompt.push_str("\nUnread messages:\n");
            for msg in &context.unread_messages {
                prompt.push_str(&format!("- [{}]: {}\n", msg.from, msg.content));
            }
        }

        // Add current task
        if let Some(task) = &context.current_task {
            prompt.push_str(&format!("\nCurrent task: {}\n", task));
        }

        // Add available decision instructions with JSON format
        prompt.push_str(
            "\nDecide your next action. Respond with ONLY a valid JSON object with one of these formats:\n\
            {\n  \"action\": \"send_message\",\n  \"target\": \"agent_id_or_group_id\",\n  \"content\": \"message content\"\n}\n\
            {\n  \"action\": \"create_group\",\n  \"name\": \"group name\",\n  \"members\": [\"member1\", \"member2\"]\n}\n\
            {\n  \"action\": \"execute_task\",\n  \"task\": \"task description\"\n}\n\
            {\n  \"action\": \"wait\"\n}\n\n\
            Response:",
        );

        prompt
    }

    /// Parse LLM response into decision
    fn parse_decision(&self, response: &str) -> Result<Decision> {
        // Extract JSON from response (in case LLM adds extra text)
        let json_str = self.extract_json_from_response(response);

        if let Some(json_str) = json_str {
            let decision: serde_json::Value = serde_json::from_str(&json_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse JSON decision: {}", e))?;

            if let Some(action) = decision.get("action").and_then(|v| v.as_str()) {
                match action {
                    "send_message" => {
                        let target = decision.get("target")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| anyhow::anyhow!("Missing target in send_message action"))?
                            .to_string();

                        let content = decision.get("content")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| anyhow::anyhow!("Missing content in send_message action"))?
                            .to_string();

                        // Determine if target is a group or individual
                        let target = if target.starts_with("group-") || target.starts_with("group_") {
                            MessageTarget::Group(target)
                        } else {
                            MessageTarget::Direct(target)
                        };

                        Ok(Decision::SendMessage { target, content })
                    },
                    "create_group" => {
                        let name = decision.get("name")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| anyhow::anyhow!("Missing name in create_group action"))?
                            .to_string();

                        let members = decision.get("members")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default(); // Default to empty vector if no members specified

                        Ok(Decision::CreateGroup { name, members })
                    },
                    "execute_task" => {
                        let task = decision.get("task")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| anyhow::anyhow!("Missing task in execute_task action"))?
                            .to_string();

                        Ok(Decision::ExecuteTask { task })
                    },
                    "wait" => Ok(Decision::Wait),
                    _ => {
                        tracing::warn!("Unknown action in decision: {}", action);
                        Ok(Decision::Wait)
                    }
                }
            } else {
                tracing::warn!("No action field in decision JSON: {}", json_str);
                Ok(Decision::Wait)
            }
        } else {
            tracing::warn!("Could not extract JSON from response: {}", response);
            Ok(Decision::Wait)
        }
    }

    /// Extract JSON from response (handles cases where LLM adds extra text around JSON)
    fn extract_json_from_response(&self, response: &str) -> Option<String> {
        // Look for JSON object in the response
        let mut brace_count = 0;
        let mut start_idx = None;

        for (i, ch) in response.char_indices() {
            if ch == '{' {
                if brace_count == 0 {
                    start_idx = Some(i);
                }
                brace_count += 1;
            } else if ch == '}' {
                brace_count -= 1;
                if brace_count == 0 && start_idx.is_some() {
                    return Some(response[start_idx.unwrap()..=i].to_string());
                }
            }
        }

        // If we couldn't find a properly closed JSON object, try to parse the whole response
        if serde_json::from_str::<serde_json::Value>(response.trim()).is_ok() {
            Some(response.trim().to_string())
        } else {
            None
        }
    }

    /// Execute task
    pub async fn execute_task(&self, task: &str) -> Result<String> {
        let prompt = format!(
            "{}\n\nComplete the following task:\n{}\n",
            self.agent.system_prompt(),
            task
        );

        self.llm.complete(&prompt).await
    }
}

/// Agent Decision
#[derive(Debug, Clone)]
pub enum Decision {
    /// Send message
    SendMessage {
        target: MessageTarget,
        content: String,
    },
    /// Create group chat
    CreateGroup {
        name: String,
        members: Vec<String>,
    },
    /// Execute task
    ExecuteTask {
        task: String,
    },
    /// Wait
    Wait,
}

/// Agent Context
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