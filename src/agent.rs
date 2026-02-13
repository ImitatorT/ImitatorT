//! 基于 swarms-rs 的 Agent 实现
//!
//! 提供通用的 Agent 能力，Agent 可以自主决定：
//! - 创建群聊
//! - 发起私聊
//! - 发送广播
//!
//! 所有行为由 Agent 自主决策，框架只提供能力

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use swarms_rs::{agent::SwarmsAgent, llm::provider::openai::OpenAI, structs::agent::Agent as AgentTrait};
use tracing::{debug, info};

use crate::messaging::{AgentMessageReceiver, Message, MessageBus};
use crate::tool::ToolRegistry;

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub model: String,
    pub api_key: String,
    pub base_url: String,
    /// 可选：Agent 元数据（用于扩展属性，如角色、部门等）
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

/// 基于 swarms-rs 的通用 Agent
///
/// Agent 可以自主决定通信行为：
/// - 调用 `create_group()` 创建群聊
/// - 调用 `send_private()` 发起私聊
/// - 调用 `send_broadcast()` 发送广播
/// - 调用 `receive_message()` 接收消息
pub struct Agent {
    config: AgentConfig,
    inner: SwarmsAgent<OpenAI>,
    tool_registry: ToolRegistry,
    /// Agent 的消息接收器（由外部注入）
    message_receiver: Option<AgentMessageReceiver>,
    /// 消息总线引用（用于发送消息）
    message_bus: Option<Arc<MessageBus>>,
}

impl Agent {
    /// 创建新的 Agent
    pub async fn new(config: AgentConfig) -> Result<Self> {
        let client = OpenAI::from_url(&config.base_url, &config.api_key).set_model(&config.model);

        let inner = client
            .agent_builder()
            .agent_name(&config.name)
            .system_prompt(&config.system_prompt)
            .user_name("User")
            .max_loops(1)
            .temperature(0.7)
            .build();

        let tool_registry = ToolRegistry;

        info!("Created agent: {} (id: {})", config.name, config.id);

        Ok(Self {
            config,
            inner,
            tool_registry,
            message_receiver: None,
            message_bus: None,
        })
    }

    /// 获取 Agent ID
    pub fn id(&self) -> &str {
        &self.config.id
    }

    /// 获取 Agent 名称
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// 获取系统提示词
    pub fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    /// 获取元数据
    pub fn metadata(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.config.metadata
    }

    /// 连接消息总线
    ///
    /// 注册 Agent 到消息系统，获得收发消息的能力
    pub fn connect_messaging(&mut self, bus: Arc<MessageBus>) {
        let receiver = bus.register_agent(&self.config.id);
        self.message_receiver = Some(receiver);
        self.message_bus = Some(bus);
        info!("Agent {} connected to messaging bus", self.config.id);
    }

    /// 断开消息总线
    pub fn disconnect_messaging(&mut self) {
        if let Some(ref bus) = self.message_bus {
            bus.unregister_agent(&self.config.id);
        }
        self.message_receiver = None;
        self.message_bus = None;
        info!("Agent {} disconnected from messaging bus", self.config.id);
    }

    /// 创建群聊
    ///
    /// Agent 自主决定创建群聊，邀请其他成员
    pub async fn create_group(
        &mut self,
        group_id: &str,
        name: &str,
        members: Vec<String>,
    ) -> Result<String> {
        let bus = self
            .message_bus
            .as_ref()
            .context("Agent not connected to messaging bus")?;

        // 自动包含创建者
        let mut all_members = members;
        if !all_members.contains(&self.config.id) {
            all_members.push(self.config.id.clone());
        }

        let group_id = bus
            .create_group(group_id, name, &self.config.id, all_members)
            .await?;

        // 自动订阅群聊消息
        if let Some(ref mut receiver) = self.message_receiver {
            receiver.join_group(&group_id, bus)?;
        }

        Ok(group_id)
    }

    /// 邀请成员加入群聊
    pub async fn invite_to_group(&self, group_id: &str, invitee: &str) -> Result<()> {
        let bus = self
            .message_bus
            .as_ref()
            .context("Agent not connected to messaging bus")?;

        bus.invite_to_group(group_id, &self.config.id, invitee)
            .await
    }

    /// 退出群聊
    pub async fn leave_group(&mut self, group_id: &str) -> Result<()> {
        let bus = self
            .message_bus
            .as_ref()
            .context("Agent not connected to messaging bus")?;

        bus.leave_group(group_id, &self.config.id).await?;

        // 取消订阅群聊消息
        if let Some(ref mut receiver) = self.message_receiver {
            receiver.leave_group(group_id);
        }

        Ok(())
    }

    /// 发送私聊消息
    pub async fn send_private(&self, to: &str, content: &str) -> Result<()> {
        let bus = self
            .message_bus
            .as_ref()
            .context("Agent not connected to messaging bus")?;

        let msg = Message::private(&self.config.id, to, content);
        bus.send_private(msg).await
    }

    /// 发送群聊消息
    pub async fn send_group(&self, group_id: &str, content: &str) -> Result<()> {
        let bus = self
            .message_bus
            .as_ref()
            .context("Agent not connected to messaging bus")?;

        let msg = Message::group(&self.config.id, group_id, content);
        bus.send_group(msg).await
    }

    /// 发送广播消息（公司全员群）
    pub fn send_broadcast(&self, content: &str) -> Result<usize> {
        let bus = self
            .message_bus
            .as_ref()
            .context("Agent not connected to messaging bus")?;

        let msg = Message::broadcast(&self.config.id, content);
        bus.broadcast(msg)
    }

    /// 接收消息
    ///
    /// 阻塞等待，直到收到消息
    pub async fn receive_message(&mut self) -> Option<Message> {
        if let Some(ref mut receiver) = self.message_receiver {
            receiver.recv().await
        } else {
            None
        }
    }

    /// 尝试接收消息（非阻塞）
    pub fn try_receive_message(&mut self) -> Option<Message> {
        if let Some(ref mut receiver) = self.message_receiver {
            receiver.try_recv()
        } else {
            None
        }
    }

    /// 运行 Agent 处理任务
    ///
    /// 这是 Agent 的"思考"能力，可以结合接收到的消息做决策
    pub async fn run(&self, task: &str) -> Result<String> {
        debug!("Agent {} running task: {}", self.config.name, task);

        let response = AgentTrait::run(&self.inner, task.to_string())
            .await
            .context("Agent execution failed")?;

        Ok(response)
    }

    /// 执行工具调用
    pub async fn execute_tool(&self, tool_name: &str, arguments: &str) -> Result<String> {
        let tool_call = crate::tool::ToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: "function".to_string(),
            function: crate::tool::FunctionCall {
                name: tool_name.to_string(),
                arguments: arguments.to_string(),
            },
        };
        ToolRegistry::execute(&tool_call).await
    }

    /// 获取可用工具列表
    pub fn available_tools(&self) -> Vec<crate::tool::Tool> {
        ToolRegistry::get_tools()
    }

    /// 获取所在的所有群聊
    pub async fn list_my_groups(&self) -> Result<Vec<crate::messaging::GroupInfo>> {
        let bus = self
            .message_bus
            .as_ref()
            .context("Agent not connected to messaging bus")?;

        Ok(bus.list_agent_groups(&self.config.id).await)
    }
}

/// Agent 管理器
pub struct AgentManager {
    agents: dashmap::DashMap<String, Arc<Agent>>,
}

impl AgentManager {
    /// 创建新的 Agent 管理器
    pub fn new() -> Self {
        Self {
            agents: dashmap::DashMap::new(),
        }
    }

    /// 注册 Agent
    pub async fn register(&self, config: AgentConfig) -> Result<Arc<Agent>> {
        let agent = Arc::new(Agent::new(config).await?);
        let id = agent.id().to_string();
        self.agents.insert(id.clone(), agent.clone());
        info!("Registered agent: {}", id);
        Ok(agent)
    }

    /// 获取 Agent
    pub fn get(&self, id: &str) -> Option<Arc<Agent>> {
        self.agents.get(id).map(|a| a.clone())
    }

    /// 列出所有 Agent
    pub fn list(&self) -> Vec<Arc<Agent>> {
        self.agents.iter().map(|a| a.clone()).collect()
    }

    /// 移除 Agent
    pub fn remove(&self, id: &str) -> Option<Arc<Agent>> {
        self.agents.remove(id).map(|(_, a)| a)
    }

    /// 获取 Agent 数量
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_creation() {
        let config = AgentConfig {
            id: "test-001".to_string(),
            name: "Test Agent".to_string(),
            system_prompt: "You are a test agent.".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: "sk-test".to_string(),
            base_url: "http://localhost:8080".to_string(),
            metadata: serde_json::Map::new(),
        };

        assert_eq!(config.id, "test-001");
        assert_eq!(config.name, "Test Agent");
        assert_eq!(config.system_prompt, "You are a test agent.");
    }

    #[test]
    fn test_agent_manager_creation() {
        let manager = AgentManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_agent_manager_default() {
        let manager: AgentManager = Default::default();
        assert!(manager.is_empty());
    }
}
