//! A2A HTTP 服务端
//!
//! 提供 Agent 间的 HTTP 通信能力：
//! - Agent 注册和发现
//! - 消息发送和接收
//! - 健康检查
//!
//! 设计原则：
//! - 每个 Agent 可以运行独立的 HTTP 服务
//! - Agent 通过 HTTP 互相发现和通信
//! - 支持跨进程、跨机器通信

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::core::messaging::{Message, MessageBus};

/// Agent 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Agent 注册请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterAgentRequest {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// 发送消息请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub from: String,
    pub to: Vec<String>,
    pub content: String,
    #[serde(rename = "type")]
    pub msg_type: String,
}

/// 创建群聊请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupRequest {
    pub group_id: String,
    pub name: String,
    pub creator: String,
    pub members: Vec<String>,
}

/// 邀请成员请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteMemberRequest {
    pub group_id: String,
    pub inviter: String,
    pub invitee: String,
}

/// API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

/// A2A 服务状态
pub struct A2AServerState {
    /// 本地 Agent 信息
    local_agent: RwLock<Option<AgentInfo>>,
    /// 已发现的远程 Agent
    remote_agents: RwLock<std::collections::HashMap<String, AgentInfo>>,
    /// 消息总线
    message_bus: Arc<MessageBus>,
    /// 服务端地址
    #[allow(dead_code)]
    bind_addr: SocketAddr,
}

impl A2AServerState {
    pub fn new(message_bus: Arc<MessageBus>, bind_addr: SocketAddr) -> Self {
        Self {
            local_agent: RwLock::new(None),
            remote_agents: RwLock::new(std::collections::HashMap::new()),
            message_bus,
            bind_addr,
        }
    }

    /// 设置本地 Agent 信息
    pub async fn set_local_agent(&self, info: AgentInfo) {
        let mut local = self.local_agent.write().await;
        *local = Some(info);
    }

    /// 获取本地 Agent 信息
    pub async fn get_local_agent(&self) -> Option<AgentInfo> {
        self.local_agent.read().await.clone()
    }

    /// 注册远程 Agent
    pub async fn register_remote_agent(&self, info: AgentInfo) {
        let mut agents = self.remote_agents.write().await;
        info!("Registered remote agent: {} at {}", info.id, info.endpoint);
        agents.insert(info.id.clone(), info);
    }

    /// 获取远程 Agent
    pub async fn get_remote_agent(&self, id: &str) -> Option<AgentInfo> {
        self.remote_agents.read().await.get(id).cloned()
    }

    /// 列出所有远程 Agent
    pub async fn list_remote_agents(&self) -> Vec<AgentInfo> {
        self.remote_agents.read().await.values().cloned().collect()
    }

    /// 移除远程 Agent
    pub async fn remove_remote_agent(&self, id: &str) {
        self.remote_agents.write().await.remove(id);
    }
}

/// 健康检查
async fn health_check() -> StatusCode {
    StatusCode::OK
}

/// 获取本地 Agent 信息
async fn get_local_agent(
    State(state): State<Arc<A2AServerState>>,
) -> Json<ApiResponse<Option<AgentInfo>>> {
    let agent = state.get_local_agent().await;
    Json(ApiResponse::success(agent))
}

/// 注册本地 Agent
async fn register_local_agent(
    State(state): State<Arc<A2AServerState>>,
    Json(req): Json<RegisterAgentRequest>,
) -> Json<ApiResponse<String>> {
    let info = AgentInfo {
        id: req.id.clone(),
        name: req.name,
        endpoint: req.endpoint,
        capabilities: req.capabilities,
        metadata: req.metadata,
    };

    state.set_local_agent(info).await;

    // 注册到本地消息总线
    state.message_bus.register_agent(&req.id);

    info!("Local agent registered: {}", req.id);
    Json(ApiResponse::success("Agent registered".to_string()))
}

/// 发现远程 Agent
async fn discover_agents(
    State(state): State<Arc<A2AServerState>>,
) -> Json<ApiResponse<Vec<AgentInfo>>> {
    let agents = state.list_remote_agents().await;
    Json(ApiResponse::success(agents))
}

/// 注册远程 Agent
async fn register_remote_agent(
    State(state): State<Arc<A2AServerState>>,
    Json(req): Json<RegisterAgentRequest>,
) -> Json<ApiResponse<String>> {
    let info = AgentInfo {
        id: req.id,
        name: req.name,
        endpoint: req.endpoint,
        capabilities: req.capabilities,
        metadata: req.metadata,
    };

    state.register_remote_agent(info).await;
    Json(ApiResponse::success("Remote agent registered".to_string()))
}

/// 发送消息
async fn send_message(
    State(state): State<Arc<A2AServerState>>,
    Json(req): Json<SendMessageRequest>,
) -> Json<ApiResponse<String>> {
    let msg_type = match req.msg_type.as_str() {
        "private" => crate::core::messaging::MessageType::Private,
        "group" => crate::core::messaging::MessageType::Group,
        "broadcast" => crate::core::messaging::MessageType::Broadcast,
        _ => {
            return Json(ApiResponse::error(format!(
                "Unknown message type: {}",
                req.msg_type
            )));
        }
    };

    let message = Message {
        id: uuid::Uuid::new_v4().to_string(),
        msg_type,
        from: req.from,
        to: req.to,
        content: req.content,
        timestamp: chrono::Utc::now().timestamp(),
        metadata: None,
    };

    match state.message_bus.send(message).await {
        Ok(_) => Json(ApiResponse::success("Message sent".to_string())),
        Err(e) => {
            error!("Failed to send message: {}", e);
            Json(ApiResponse::error(e.to_string()))
        }
    }
}

/// 创建群聊
async fn create_group(
    State(state): State<Arc<A2AServerState>>,
    Json(req): Json<CreateGroupRequest>,
) -> Json<ApiResponse<String>> {
    match state
        .message_bus
        .create_group(&req.group_id, &req.name, &req.creator, req.members)
        .await
    {
        Ok(group_id) => Json(ApiResponse::success(group_id)),
        Err(e) => {
            error!("Failed to create group: {}", e);
            Json(ApiResponse::error(e.to_string()))
        }
    }
}

/// 邀请成员
async fn invite_member(
    State(state): State<Arc<A2AServerState>>,
    Json(req): Json<InviteMemberRequest>,
) -> Json<ApiResponse<String>> {
    match state
        .message_bus
        .invite_to_group(&req.group_id, &req.inviter, &req.invitee)
        .await
    {
        Ok(_) => Json(ApiResponse::success("Member invited".to_string())),
        Err(e) => {
            error!("Failed to invite member: {}", e);
            Json(ApiResponse::error(e.to_string()))
        }
    }
}

/// 获取群聊信息
async fn get_group(
    State(state): State<Arc<A2AServerState>>,
    Path(group_id): Path<String>,
) -> Result<Json<ApiResponse<Option<crate::core::messaging::GroupInfo>>>, StatusCode> {
    let group = state.message_bus.get_group(&group_id).await;
    Ok(Json(ApiResponse::success(group)))
}

/// A2A HTTP 服务端
pub struct A2AServer {
    state: Arc<A2AServerState>,
    bind_addr: SocketAddr,
}

impl A2AServer {
    /// 创建新的 A2A 服务端
    pub fn new(message_bus: Arc<MessageBus>, bind_addr: SocketAddr) -> Self {
        let state = Arc::new(A2AServerState::new(message_bus, bind_addr));

        Self { state, bind_addr }
    }

    /// 获取服务端状态（用于共享）
    pub fn state(&self) -> Arc<A2AServerState> {
        self.state.clone()
    }

    /// 启动服务端
    pub async fn start(&self) -> Result<()> {
        let app = Router::new()
            // 健康检查
            .route("/health", get(health_check))
            // Agent 管理
            .route("/agent", get(get_local_agent))
            .route("/agent/register", post(register_local_agent))
            // 服务发现
            .route("/agents", get(discover_agents))
            .route("/agents/register", post(register_remote_agent))
            // 消息
            .route("/messages", post(send_message))
            // 群聊
            .route("/groups", post(create_group))
            .route("/groups/:group_id", get(get_group))
            .route("/groups/invite", post(invite_member))
            // 状态
            .with_state(self.state.clone());

        info!("Starting A2A HTTP server on {}", self.bind_addr);

        let listener = tokio::net::TcpListener::bind(self.bind_addr)
            .await
            .context("Failed to bind TCP listener")?;

        axum::serve(listener, app).await.context("Server error")?;

        Ok(())
    }

    /// 启动服务端（后台任务）
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = self.start().await {
                error!("A2A server error: {}", e);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_info_creation() {
        let info = AgentInfo {
            id: "agent-001".to_string(),
            name: "Test Agent".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            capabilities: vec!["chat".to_string(), "task".to_string()],
            metadata: None,
        };

        assert_eq!(info.id, "agent-001");
        assert_eq!(info.endpoint, "http://localhost:8080");
    }

    #[test]
    fn test_api_response_success() {
        let resp: ApiResponse<String> = ApiResponse::success("test data".to_string());
        assert!(resp.success);
        assert_eq!(resp.data, Some("test data".to_string()));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let resp: ApiResponse<String> = ApiResponse::error("something went wrong");
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert_eq!(resp.error, Some("something went wrong".to_string()));
    }

    #[tokio::test]
    async fn test_server_state() {
        let bus = Arc::new(MessageBus::new());
        let addr = "127.0.0.1:0".parse().unwrap();
        let state = Arc::new(A2AServerState::new(bus, addr));

        // 测试本地 Agent
        let local = AgentInfo {
            id: "local".to_string(),
            name: "Local".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            capabilities: vec![],
            metadata: None,
        };

        state.set_local_agent(local.clone()).await;
        assert_eq!(state.get_local_agent().await.unwrap().id, "local");

        // 测试远程 Agent
        let remote = AgentInfo {
            id: "remote".to_string(),
            name: "Remote".to_string(),
            endpoint: "http://remote:8080".to_string(),
            capabilities: vec![],
            metadata: None,
        };

        state.register_remote_agent(remote).await;
        assert_eq!(state.list_remote_agents().await.len(), 1);
        assert_eq!(
            state.get_remote_agent("remote").await.unwrap().name,
            "Remote"
        );
    }
}
