//! 虚拟公司框架
//!
//! 提供多 Agent 协作的基础能力：
//! - Agent 管理（基于 swarms-rs）
//! - 消息通信（私聊、群聊、广播）
//! - 消息路由（本地、远程、跨节点）
//! - A2A 协议（HTTP 服务发现和通信）
//! - 工具调用（execute_command, fetch_url）

pub mod a2a;
pub mod a2a_client;
pub mod a2a_server;
pub mod agent;
pub mod framework;
pub mod messaging;
pub mod router;
pub mod tool;

// 内部模块，不对外暴露
mod config;
mod llm;
mod logger;
mod matrix;
mod output;
mod store;

// 重新导出常用类型
pub use a2a_client::{A2AClient, AgentNetwork};
pub use a2a_server::{A2AServer, AgentInfo};
pub use agent::{Agent, AgentConfig, AgentManager};
pub use framework::{AppBuilder, VirtualCompany};
pub use messaging::{AgentMessageReceiver, GroupInfo, Message, MessageBus, MessageType};
pub use router::{AgentConnector, MessageRouter, RouteTarget};
pub use tool::{Tool, ToolRegistry};

/// 框架版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
