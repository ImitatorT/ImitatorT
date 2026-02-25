//! Agent 运行时
//!
//! 负责与LLM交互和执行决策

use anyhow::Result;

use crate::domain::{Agent, Message, MessageTarget};
use crate::infrastructure::llm::OpenAIClient;

/// Agent 运行时 - 负责思考和执行
pub struct AgentRuntime {
    agent: Agent,
    llm: OpenAIClient,
}

impl AgentRuntime {
    /// 创建新的Agent运行时
    pub async fn new(agent: Agent) -> Result<Self> {
        let llm = OpenAIClient::new_with_base_url(
            agent.llm_config.api_key.clone(),
            agent.llm_config.model.clone(),
            agent.llm_config.base_url.clone(),
        );

        Ok(Self { agent, llm })
    }

    /// 获取Agent ID
    pub fn id(&self) -> &str {
        &self.agent.id
    }

    /// 获取Agent名称
    pub fn name(&self) -> &str {
        &self.agent.name
    }

    /// 获取Agent引用
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    /// 思考并做出决策
    pub async fn think(&self, context: Context) -> Result<Decision> {
        // 构建提示词
        let prompt = self.build_thinking_prompt(&context);

        // 调用LLM
        let response = self.llm.complete(&prompt).await?;

        // 解析决策
        let decision = self.parse_decision(&response)?;

        Ok(decision)
    }

    /// 构建思考提示词
    fn build_thinking_prompt(&self, context: &Context) -> String {
        let mut prompt = format!(
            "{}

当前情况：
",
            self.agent.system_prompt()
        );

        // 添加未读消息
        if !context.unread_messages.is_empty() {
            prompt.push_str("\n未读消息：\n");
            for msg in &context.unread_messages {
                prompt.push_str(&format!("- [{}]: {}\n", msg.from, msg.content));
            }
        }

        // 添加当前任务
        if let Some(task) = &context.current_task {
            prompt.push_str(&format!("\n当前任务：{}\n", task));
        }

        // 添加可用决策说明
        prompt.push_str(
            "\n请决定你的下一步行动。你可以：\n\
            1. SEND_MESSAGE <目标> <内容> - 发送消息（目标是agent_id或group_id）\n\
            2. CREATE_GROUP <群名称> <成员1,成员2,...> - 创建群聊\n\
            3. EXECUTE_TASK <任务描述> - 执行任务\n\
            4. WAIT - 暂时等待\n\
            \n请输出你的决策（一行）：\n",
        );

        prompt
    }

    /// 解析LLM响应为决策
    fn parse_decision(&self, response: &str) -> Result<Decision> {
        let line = response.lines().next().unwrap_or("WAIT").trim();

        if line.starts_with("SEND_MESSAGE ") {
            let parts: Vec<&str> = line[13..].splitn(2, ' ').collect();
            if parts.len() == 2 {
                let target = parts[0].to_string();
                let content = parts[1].to_string();

                // 判断目标是群组还是个人
                let target = if target.starts_with("group-") {
                    MessageTarget::Group(target)
                } else {
                    MessageTarget::Direct(target)
                };

                return Ok(Decision::SendMessage { target, content });
            }
        } else if line.starts_with("CREATE_GROUP ") {
            let parts: Vec<&str> = line[13..].splitn(2, ' ').collect();
            if parts.len() == 2 {
                let name = parts[0].to_string();
                let members: Vec<String> = parts[1].split(',').map(|s| s.trim().to_string()).collect();
                return Ok(Decision::CreateGroup { name, members });
            }
        } else if line.starts_with("EXECUTE_TASK ") {
            let task = line[13..].to_string();
            return Ok(Decision::ExecuteTask { task });
        }

        Ok(Decision::Wait)
    }

    /// 执行任务
    pub async fn execute_task(&self, task: &str) -> Result<String> {
        let prompt = format!(
            "{}

请完成以下任务：
{}\n",
            self.agent.system_prompt(),
            task
        );

        self.llm.complete(&prompt).await
    }
}

/// Agent 决策
#[derive(Debug, Clone)]
pub enum Decision {
    /// 发送消息
    SendMessage {
        target: MessageTarget,
        content: String,
    },
    /// 创建群聊
    CreateGroup {
        name: String,
        members: Vec<String>,
    },
    /// 执行任务
    ExecuteTask {
        task: String,
    },
    /// 等待
    Wait,
}

/// Agent 上下文
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// 未读消息
    pub unread_messages: Vec<Message>,
    /// 当前任务
    pub current_task: Option<String>,
    /// 组织架构信息
    pub organization_info: Option<String>,
}

impl Context {
    /// 添加上下文
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.unread_messages = messages;
        self
    }

    /// 添加任务
    pub fn with_task(mut self, task: impl Into<String>) -> Self {
        self.current_task = Some(task.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{LLMConfig, Role};

    #[test]
    fn test_decision_parsing() {
        let role = Role::simple("测试", "你是测试");
        let llm = LLMConfig::openai("test");
        let agent = Agent::new("a1", "测试员", role, llm);

        let runtime = AgentRuntime {
            agent,
            llm: OpenAIClient::new_with_base_url("test".to_string(), "gpt-4".to_string(), "http://localhost".to_string()),
        };

        // 测试发送消息决策
        let decision = runtime
            .parse_decision("SEND_MESSAGE agent-2 你好！")
            .unwrap();
        match decision {
            Decision::SendMessage { target, content } => {
                match target {
                    MessageTarget::Direct(id) => assert_eq!(id, "agent-2"),
                    _ => panic!("Expected Direct target"),
                }
                assert_eq!(content, "你好！");
            }
            _ => panic!("Expected SendMessage decision"),
        }

        // 测试群聊决策
        let decision = runtime
            .parse_decision("CREATE_GROUP 测试群 agent-1,agent-2")
            .unwrap();
        match decision {
            Decision::CreateGroup { name, members } => {
                assert_eq!(name, "测试群");
                assert_eq!(members, vec!["agent-1", "agent-2"]);
            }
            _ => panic!("Expected CreateGroup decision"),
        }

        // 测试等待决策
        let decision = runtime.parse_decision("WAIT").unwrap();
        assert!(matches!(decision, Decision::Wait));
    }
}
