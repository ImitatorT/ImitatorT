//! 自主Agent实现
//!
//! 能够自主接收消息、做出决策并执行

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info};

use crate::core::agent::{AgentRuntime, Context, Decision};
use crate::core::messaging::{MessageBus, MessageReceiver};
use crate::core::watchdog_agent::WatchdogAgent;
use crate::domain::{Agent, Message, MessageTarget};

/// 自主Agent
///
/// 封装Agent运行时和消息处理能力
#[derive(Clone)]
pub struct AutonomousAgent {
    runtime: Arc<AgentRuntime>,
    message_bus: Arc<MessageBus>,
    message_rx: Arc<RwLock<MessageReceiver>>,
    message_tx: broadcast::Sender<Message>,
    pending_task: Arc<RwLock<Option<String>>>,
    watchdog_agent: Option<Arc<WatchdogAgent>>,
}

impl AutonomousAgent {
    /// 创建新的自主Agent
    pub async fn new(
        agent: Agent,
        message_bus: Arc<MessageBus>,
        watchdog_agent: Option<Arc<WatchdogAgent>>,
    ) -> Result<Self> {
        let runtime = Arc::new(AgentRuntime::new(agent).await?);

        // 注册到消息总线
        let private_rx = message_bus.register(runtime.id());

        let message_rx = Arc::new(RwLock::new(MessageReceiver::new(
            runtime.id().to_string(),
            private_rx,
        )));

        let (message_tx, _) = broadcast::channel(100);

        let autonomous_agent = Self {
            runtime,
            message_bus,
            message_rx,
            message_tx,
            pending_task: Arc::new(RwLock::new(None)),
            watchdog_agent: watchdog_agent.clone(),
        };

        // 如果提供了WatchdogAgent，则自动注册默认唤醒机制
        if let Some(wa) = &watchdog_agent {
            if let Err(e) = wa.register_default_watchers(autonomous_agent.id()) {
                error!("Failed to register default watchers for agent {}: {}", autonomous_agent.id(), e);
            } else {
                info!("Successfully registered default watchers for agent {}", autonomous_agent.id());
            }
        }

        Ok(autonomous_agent)
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
                        error!("Agent {} failed to execute decision: {}", self.id(), e);
                        // 在这里我们可以考虑实现重试逻辑或其他恢复机制
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
                };

                let _ = self.message_tx.send(msg.clone());
                info!("Agent {} sent message: {:?}", self.id(), msg);
            }
            Decision::CreateGroup { name, members } => {
                info!("Agent {} wants to create group: {} with members: {:?}", self.id(), name, members);
                // 使用当前时间戳作为群组ID的一部分，确保唯一性
                let group_id = format!("group_{}_{}", name.replace(" ", "_"), chrono::Utc::now().timestamp());

                // 将发起创建的Agent也加入群组
                let mut all_members = members;
                if !all_members.contains(&self.id().to_string()) {
                    all_members.push(self.id().to_string());
                }

                match self.message_bus.create_group(
                    &group_id,
                    &name,
                    self.id(),
                    all_members
                ).await {
                    Ok(()) => {
                        info!("Agent {} successfully created group: {}", self.id(), group_id);
                    }
                    Err(e) => {
                        error!("Agent {} failed to create group {}: {}", self.id(), group_id, e);
                    }
                }
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
            Decision::Error(error_msg) => {
                error!("Agent {} received error decision: {}", self.id(), error_msg);
            }
        }

        Ok(())
    }
}
