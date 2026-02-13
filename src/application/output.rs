//! 输出抽象层
//!
//! 提供统一的输出接口，支持多种输出目标：
//! - Matrix: 作为前端展示
//! - CLI: 命令行输出
//! - A2A: Agent 间通信

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::core::store::{ChatMessage, MessageStore, MessageType};
use crate::infrastructure::matrix::MatrixClient;
use crate::protocol::types::{A2AAgent, A2AMessage, MessageContent};

/// 输出模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputMode {
    /// Matrix 前端
    Matrix,
    /// 命令行输出
    Cli,
    /// A2A 协议
    A2A,
}

impl std::str::FromStr for OutputMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "matrix" => Ok(OutputMode::Matrix),
            "cli" => Ok(OutputMode::Cli),
            "a2a" => Ok(OutputMode::A2A),
            _ => Err(format!("Unknown output mode: {}", s)),
        }
    }
}

impl std::fmt::Display for OutputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputMode::Matrix => write!(f, "matrix"),
            OutputMode::Cli => write!(f, "cli"),
            OutputMode::A2A => write!(f, "a2a"),
        }
    }
}

/// 输出 trait
#[async_trait]
pub trait Output: Send + Sync {
    /// 发送消息
    async fn send_message(&self, sender: &str, content: &str) -> Result<()>;

    /// 获取上下文（最近的消息）
    async fn get_context(&self, limit: usize) -> Result<String>;

    /// 获取输出模式
    fn mode(&self) -> OutputMode;
}

/// Matrix 输出实现
pub struct MatrixOutput {
    client: MatrixClient,
    room_id: String,
    store: MessageStore,
}

impl MatrixOutput {
    pub fn new(client: MatrixClient, room_id: String, store: MessageStore) -> Self {
        Self {
            client,
            room_id,
            store,
        }
    }
}

#[async_trait]
impl Output for MatrixOutput {
    async fn send_message(&self, sender: &str, content: &str) -> Result<()> {
        // 发送到 Matrix
        self.client
            .send_text_message(&self.room_id, content)
            .await
            .context("Failed to send message to Matrix")?;

        // 保存到存储
        let msg = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: sender.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            message_type: MessageType::Text,
        };
        self.store.add_message(msg).await?;

        info!("Sent message to Matrix room {}", self.room_id);
        Ok(())
    }

    async fn get_context(&self, limit: usize) -> Result<String> {
        // 先从 Matrix 拉取最新上下文
        match self.client.latest_context(&self.room_id, limit).await {
            Ok(context) => Ok(context),
            Err(e) => {
                warn!(
                    "Failed to get context from Matrix: {}, falling back to store",
                    e
                );
                Ok(self.store.get_context_string(limit).await)
            }
        }
    }

    fn mode(&self) -> OutputMode {
        OutputMode::Matrix
    }
}

/// CLI 输出实现
pub struct CliOutput {
    store: MessageStore,
    echo: bool, // 是否回显发送的消息
}

impl CliOutput {
    pub fn new(store: MessageStore, echo: bool) -> Self {
        Self { store, echo }
    }
}

#[async_trait]
impl Output for CliOutput {
    async fn send_message(&self, sender: &str, content: &str) -> Result<()> {
        // 保存到存储
        let msg = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: sender.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            message_type: MessageType::Text,
        };
        self.store.add_message(msg).await?;

        // 打印到命令行
        if self.echo {
            println!("\n[{}] {}", sender, content);
        }

        debug!("Message output to CLI from {}", sender);
        Ok(())
    }

    async fn get_context(&self, limit: usize) -> Result<String> {
        let context = self.store.get_context_string(limit).await;
        if context.is_empty() {
            Ok("(No previous context)".to_string())
        } else {
            Ok(context)
        }
    }

    fn mode(&self) -> OutputMode {
        OutputMode::Cli
    }
}

/// A2A 输出实现
pub struct A2AOutput {
    agent: Arc<A2AAgent>,
    store: MessageStore,
    default_receiver: Option<String>,
}

impl A2AOutput {
    pub fn new(
        agent: Arc<A2AAgent>,
        store: MessageStore,
        default_receiver: Option<String>,
    ) -> Self {
        Self {
            agent,
            store,
            default_receiver,
        }
    }

    /// 发送消息给指定 Agent
    #[allow(dead_code)]
    pub async fn send_to(&self, receiver_id: &str, content: &str) -> Result<()> {
        let content = MessageContent::Text {
            text: content.to_string(),
        };

        let result: anyhow::Result<()> = self.agent.send_message(receiver_id, content).await;
        result.map_err(|e| anyhow::anyhow!("Failed to send A2A message: {}", e))?;

        info!("Sent A2A message to {}", receiver_id);
        Ok(())
    }

    /// 广播消息给所有 Peers
    #[allow(dead_code)]
    pub async fn broadcast(&self, content: &str) -> Result<usize> {
        let content = MessageContent::Text {
            text: content.to_string(),
        };

        let sent = self.agent.broadcast(content).await?;
        info!("Broadcasted A2A message to {} peers", sent);
        Ok(sent)
    }
}

#[async_trait]
impl Output for A2AOutput {
    async fn send_message(&self, sender: &str, content: &str) -> Result<()> {
        // 保存到存储
        let msg = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: sender.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            message_type: MessageType::Text,
        };
        self.store.add_message(msg).await?;

        // 如果有默认接收者，通过 A2A 发送
        if let Some(ref receiver) = self.default_receiver {
            let a2a_content = MessageContent::Text {
                text: content.to_string(),
            };
            let result: anyhow::Result<()> = self.agent.send_message(receiver, a2a_content).await;
            result.map_err(|e| anyhow::anyhow!("Failed to send A2A message: {}", e))?;
        }

        debug!("Message output via A2A from {}", sender);
        Ok(())
    }

    async fn get_context(&self, limit: usize) -> Result<String> {
        let context = self.store.get_context_string(limit).await;
        if context.is_empty() {
            Ok("(No previous context)".to_string())
        } else {
            Ok(context)
        }
    }

    fn mode(&self) -> OutputMode {
        OutputMode::A2A
    }
}

/// 混合输出（同时输出到多个目标）
pub struct HybridOutput {
    outputs: Vec<Box<dyn Output>>,
}

impl HybridOutput {
    #[allow(dead_code)]
    pub fn new(outputs: Vec<Box<dyn Output>>) -> Self {
        Self { outputs }
    }
}

#[async_trait]
impl Output for HybridOutput {
    async fn send_message(&self, sender: &str, content: &str) -> Result<()> {
        let mut last_error = None;

        for output in &self.outputs {
            if let Err(e) = output.send_message(sender, content).await {
                error!("Output {:?} failed: {}", output.mode(), e);
                last_error = Some(e);
            }
        }

        match last_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    async fn get_context(&self, limit: usize) -> Result<String> {
        // 使用第一个输出的上下文
        if let Some(first) = self.outputs.first() {
            first.get_context(limit).await
        } else {
            Ok("(No context available)".to_string())
        }
    }

    fn mode(&self) -> OutputMode {
        OutputMode::Matrix // 返回默认模式
    }
}

/// 输出工厂
pub struct OutputFactory;

impl OutputFactory {
    /// 创建 Matrix 输出
    pub fn create_matrix(
        homeserver: String,
        token: String,
        room_id: String,
        store: MessageStore,
    ) -> Result<Box<dyn Output>> {
        let client = MatrixClient::new(homeserver, token);
        Ok(Box::new(MatrixOutput::new(client, room_id, store)))
    }

    /// 创建 CLI 输出
    pub fn create_cli(store: MessageStore, echo: bool) -> Box<dyn Output> {
        Box::new(CliOutput::new(store, echo))
    }

    /// 创建 A2A 输出
    pub fn create_a2a(
        agent: Arc<A2AAgent>,
        store: MessageStore,
        default_receiver: Option<String>,
    ) -> Box<dyn Output> {
        Box::new(A2AOutput::new(agent, store, default_receiver))
    }
}

/// 输出桥接器（用于 Matrix <-> A2A 双向同步）
#[allow(dead_code)]
pub struct OutputBridge {
    matrix_output: Option<Arc<MatrixOutput>>,
    a2a_output: Option<Arc<A2AOutput>>,
    a2a_message_rx: mpsc::Receiver<A2AMessage>,
}

impl OutputBridge {
    #[allow(dead_code)]
    pub fn new(
        matrix_output: Option<Arc<MatrixOutput>>,
        a2a_output: Option<Arc<A2AOutput>>,
    ) -> (Self, mpsc::Sender<A2AMessage>) {
        let (tx, rx) = mpsc::channel(100);
        (
            Self {
                matrix_output,
                a2a_output,
                a2a_message_rx: rx,
            },
            tx,
        )
    }

    /// 运行桥接循环，同步消息
    #[allow(dead_code)]
    pub async fn run(mut self) -> Result<()> {
        info!("Output bridge started");

        while let Some(message) = self.a2a_message_rx.recv().await {
            match &message.content {
                MessageContent::Text { text } => {
                    // A2A -> Matrix
                    if let Some(ref matrix) = self.matrix_output {
                        let sender = &message.sender;
                        if let Err(e) = matrix.send_message(sender, text).await {
                            error!("Failed to bridge A2A message to Matrix: {}", e);
                        } else {
                            debug!("Bridged A2A message from {} to Matrix", sender);
                        }
                    }
                }
                _ => {
                    // 其他类型消息暂不处理
                    debug!("Ignoring non-text A2A message in bridge");
                }
            }
        }

        info!("Output bridge stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::types::create_default_agent_card;

    #[test]
    fn test_output_mode_from_str() {
        assert_eq!("matrix".parse::<OutputMode>().unwrap(), OutputMode::Matrix);
        assert_eq!("cli".parse::<OutputMode>().unwrap(), OutputMode::Cli);
        assert_eq!("a2a".parse::<OutputMode>().unwrap(), OutputMode::A2A);
        assert!("unknown".parse::<OutputMode>().is_err());
    }

    #[test]
    fn test_output_mode_display() {
        assert_eq!(OutputMode::Matrix.to_string(), "matrix");
        assert_eq!(OutputMode::Cli.to_string(), "cli");
        assert_eq!(OutputMode::A2A.to_string(), "a2a");
    }

    #[tokio::test]
    async fn test_cli_output() {
        let store = MessageStore::new(10);
        let cli = CliOutput::new(store.clone(), false);

        cli.send_message("test-agent", "Hello CLI").await.unwrap();

        let context = cli.get_context(10).await.unwrap();
        assert!(context.contains("Hello CLI"));
    }

    #[tokio::test]
    async fn test_a2a_output_context() {
        let card = create_default_agent_card("agent-1", "Test Agent");
        let agent = Arc::new(A2AAgent::new(card));
        let store = MessageStore::new(10);
        let a2a = A2AOutput::new(agent, store.clone(), None);

        a2a.send_message("test-agent", "Hello A2A").await.unwrap();

        let context = a2a.get_context(10).await.unwrap();
        assert!(context.contains("Hello A2A"));
    }
}
