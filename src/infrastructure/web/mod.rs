//! Web 服务器模块
//!
//! 提供 HTTP API 和 WebSocket 支持

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

use crate::domain::{Agent, Message, MessageTarget};

// ==================== 错误响应 ====================

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// ==================== 状态 ====================

#[derive(Clone)]
pub struct AppState {
    pub agents: Vec<Agent>,
    pub message_tx: broadcast::Sender<Message>,
}

// ==================== API 响应类型 ====================

#[derive(Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub role: String,
    pub department: String,
}

// ==================== 请求类型 ====================

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub from: String,
    pub to: Option<String>,
    pub content: String,
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub name: String,
    pub email: Option<String>,
}

// ==================== 处理器 ====================

/// 健康检查
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": Utc::now().to_rfc3339(),
    }))
}

/// 获取 Agent 列表
async fn list_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let agents: Vec<AgentResponse> = state
        .agents
        .iter()
        .map(|a| AgentResponse {
            id: a.id.clone(),
            name: a.name.clone(),
            role: a.role.title.clone(),
            department: a.department_id.clone().unwrap_or_default(),
        })
        .collect();

    Json(agents)
}

/// 获取单个 Agent
async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> impl IntoResponse {
    match state.agents.iter().find(|a| a.id == agent_id) {
        Some(agent) => Json(serde_json::json!({
            "id": agent.id,
            "name": agent.name,
            "role": agent.role.title,
            "department": agent.department_id,
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Agent not found: {}", agent_id),
            }),
        )
            .into_response(),
    }
}

/// 获取公司信息
async fn get_company(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let departments: Vec<String> = state
        .agents
        .iter()
        .filter_map(|a| a.department_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    Json(serde_json::json!({
        "name": "ImitatorT Virtual Company",
        "agent_count": state.agents.len(),
        "departments": departments,
    }))
}

/// 发送消息
async fn send_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let to = if let Some(to_id) = req.to {
        MessageTarget::Direct(to_id)
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing 'to' field".to_string(),
            }),
        )
            .into_response();
    };

    let message = Message {
        id: uuid::Uuid::new_v4().to_string(),
        from: req.from,
        to,
        content: req.content,
        timestamp: Utc::now().timestamp(),
        reply_to: None,
        mentions: Vec::new(),
    };

    // 发送消息
    let _ = state.message_tx.send(message.clone());

    Json(serde_json::json!({
        "id": message.id,
        "status": "sent",
        "timestamp": message.timestamp,
    }))
    .into_response()
}

/// 登录（模拟）
async fn login(Json(req): Json<AuthRequest>) -> impl IntoResponse {
    info!("Login attempt: {}", req.username);

    Json(serde_json::json!({
        "success": true,
        "token": format!("mock_token_{}", req.username),
        "user": {
            "id": req.username,
            "username": req.username,
            "name": req.username,
            "is_director": req.username == "admin" || req.username == "director",
        }
    }))
}

/// 注册（模拟）
async fn register(Json(req): Json<RegisterRequest>) -> impl IntoResponse {
    info!("Register attempt: {}", req.username);

    Json(serde_json::json!({
        "success": true,
        "token": format!("mock_token_{}", req.username),
        "user": {
            "id": req.username,
            "username": req.username,
            "name": req.name,
            "is_director": false,
        }
    }))
}

/// 检查用户名
async fn check_username(Query(params): Query<std::collections::HashMap<String, String>>) -> impl IntoResponse {
    let username = params.get("username").cloned().unwrap_or_default();
    let exists = username == "admin" || username == "director";

    Json(serde_json::json!({
        "exists": exists,
        "available": !exists,
    }))
}

/// WebSocket 处理
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(
    mut socket: axum::extract::ws::WebSocket,
    state: Arc<AppState>,
) {
    let mut rx = state.message_tx.subscribe();

    info!("WebSocket connection established");

    loop {
        tokio::select! {
            // 接收消息
            Ok(message) = rx.recv() => {
                let to_str = match &message.to {
                    MessageTarget::Direct(id) => Some(id.clone()),
                    MessageTarget::Group(id) => Some(format!("group:{}", id)),
                };

                let msg_json = serde_json::json!({
                    "type": "message",
                    "data": {
                        "id": message.id,
                        "from": message.from,
                        "to": to_str,
                        "content": message.content,
                        "timestamp": message.timestamp,
                    }
                });

                if let Err(e) = socket.send(axum::extract::ws::Message::Text(
                    msg_json.to_string().into()
                )).await {
                    error!("WebSocket send error: {}", e);
                    break;
                }
            }

            // 接收客户端消息
            Some(Ok(msg)) = socket.recv() => {
                match msg {
                    axum::extract::ws::Message::Close(_) => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    axum::extract::ws::Message::Ping(ping) => {
                        if let Err(e) = socket.send(axum::extract::ws::Message::Pong(ping)).await {
                            error!("WebSocket pong error: {}", e);
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// ==================== 路由 ====================

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::permissive();

    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/company", get(get_company))
        .route("/api/agents", get(list_agents))
        .route("/api/agents/{id}", get(get_agent))
        .route("/api/messages", post(send_message))
        .route("/api/auth/login", post(login))
        .route("/api/auth/register", post(register))
        .route("/api/auth/check-username", get(check_username))
        .route("/ws", get(websocket_handler))
        .layer(cors)
        .with_state(state)
}

// ==================== 服务器启动 ====================

pub async fn start_web_server(
    bind_addr: &str,
    agents: Vec<Agent>,
    message_tx: broadcast::Sender<Message>,
) -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        agents,
        message_tx,
    });

    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    info!("Web server started on http://{}", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
