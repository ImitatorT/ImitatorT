//! 虚拟公司框架
//!
//! 提供多 Agent 协作的基础能力：
//! - Agent 管理（基于 swarms-rs）
//! - 消息通信（私聊、群聊、广播）
//! - 消息路由（本地、远程、跨节点）
//! - A2A 协议（HTTP 服务发现和通信）
//! - 工具调用（execute_command, fetch_url）
//!
//! # 架构分层
//!
//! - `core`: 核心层，包含领域模型和通用能力
//! - `infrastructure`: 基础设施层，外部系统交互
//! - `protocol`: 协议层，A2A 协议实现
//! - `application`: 应用层，业务编排

// 核心层
pub mod core;

// 基础设施层
pub mod infrastructure;

// 协议层
pub mod protocol;

// 应用层
pub mod application;

// 工具模块
pub mod utils;

// 重新导出核心类型（向后兼容）
pub use core::agent::{Agent, AgentConfig, AgentManager};
pub use core::config::{AppConfig, OutputMode, StoreType};
pub use core::messaging::{AgentMessageReceiver, GroupInfo, Message, MessageBus, MessageType};
pub use core::store::{ChatMessage, MessageStore, MessageType as StoreMessageType};

// 重新导出基础设施类型（向后兼容）
pub use infrastructure::llm::{Message as LlmMessage, OpenAIClient};
pub use infrastructure::logger;
pub use infrastructure::matrix::MatrixClient;

// 重新导出协议类型（向后兼容）
pub use protocol::client::{A2AClient, AgentNetwork};
pub use protocol::router::{AgentConnector, MessageRouter, RouteTarget};
pub use protocol::server::{A2AServer, AgentInfo};
pub use protocol::types::{
    create_default_agent_card, AgentCard, AgentEndpoints, A2AAgent, A2AMessage, Artifact,
    MessageContent, Skill, TaskRequest, TaskResult, TextMessageHandler,
};

// 重新导出应用类型（向后兼容）
pub use application::framework::{AppBuilder, VirtualCompany};
pub use application::output::{
    A2AOutput, CliOutput, HybridOutput, MatrixOutput, Output, OutputBridge, OutputFactory,
    OutputMode as AppOutputMode,
};
pub use application::tool::{Tool, ToolCall, ToolRegistry};

/// 框架版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
