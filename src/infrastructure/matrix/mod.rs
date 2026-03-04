//! Matrix 平台集成模块
//!
//! 提供与 Matrix Homeserver 的完整集成，支持：
//! - 虚拟用户管理
//! - 房间同步
//! - 消息发送/接收
//! - Appservice 事件处理

pub mod config;
pub mod client;
pub mod appservice;
pub mod events;
pub mod mapper;
pub mod sync;

pub use config::MatrixConfig;
pub use client::MatrixClient;
pub use appservice::AppService;
pub use mapper::Mapper;
pub use sync::{SyncService, MatrixNotifier};
