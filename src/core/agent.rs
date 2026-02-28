//! Agent Runtime
//!
//! Responsible for interacting with LLM and executing ReAct decisions

use crate::domain::{Agent, Message, MessageTarget};
use crate::infrastructure::llm::{OpenAIClient, Tool, ToolCall, ToolResponse};
use anyhow::Result;

/// Agent Runtime - Responsible for ReAct thinking and execution
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

    /// Execute ReAct loop with multiple reasoning and action steps
    pub async fn react_think(&self, context: Context, tools: Vec<Tool>) -> Result<Decision> {
        // Start with initial context
        let mut current_context = context;

        // Perform ReAct loop up to max_iterations times
        let max_iterations = 5; // Prevent infinite loops
        for _iteration in 0..max_iterations {
            // Step 1: Reason about the current state
            let reasoning_result = self.reason_step(&current_context, &tools).await?;

            match reasoning_result {
                ReActStepResult::Action {
                    tool_calls,
                    observation,
                } => {
                    // Step 2: Execute actions and observe results
                    let observations = self.execute_actions(tool_calls, &current_context).await?;

                    // Step 3: Update context with observations for next iteration
                    let obs_text = format!("{}\n{}", observation, observations.join("\n"));
                    current_context = current_context.with_observation(obs_text);
                }
                ReActStepResult::FinalDecision(decision) => {
                    // Return final decision when no more actions needed
                    return Ok(decision);
                }
            }
        }

        // If we reach max iterations, return the final thought as a task execution
        Ok(Decision::ExecuteTask {
            task: "Final response after multiple reasoning steps".to_string(),
        })
    }

    /// Single reasoning step in the ReAct loop
    async fn reason_step(&self, context: &Context, tools: &[Tool]) -> Result<ReActStepResult> {
        let prompt = self.build_react_prompt(context, tools);

        // Use chat_with_tools to get both reasoning and potential tool calls
        let response = self.llm.chat_with_tools(
            vec![
                crate::infrastructure::llm::Message::system(prompt),
                crate::infrastructure::llm::Message::user("Think step by step and decide what actions to take. If you need to use tools, specify them. Otherwise, provide your final response.")
            ],
            tools.to_vec()
        ).await?;

        match response {
            ToolResponse::Message(content) => {
                // If the response is just text, treat it as a final decision
                Ok(ReActStepResult::FinalDecision(Decision::ExecuteTask {
                    task: content,
                }))
            }
            ToolResponse::ToolCalls {
                content,
                tool_calls,
            } => {
                // If there are tool calls, execute them
                Ok(ReActStepResult::Action {
                    tool_calls,
                    observation: content,
                })
            }
        }
    }

    /// Execute multiple actions and return observations
    async fn execute_actions(
        &self,
        tool_calls: Vec<ToolCall>,
        _context: &Context,
    ) -> Result<Vec<String>> {
        let mut observations = Vec::new();

        // Execute each tool call sequentially (could be parallelized depending on dependencies)
        for tool_call in tool_calls {
            // In a real implementation, this would look up the actual tool function and execute it
            // For now, we'll simulate the execution and return a placeholder result
            let observation = format!(
                "Executed tool '{}' with arguments: {}. Result: Tool executed successfully.",
                tool_call.name, tool_call.arguments
            );
            observations.push(observation);
        }

        Ok(observations)
    }

    /// Build ReAct-style prompt that encourages reasoning and action
    fn build_react_prompt(&self, context: &Context, tools: &[Tool]) -> String {
        let mut prompt = format!(
            "You are {}, {}. ",
            self.agent.name,
            self.agent.system_prompt()
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
                    msg.from, msg.content, target_desc
                ));
            }
        }

        if !context.observations.is_empty() {
            prompt.push_str("\nPrevious observations:\n");
            for obs in &context.observations {
                prompt.push_str(&format!("- {}\n", obs));
            }
        }

        if !tools.is_empty() {
            prompt.push_str("\nYou have access to these tools:\n");
            for tool in tools {
                prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
            }
        }

        prompt.push_str("\nFollow the ReAct (Reasoning and Acting) framework:\n");
        prompt.push_str("1. Think: Analyze the situation and plan your approach\n");
        prompt.push_str("2. Act: Use tools or take actions as needed\n");
        prompt.push_str("3. Observe: See the results of your actions\n");
        prompt.push_str("4. Repeat: Continue until you achieve your goal\n");
        prompt.push_str("\nProvide your response in JSON format with fields: 'action' (one of 'send_message', 'create_group', 'execute_task', 'wait', or use available tools), 'target' (for messages), 'content' (for messages), 'group_name' (for groups), 'members' (for groups), 'task' (for execute_task).");

        prompt
    }

    /// Legacy think method for backward compatibility
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
            self.agent.name,
            self.agent.system_prompt()
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
                    msg.from, msg.content, target_desc
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

/// Result of a single ReAct step
enum ReActStepResult {
    Action {
        tool_calls: Vec<ToolCall>,
        observation: String,
    },
    FinalDecision(Decision),
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
    /// Previous observations from tool executions
    pub observations: Vec<String>,
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

    /// Add observation
    pub fn with_observation(mut self, observation: impl Into<String>) -> Self {
        self.observations.push(observation.into());
        self
    }

    /// Add multiple observations
    pub fn with_observations(mut self, observations: Vec<String>) -> Self {
        self.observations.extend(observations);
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
