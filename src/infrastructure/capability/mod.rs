//! Capability 执行器接口
//!
//! 框架只定义执行器能力接口，具体实现由应用层提供

mod executor;
pub use executor::{CapabilityExecutor, CapabilityExecutorRegistry, CapabilityResult, FnCapabilityExecutor};

mod protocol_handler;
pub use protocol_handler::McpProtocolHandler;

mod mcp_server;
pub use mcp_server::McpServer;

mod mcp_client;
pub use mcp_server::McpClient;  // McpClient is defined in mcp_server.rs
pub use mcp_client::{McpHttpClient, McpWebSocketClient, McpSseClient, McpStdioClient, McpTransport};

// Re-export commonly used types
pub use crate::domain::capability::{
    Capability, CapabilityPath, CapabilityCallContext, CapabilityProvider, MatchType,
    CapabilityNodeInfo, CapabilityAccessType, SkillCapabilityBinding, BindingType
};
pub use crate::core::capability::{CapabilityRegistry, CapabilityNode};
pub use crate::core::capability_provider::{CompositeCapabilityProvider, RegistryCapabilityProvider};