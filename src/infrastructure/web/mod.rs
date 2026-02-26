//! Web 服务器模块
//!
//! 提供 HTTP API 和 WebSocket 支持

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
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
use crate::domain::user::User;
use crate::infrastructure::auth::{JwtService, PasswordService, UserInfo};

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
    pub store: Arc<dyn crate::core::store::Store>,
    pub jwt_service: JwtService,
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

// ==================== 客户端消息类型 ====================

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "send_message")]
    SendMessage {
        from: String,
        to: String,
        content: String,
    },
    #[serde(rename = "ping")]
    Ping,
}


// 为简化，我们创建一个验证JWT的辅助函数
#[allow(dead_code)]
async fn validate_jwt(state: &AppState, token: &str) -> Option<UserInfo> {
    state.jwt_service.validate_token(token).ok()
}

/// 获取当前用户信息
async fn get_current_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // 从头部获取授权信息
    let auth_header = headers.get("authorization")
        .and_then(|value| value.to_str().ok());

    if let Some(auth_str) = auth_header {
        if auth_str.starts_with("Bearer ") {
            let token = &auth_str[7..];

            if let Ok(user_info) = state.jwt_service.validate_token(token) {
                return Json(serde_json::json!({
                    "success": true,
                    "data": {
                        "id": user_info.id,
                        "username": user_info.username,
                        "name": user_info.name,
                        "email": user_info.email,
                        "is_director": user_info.is_director,
                    }
                })).into_response();
            }
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "Unauthorized".to_string(),
        })
    ).into_response()
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

/// 登录
async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AuthRequest>,
) -> impl IntoResponse {
    info!("Login attempt: {}", req.username);

    // 从数据库查找用户
    match state.store.load_user_by_username(&req.username).await {
        Ok(Some(user)) => {
            // 验证密码
            match PasswordService::verify_password(&user.password_hash, &req.password) {
                Ok(valid) if valid => {
                    // 生成JWT令牌
                    let token: String = match state.jwt_service.generate_token(&UserInfo {
                        id: user.id.clone(),
                        username: user.username.clone(),
                        name: user.name.clone(),
                        email: user.email.clone(),
                        is_director: user.is_director,
                    }) {
                        Ok(token) => token,
                        Err(e) => {
                            error!("Failed to generate token: {}", e);
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ErrorResponse {
                                    error: "Failed to generate token".to_string(),
                                })
                            ).into_response();
                        }
                    };

                    Json(serde_json::json!({
                        "success": true,
                        "data": {
                            "token": token,
                            "user": {
                                "id": user.id,
                                "username": user.username,
                                "name": user.name,
                                "email": user.email,
                                "is_director": user.is_director,
                            }
                        }
                    })).into_response()
                },
                Ok(_) | Err(_) => {
                    // 密码错误
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(ErrorResponse {
                            error: "Invalid username or password".to_string(),
                        })
                    ).into_response()
                }
            }
        },
        Ok(None) => {
            // 用户不存在
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid username or password".to_string(),
                })
            ).into_response()
        },
        Err(e) => {
            error!("Database error during login: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                })
            ).into_response()
        }
    }
}

/// 注册
async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    info!("Register attempt: {}", req.username);

    // 检查用户名是否已存在
    match state.store.load_user_by_username(&req.username).await {
        Ok(Some(_)) => {
            // 用户名已存在
            return (
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: "Username already exists".to_string(),
                })
            ).into_response();
        },
        Ok(None) => {
            // 用户名不存在，可以创建
        },
        Err(e) => {
            error!("Database error during registration check: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                })
            ).into_response();
        }
    }

    // 哈希密码
    let password_hash = match PasswordService::hash_password(&req.password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Failed to hash password: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to process password".to_string(),
                })
            ).into_response();
        }
    };

    // 创建用户
    let user = User::new(
        req.username.clone(),
        req.name.clone(),
        password_hash,
        req.email.clone(),
    );

    // 保存用户到数据库
    if let Err(e) = state.store.save_user(&user).await {
        error!("Failed to save user: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to register user".to_string(),
            })
        ).into_response();
    }

    // 生成JWT令牌
    let token: String = match state.jwt_service.generate_token(&UserInfo {
        id: user.id.clone(),
        username: user.username.clone(),
        name: user.name.clone(),
        email: user.email.clone(),
        is_director: user.is_director,
    }) {
        Ok(token) => token,
        Err(e) => {
            error!("Failed to generate token: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to generate token".to_string(),
                })
            ).into_response();
        }
    };

    Json(serde_json::json!({
        "success": true,
        "data": {
            "token": token,
            "user": {
                "id": user.id,
                "username": user.username,
                "name": user.name,
                "email": user.email,
                "is_director": user.is_director,
            }
        }
    })).into_response()
}

/// 检查用户名
async fn check_username(Query(params): Query<std::collections::HashMap<String, String>>) -> impl IntoResponse {
    let username = params.get("username").cloned().unwrap_or_default();
    let exists = username == "admin" || username == "director";

    Json(serde_json::json!({
        "success": true,
        "data": {
            "exists": exists,
            "available": !exists,
        }
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
                    axum::extract::ws::Message::Text(text) => {
                        // 解析客户端发送的消息
                        if let Ok(client_message) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_message {
                                ClientMessage::SendMessage { from, to, content } => {
                                    // 验证消息参数
                                    if from.is_empty() || to.is_empty() || content.is_empty() {
                                        // 发送错误响应
                                        let error_msg = serde_json::json!({
                                            "type": "error",
                                            "message": "Invalid message format: missing required fields"
                                        });

                                        if let Err(e) = socket.send(axum::extract::ws::Message::Text(
                                            error_msg.to_string().into()
                                        )).await {
                                            error!("WebSocket send error: {}", e);
                                            break;
                                        }
                                        continue;
                                    }

                                    // 构造消息目标
                                    let target = if to.starts_with("group:") {
                                        MessageTarget::Group(to.strip_prefix("group:").unwrap_or(&to).to_string())
                                    } else {
                                        MessageTarget::Direct(to)
                                    };

                                    // 创建消息
                                    let message = Message {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        from,
                                        to: target,
                                        content,
                                        timestamp: Utc::now().timestamp(),
                                        reply_to: None,
                                        mentions: Vec::new(),
                                    };

                                    // 发送消息到消息总线
                                    if let Err(e) = state.message_tx.send(message.clone()) {
                                        error!("Failed to send message: {}", e);
                                    }
                                }
                                ClientMessage::Ping => {
                                    // 回复pong消息
                                    let pong_msg = serde_json::json!({
                                        "type": "pong"
                                    });

                                    if let Err(e) = socket.send(axum::extract::ws::Message::Text(
                                        pong_msg.to_string().into()
                                    )).await {
                                        error!("WebSocket send error: {}", e);
                                        break;
                                    }
                                }
                            }
                        } else {
                            // 解析JSON失败，发送错误响应
                            let error_msg = serde_json::json!({
                                "type": "error",
                                "message": "Invalid JSON format"
                            });

                            if let Err(e) = socket.send(axum::extract::ws::Message::Text(
                                error_msg.to_string().into()
                            )).await {
                                error!("WebSocket send error: {}", e);
                                break;
                            }
                        }
                    }
                    _ => {
                        // 忽略其他类型的消息
                        continue;
                    }
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
        .route("/api/auth/current", get(get_current_user))
        .route("/ws", get(websocket_handler))
        .layer(cors)
        .with_state(state)
}

// ==================== 服务器启动 ====================

pub async fn start_web_server(
    bind_addr: &str,
    agents: Vec<Agent>,
    message_tx: broadcast::Sender<Message>,
    store: Arc<dyn crate::core::store::Store>,
) -> anyhow::Result<()> {
    // 创建JWT服务
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret_key_for_dev".to_string());
    let jwt_service = JwtService::new(&jwt_secret);

    let state = Arc::new(AppState {
        agents,
        message_tx,
        store,
        jwt_service,
    });

    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    info!("Web server started on http://{}", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
