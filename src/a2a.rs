//! A2A (Agent-to-Agent) 协议简化实现
//!
//! 基于 Google A2A 协议的简化版本，提供 Agent 间的通信能力
//! 核心功能：
//! - Agent Card 描述
//! - 消息传递
//! - 任务管理
//! - 流式响应支持

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Agent 能力描述 (Agent Card)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub endpoints: AgentEndpoints,
    pub skills: Vec<Skill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEndpoints {
    pub a2a_endpoint: String,
    pub webhook_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A2A 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub id: String,
    pub sender: String,
    pub receiver: String,
    pub content: MessageContent,
    pub timestamp: i64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "task")]
    Task { task: TaskRequest },
    #[serde(rename = "task_response")]
    TaskResponse { task_id: String, result: TaskResult },
    #[serde(rename = "status")]
    Status { status: String, message: Option<String> },
}

/// 任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub id: String,
    pub description: String,
    pub parameters: Option<serde_json::Value>,
    pub context: Vec<String>,
}

/// 任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    Success { output: String, artifacts: Vec<Artifact> },
    Error { code: String, message: String },
    Pending,
}

/// 任务产出物
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub content_type: String,
    pub content: String,
}

/// A2A Agent 运行时
pub struct A2AAgent {
    card: AgentCard,
    peers: Arc<RwLock<HashMap<String, AgentCard>>>,
    message_tx: mpsc::Sender<A2AMessage>,
    #[allow(dead_code)]
    message_rx: Arc<RwLock<mpsc::Receiver<A2AMessage>>>,
    #[allow(dead_code)]
    handlers: Arc<RwLock<HashMap<String, Box<dyn MessageHandler>>>>,
}

/// 消息处理 trait
#[async_trait::async_trait]
pub trait MessageHandler: Send + Sync {
    #[allow(dead_code)]
    async fn handle(&self, message: &A2AMessage) -> Result<Option<A2AMessage>>;
}

impl A2AAgent {
    /// 创建新的 A2A Agent
    #[allow(dead_code)]
    pub fn new(card: AgentCard) -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            card,
            peers: Arc::new(RwLock::new(HashMap::new())),
            message_tx: tx,
            message_rx: Arc::new(RwLock::new(rx)),
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取 Agent Card
    #[allow(dead_code)]
    pub fn card(&self) -> &AgentCard {
        &self.card
    }

    /// 注册 Peer Agent
    pub async fn register_peer(&self, peer: AgentCard) {
        let peer_name = peer.name.clone();
        let peer_id = peer.id.clone();
        let mut peers = self.peers.write().await;
        peers.insert(peer_id.clone(), peer);
        info!("Registered peer agent: {} ({})", peer_name, peer_id);
    }

    /// 获取 Peer Agent
    #[allow(dead_code)]
    pub async fn get_peer(&self, peer_id: &str) -> Option<AgentCard> {
        let peers = self.peers.read().await;
        peers.get(peer_id).cloned()
    }

    /// 列出所有 Peers
    #[allow(dead_code)]
    pub async fn list_peers(&self) -> Vec<AgentCard> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// 发送消息给指定 Agent
    pub async fn send_message(&self, receiver_id: &str, content: MessageContent) -> Result<()> {
        let peers = self.peers.read().await;
        let receiver = peers
            .get(receiver_id)
            .context("Peer agent not found")?;

        let message = A2AMessage {
            id: Uuid::new_v4().to_string(),
            sender: self.card.id.clone(),
            receiver: receiver_id.to_string(),
            content,
            timestamp: chrono::Utc::now().timestamp(),
            metadata: HashMap::new(),
        };

        // 发送消息到本地通道（模拟网络传输）
        let msg_id = message.id.clone();
        self.message_tx
            .send(message)
            .await
            .context("Failed to send message")?;

        debug!(
            "Sent message {} to {} at {}",
            msg_id, receiver.name, receiver.endpoints.a2a_endpoint
        );

        Ok(())
    }

    /// 广播消息给所有 Peers
    #[allow(dead_code)]
    pub async fn broadcast(&self, content: MessageContent) -> Result<usize> {
        let peers = self.peers.read().await;
        let peer_ids: Vec<String> = peers.keys().cloned().collect();
        drop(peers);

        let mut sent = 0;
        for peer_id in peer_ids {
            if let Err(e) = self.send_message(&peer_id, content.clone()).await {
                warn!("Failed to send message to {}: {}", peer_id, e);
            } else {
                sent += 1;
            }
        }

        Ok(sent)
    }

    /// 注册消息处理器
    #[allow(dead_code)]
    pub async fn register_handler<H>(&self, message_type: &str, handler: H)
    where
        H: MessageHandler + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.insert(message_type.to_string(), Box::new(handler));
    }

    /// 处理消息循环
    #[allow(dead_code)]
    pub async fn run(&self) -> Result<()> {
        let mut rx = self.message_rx.write().await;
        
        info!("A2A Agent {} started", self.card.name);

        while let Some(message) = rx.recv().await {
            debug!("Received message: {} from {}", message.id, message.sender);

            // 获取消息类型
            let message_type = match &message.content {
                MessageContent::Text { .. } => "text",
                MessageContent::Task { .. } => "task",
                MessageContent::TaskResponse { .. } => "task_response",
                MessageContent::Status { .. } => "status",
            };

            // 查找并执行处理器
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(message_type) {
                match handler.handle(&message).await {
                    Ok(Some(response)) => {
                        if let Err(e) = self.message_tx.send(response).await {
                            error!("Failed to send response: {}", e);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        error!("Handler error for message {}: {}", message.id, e);
                    }
                }
            } else {
                warn!("No handler registered for message type: {}", message_type);
            }
        }

        Ok(())
    }

    /// 获取消息发送器（用于外部注入消息）
    #[allow(dead_code)]
    pub fn message_sender(&self) -> mpsc::Sender<A2AMessage> {
        self.message_tx.clone()
    }
}

/// 简单的文本消息处理器
#[allow(dead_code)]
pub struct TextMessageHandler {
    callback: Box<dyn Fn(&A2AMessage) + Send + Sync>,
}

impl TextMessageHandler {
    #[allow(dead_code)]
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(&A2AMessage) + Send + Sync + 'static,
    {
        Self {
            callback: Box::new(callback),
        }
    }
}

#[async_trait::async_trait]
impl MessageHandler for TextMessageHandler {
    async fn handle(&self, message: &A2AMessage) -> Result<Option<A2AMessage>> {
        (self.callback)(message);
        Ok(None)
    }
}

/// A2A 协议客户端（用于 HTTP 通信）
#[allow(dead_code)]
pub struct A2AClient {
    http: reqwest::Client,
    agent_card: AgentCard,
}

impl A2AClient {
    #[allow(dead_code)]
    pub fn new(agent_card: AgentCard) -> Self {
        Self {
            http: reqwest::Client::new(),
            agent_card,
        }
    }

    /// 发送任务请求
    #[allow(dead_code)]
    pub async fn send_task(
        &self,
        target_endpoint: &str,
        description: &str,
        parameters: Option<serde_json::Value>,
    ) -> Result<TaskResult> {
        let task = TaskRequest {
            id: Uuid::new_v4().to_string(),
            description: description.to_string(),
            parameters,
            context: vec![],
        };

        let _message = A2AMessage {
            id: Uuid::new_v4().to_string(),
            sender: self.agent_card.id.clone(),
            receiver: target_endpoint.to_string(),
            content: MessageContent::Task { task },
            timestamp: chrono::Utc::now().timestamp(),
            metadata: HashMap::new(),
        };

        // 这里简化实现，实际应该通过 HTTP 发送
        info!("Sending task to {}: {}", target_endpoint, description);

        // 模拟异步响应
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(TaskResult::Success {
            output: format!("Task '{}' completed", description),
            artifacts: vec![],
        })
    }

    /// 获取远程 Agent Card
    #[allow(dead_code)]
    pub async fn discover_agent(&self, endpoint: &str) -> Result<AgentCard> {
        let url = format!("{}/.well-known/agent.json", endpoint);
        let response = self.http.get(&url).send().await?;
        let card: AgentCard = response.json().await?;
        Ok(card)
    }
}

/// 创建默认的 Agent Card
pub fn create_default_agent_card(agent_id: &str, name: &str) -> AgentCard {
    AgentCard {
        id: agent_id.to_string(),
        name: name.to_string(),
        description: format!("A2A Agent: {}", name),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: vec!["chat".to_string(), "task_execution".to_string()],
        endpoints: AgentEndpoints {
            a2a_endpoint: format!("/a2a/agents/{}", agent_id),
            webhook_endpoint: None,
        },
        skills: vec![Skill {
            name: "conversation".to_string(),
            description: "General conversation and task execution".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                }
            }),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_card_creation() {
        let card = create_default_agent_card("agent-1", "Test Agent");
        assert_eq!(card.id, "agent-1");
        assert_eq!(card.name, "Test Agent");
        assert!(!card.capabilities.is_empty());
    }

    #[tokio::test]
    async fn test_agent_register_peer() {
        let card = create_default_agent_card("agent-1", "Agent 1");
        let agent = A2AAgent::new(card);

        let peer_card = create_default_agent_card("agent-2", "Agent 2");
        agent.register_peer(peer_card.clone()).await;

        let peers = agent.list_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].id, "agent-2");
    }

    #[tokio::test]
    async fn test_message_creation() {
        let msg = A2AMessage {
            id: "msg-1".to_string(),
            sender: "agent-a".to_string(),
            receiver: "agent-b".to_string(),
            content: MessageContent::Text {
                text: "Hello".to_string(),
            },
            timestamp: chrono::Utc::now().timestamp(),
            metadata: HashMap::new(),
        };

        match &msg.content {
            MessageContent::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_task_result_serialization() {
        let result = TaskResult::Success {
            output: "Test output".to_string(),
            artifacts: vec![Artifact {
                name: "test.txt".to_string(),
                content_type: "text/plain".to_string(),
                content: "Hello World".to_string(),
            }],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test output"));
    }
}
