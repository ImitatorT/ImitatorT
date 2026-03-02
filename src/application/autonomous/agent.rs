//! 自主Agent实现
//!
//! 能够自主接收消息、做出决策并执行

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info};

use crate::core::agent::{AgentRuntime, Context, Decision};
use crate::core::messaging::MessageBus;
use crate::core::watchdog_agent::WatchdogAgent;
use crate::domain::{Agent, Message, MessageTarget};

/// 自主Agent
///
/// 封装Agent运行时和消息处理能力
#[derive(Clone)]
pub struct AutonomousAgent {
    runtime: Arc<AgentRuntime>,
    message_bus: Arc<MessageBus>,
    message_tx: broadcast::Sender<Message>,
}

impl AutonomousAgent {
    /// 创建新的自主Agent
    pub async fn new(
        agent: Agent,
        message_bus: Arc<MessageBus>,
        watchdog_agent: Option<Arc<WatchdogAgent>>,
    ) -> Result<Self> {
        let runtime = Arc::new(AgentRuntime::new(agent).await?);

        let (message_tx, _) = broadcast::channel(100);

        // 如果提供了WatchdogAgent，则自动注册默认唤醒机制
        if let Some(wa) = &watchdog_agent {
            if let Err(e) = wa.register_default_watchers(runtime.id()) {
                error!(
                    "Failed to register default watchers for agent {}: {}",
                    runtime.id(),
                    e
                );
            } else {
                info!(
                    "Successfully registered default watchers for agent {}",
                    runtime.id()
                );
            }
        }

        let autonomous_agent = Self {
            runtime,
            message_bus,
            message_tx,
        };

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

    /// 激活Agent处理指定上下文
    pub async fn activate(&self, context: Context) -> Result<()> {
        info!("Agent {} activated with context", self.id());

        // 获取可用的工具（这里可以从WatchdogAgent或其他地方获取）
        // 在实际实现中，这应该连接到工具注册表来获取该Agent有权访问的工具
        let tools = vec![]; // Placeholder - in a real implementation, this would come from registered tools

        // 使用ReAct模式进行思考和决策
        match self.runtime.react_think(context, tools).await {
            Ok(decision) => {
                debug!("Agent {} decision: {:?}", self.id(), decision);

                // 执行决策
                if let Err(e) = self.execute_decision(decision).await {
                    error!("Agent {} failed to execute decision: {}", self.id(), e);
                }
            }
            Err(e) => {
                error!("Agent {} ReAct think error: {}", self.id(), e);
            }
        }

        Ok(())
    }

    /// 执行决策
    async fn execute_decision(&self, decision: Decision) -> Result<()> {
        match decision {
            Decision::SendMessage { target, content } => {
                let msg = match target {
                    MessageTarget::Direct(to) => Message::private(self.id(), to, content),
                    MessageTarget::Group(group_id) => Message::group(self.id(), group_id, content),
                };

                let _ = self.message_tx.send(msg.clone());
                info!("Agent {} sent message: {:?}", self.id(), msg);
            }
            Decision::CreateGroup { name, members } => {
                info!(
                    "Agent {} wants to create group: {} with members: {:?}",
                    self.id(),
                    name,
                    members
                );
                // 使用当前时间戳作为群组ID的一部分，确保唯一性
                let group_id = format!(
                    "group_{}_{}",
                    name.replace(" ", "_"),
                    chrono::Utc::now().timestamp()
                );

                // 将发起创建的Agent也加入群组
                let mut all_members = members;
                if !all_members.contains(&self.id().to_string()) {
                    all_members.push(self.id().to_string());
                }

                match self
                    .message_bus
                    .create_group(&group_id, &name, self.id(), all_members)
                    .await
                {
                    Ok(()) => {
                        info!(
                            "Agent {} successfully created group: {}",
                            self.id(),
                            group_id
                        );
                    }
                    Err(e) => {
                        error!(
                            "Agent {} failed to create group {}: {}",
                            self.id(),
                            group_id,
                            e
                        );
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
