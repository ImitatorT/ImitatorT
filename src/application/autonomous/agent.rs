//! 自主Agent实现
//!
//! 能够自主接收消息、做出决策并执行

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

use crate::core::agent::{AgentRuntime, Context, Decision};
use crate::core::messaging::{MessageBus, MessageReceiver};
use crate::domain::{Agent, Message, MessageTarget};

/// 自主Agent
///
/// 封装Agent运行时和消息处理能力
#[derive(Clone)]
pub struct AutonomousAgent {
    runtime: Arc<AgentRuntime>,
    message_rx: Arc<RwLock<MessageReceiver>>,
    message_tx: broadcast::Sender<Message>,
    pending_task: Arc<RwLock<Option<String>>>,
}

impl AutonomousAgent {
    /// 创建新的自主Agent
    pub async fn new(
        agent: Agent,
        message_bus: Arc<MessageBus>,
        broadcast_rx: broadcast::Receiver<Message>,
    ) -> Result<Self> {
        let runtime = Arc::new(AgentRuntime::new(agent).await?);

        // 注册到消息总线
        let private_rx = message_bus.register(runtime.id());

        let message_rx = Arc::new(RwLock::new(MessageReceiver::new(
            runtime.id().to_string(),
            private_rx,
            broadcast_rx,
        )));

        let (message_tx, _) = broadcast::channel(100);

        Ok(Self {
            runtime,
            message_rx,
            message_tx,
            pending_task: Arc::new(RwLock::new(None)),
        })
    }

    /// 获取Agent ID
    pub fn id(&self) -> &str {
        self.runtime.id()
    }

    /// 获取Agent名称
    pub fn name(&self) -> &str {
        self.runtime.name()
    }

    /// 分配任务
    pub fn assign_task(&self, task: impl Into<String>) -> Result<()> {
        let mut pending = self.pending_task.blocking_write();
        *pending = Some(task.into());
        Ok(())
    }

    /// 自主运行循环
    pub async fn run_loop(&self) -> Result<()> {
        info!("Agent {} started autonomous loop", self.id());

        loop {
            // 1. 收集未读消息
            let mut messages = vec![];
            {
                let mut rx = self.message_rx.write().await;
                while let Some(msg) = rx.try_recv() {
                    messages.push(msg);
                }
            }

            // 2. 检查是否有待处理任务
            let task = {
                let mut pending = self.pending_task.write().await;
                pending.take()
            };

            // 3. 构建上下文
            let mut context = Context::default().with_messages(messages);
            if let Some(task) = task {
                context = context.with_task(task);
            }

            // 4. 做出决策
            match self.runtime.think(context).await {
                Ok(decision) => {
                    debug!("Agent {} decision: {:?}", self.id(), decision);

                    // 5. 执行决策
                    if let Err(e) = self.execute_decision(decision).await {
                        warn!("Agent {} failed to execute decision: {}", self.id(), e);
                    }
                }
                Err(e) => {
                    error!("Agent {} think error: {}", self.id(), e);
                }
            }

            // 6. 短暂休眠避免CPU占用过高
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// 执行决策
    async fn execute_decision(&self, decision: Decision) -> Result<()> {
        match decision {
            Decision::SendMessage { target, content } => {
                let msg = match target {
                    MessageTarget::Direct(to) => Message::private(self.id(), to, content),
                    MessageTarget::Group(group_id) => {
                        Message::group(self.id(), group_id, content)
                    }
                    MessageTarget::Broadcast => Message::broadcast(self.id(), content),
                };

                let _ = self.message_tx.send(msg.clone());
                info!("Agent {} sent message: {:?}", self.id(), msg);
            }
            Decision::CreateGroup { name, members: _ } => {
                info!("Agent {} wants to create group: {}", self.id(), name);
                // TODO: 实现群聊创建
            }
            Decision::ExecuteTask { task } => {
                info!("Agent {} executing task: {}", self.id(), task);
                match self.runtime.execute_task(&task).await {
                    Ok(result) => {
                        info!("Agent {} task completed: {}", self.id(), result);
                    }
                    Err(e) => {
                        error!("Agent {} task failed: {}", self.id(), e);
                    }
                }
            }
            Decision::Wait => {
                // 什么都不做，等待下一次循环
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{LLMConfig, Role};

    #[test]
    fn test_autonomous_agent_creation() {
        // 这只是编译时检查，需要真实LLM才能运行
    }
}
