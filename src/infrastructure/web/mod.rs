//! Web 服务器模块
//!
//! 提供 HTTP API 和 WebSocket 支持

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

use crate::domain::invitation_code::InvitationCode;
use crate::domain::user::User;
use crate::domain::{Agent, LLMConfig, Message, MessageTarget, Organization, Role};
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
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());

    if let Some(auth_str) = auth_header {
        if auth_str.starts_with("Bearer ") {
            let token = auth_str.strip_prefix("Bearer ").unwrap();

            if let Ok(user_info) = state.jwt_service.validate_token(token) {
                return Json(serde_json::json!({
                    "success": true,
                    "data": {
                        "id": user_info.id,
                        "username": user_info.username,
                        "name": user_info.name,
                        "email": user_info.email,
                        "is_director": user_info.is_director,
                        "employee_id": user_info.employee_id,
                        "position": user_info.position,
                        "department": user_info.department,
                    }
                }))
                .into_response();
            }
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    )
        .into_response()
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
    pub invite_code: Option<String>, // 邀请码（可选，首位注册不需要）
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
                        is_director: matches!(
                            user.position,
                            crate::domain::user::Position::Chairman
                                | crate::domain::user::Position::Management
                        ),
                        employee_id: user.employee_id.clone(),
                        position: format!("{:?}", user.position),
                        department: user.department.clone(),
                    }) {
                        Ok(token) => token,
                        Err(e) => {
                            error!("Failed to generate token: {}", e);
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ErrorResponse {
                                    error: "Failed to generate token".to_string(),
                                }),
                            )
                                .into_response();
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
                                "is_director": matches!(user.position, crate::domain::user::Position::Chairman | crate::domain::user::Position::Management),
                                "employee_id": user.employee_id,
                                "position": format!("{:?}", user.position),
                                "department": user.department,
                            }
                        }
                    })).into_response()
                }
                Ok(_) | Err(_) => {
                    // 密码错误
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(ErrorResponse {
                            error: "Invalid username or password".to_string(),
                        }),
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => {
            // 用户不存在
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid username or password".to_string(),
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!("Database error during login: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
                .into_response()
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
                }),
            )
                .into_response();
        }
        Ok(None) => {
            // 用户名不存在，可以继续
        }
        Err(e) => {
            error!("Database error during registration check: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
                .into_response();
        }
    }

    // 检查是否为首注册用户（检查是否已有用户）
    let existing_users = match state.store.load_users().await {
        Ok(users) => users,
        Err(e) => {
            error!("Database error during user count check: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
                .into_response();
        }
    };

    let user_to_create = if existing_users.is_empty() {
        // The first registered user automatically becomes the corporate chairman
        if req.invite_code.is_some() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "First user registration does not require an invitation code"
                        .to_string(),
                }),
            )
                .into_response();
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
                    }),
                )
                    .into_response();
            }
        };

        // Create corporate chairman user
        User::new_chairman(
            req.username.clone(),
            req.name.clone(),
            password_hash,
            req.email.clone(),
        )
    } else {
        // 非首位注册，需要邀请码
        if req.invite_code.is_none() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invitation code is required for registration".to_string(),
                }),
            )
                .into_response();
        }

        let invite_code_str = req.invite_code.unwrap();

        // 验证邀请码
        let mut invite_code = match state
            .store
            .load_invitation_code_by_code(&invite_code_str)
            .await
        {
            Ok(Some(code)) => code,
            Ok(None) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Invalid invitation code".to_string(),
                    }),
                )
                    .into_response();
            }
            Err(e) => {
                error!("Database error during invitation code check: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Database error".to_string(),
                    }),
                )
                    .into_response();
            }
        };

        // 检查邀请码是否有效
        if !invite_code.is_valid() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invitation code has expired or reached maximum usage".to_string(),
                }),
            )
                .into_response();
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
                    }),
                )
                    .into_response();
            }
        };

        // 使用邀请码（增加使用次数）
        invite_code.use_code();

        // 更新邀请码状态
        if let Err(e) = state.store.update_invitation_code(&invite_code).await {
            error!("Failed to update invitation code: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update invitation code".to_string(),
                }),
            )
                .into_response();
        }

        // 获取当前管理层用户数量，用于生成工号
        let management_users = match state.store.load_users().await {
            Ok(users) => users
                .into_iter()
                .filter(|u| matches!(u.position, crate::domain::user::Position::Management))
                .collect::<Vec<_>>(),
            Err(e) => {
                error!("Database error during management user count: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Database error".to_string(),
                    }),
                )
                    .into_response();
            }
        };

        // 创建管理层用户（目前所有通过邀请码注册的都是管理层）
        User::new_management(
            req.username.clone(),
            req.name.clone(),
            password_hash,
            2 + management_users.len() as u32, // 管理层工号从00002开始
            req.email.clone(),
        )
    };

    // 保存用户到数据库
    if let Err(e) = state.store.save_user(&user_to_create).await {
        error!("Failed to save user: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to register user".to_string(),
            }),
        )
            .into_response();
    }

    // If it's corporate chairman or management, add user to Cliff of Contemplation Line
    if matches!(
        user_to_create.position,
        crate::domain::user::Position::Chairman | crate::domain::user::Position::Management
    ) {
        // Create or update organization structure, add user to Cliff of Contemplation Line department
        let mut org = state
            .store
            .load_organization()
            .await
            .unwrap_or_else(|_| Organization::new());

        // Ensure Cliff of Contemplation Line department exists
        let guilty_cliff_dept_id = "guilty-cliff-line";
        let guilty_cliff_dept_name = "Cliff of Contemplation Line";

        // Check if department already exists
        let dept_exists = org.departments.iter().any(|d| d.id == guilty_cliff_dept_id);
        if !dept_exists {
            // Create Cliff of Contemplation Line department
            let dept = crate::domain::Department {
                id: guilty_cliff_dept_id.to_string(),
                name: guilty_cliff_dept_name.to_string(),
                parent_id: None, // Cliff of Contemplation Line is the top-level department
                leader_id: if matches!(
                    user_to_create.position,
                    crate::domain::user::Position::Chairman
                ) {
                    Some(user_to_create.id.clone())
                } else {
                    None
                },
            };
            org.add_department(dept);
        }

        // 检查是否已有对应的Agent，如果没有则创建
        let agent_exists = org.agents.iter().any(|a| a.id == user_to_create.id);

        if !agent_exists {
            // If Agent doesn't exist, create a corresponding Agent and add to Cliff of Contemplation Line
            let new_agent = Agent {
                id: user_to_create.id.clone(),
                name: user_to_create.name.clone(),
                role: if matches!(
                    user_to_create.position,
                    crate::domain::user::Position::Chairman
                ) {
                    Role::simple("Cliff of Contemplation Line Supervisor".to_string(), "You are the supervisor of the Cliff of Contemplation Line, responsible for overseeing and managing senior company affairs.".to_string())
                        .with_responsibilities(vec!["Corporate Chairman".to_string(), "Cliff of Contemplation Line Management".to_string()])
                        .with_expertise(vec!["Corporate Governance".to_string(), "Strategic Planning".to_string()])
                } else {
                    Role::simple("Cliff of Contemplation Line Member".to_string(), "You are a member of the Cliff of Contemplation Line, participating in senior management and decision-making processes.".to_string())
                        .with_responsibilities(vec!["Management Affairs".to_string(), "Collaborative Work".to_string()])
                        .with_expertise(vec!["Team Management".to_string(), "Cross-department Collaboration".to_string()])
                },
                department_id: Some(guilty_cliff_dept_id.to_string()),
                llm_config: LLMConfig::openai("fake-api-key".to_string()),
                watched_tools: vec![],
                trigger_conditions: vec![],
            };
            org.agents.push(new_agent);
        } else {
            // If Agent already exists, update its department information
            if let Some(agent) = org.agents.iter_mut().find(|a| a.id == user_to_create.id) {
                agent.department_id = Some(guilty_cliff_dept_id.to_string());
                if matches!(
                    user_to_create.position,
                    crate::domain::user::Position::Chairman
                ) {
                    // Corporate chairman becomes Cliff of Contemplation Line supervisor
                    agent.role = Role::simple("Cliff of Contemplation Line Supervisor".to_string(), "You are the supervisor of the Cliff of Contemplation Line, responsible for overseeing and managing senior company affairs.".to_string())
                        .with_responsibilities(vec!["Corporate Chairman".to_string(), "Cliff of Contemplation Line Management".to_string()])
                        .with_expertise(vec!["Corporate Governance".to_string(), "Strategic Planning".to_string()]);

                    // Also update department leader
                    if let Some(dept) = org
                        .departments
                        .iter_mut()
                        .find(|d| d.id == guilty_cliff_dept_id)
                    {
                        dept.leader_id = Some(user_to_create.id.clone());
                    }
                } else {
                    // Management member
                    agent.role = Role::simple("Cliff of Contemplation Line Member".to_string(), "You are a member of the Cliff of Contemplation Line, participating in senior management and decision-making processes.".to_string())
                        .with_responsibilities(vec!["Management Affairs".to_string(), "Collaborative Work".to_string()])
                        .with_expertise(vec!["Team Management".to_string(), "Cross-department Collaboration".to_string()]);
                }
            }
        }

        // 保存更新后的组织架构
        if let Err(e) = state.store.save_organization(&org).await {
            error!("Failed to update organization for guilty cliff line: {}", e);
        }

        // Also update user's department information
        let mut updated_user = user_to_create.clone();
        updated_user.department = guilty_cliff_dept_name.to_string(); // Set user department to "Cliff of Contemplation Line"

        // Save updated user information
        if let Err(e) = state.store.save_user(&updated_user).await {
            error!(
                "Failed to update user department for guilty cliff line: {}",
                e
            );
        }
    }

    // Use updated user information (if there was an update)
    let final_user = if matches!(
        user_to_create.position,
        crate::domain::user::Position::Chairman | crate::domain::user::Position::Management
    ) {
        // If it's corporate chairman or management, use updated user information
        let updated_user = match state
            .store
            .load_user_by_username(&user_to_create.username)
            .await
        {
            Ok(Some(user)) => user,
            _ => user_to_create.clone(), // 如果加载失败，使用原始用户
        };
        updated_user
    } else {
        user_to_create.clone()
    };

    // 生成JWT令牌
    let token: String = match state.jwt_service.generate_token(&UserInfo {
        id: final_user.id.clone(),
        username: final_user.username.clone(),
        name: final_user.name.clone(),
        email: final_user.email.clone(),
        is_director: matches!(
            final_user.position,
            crate::domain::user::Position::Chairman | crate::domain::user::Position::Management
        ),
        employee_id: final_user.employee_id.clone(),
        position: format!("{:?}", final_user.position),
        department: final_user.department.clone(),
    }) {
        Ok(token) => token,
        Err(e) => {
            error!("Failed to generate token: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to generate token".to_string(),
                }),
            )
                .into_response();
        }
    };

    Json(serde_json::json!({
        "success": true,
        "data": {
            "token": token,
            "user": {
                "id": final_user.id,
                "username": final_user.username,
                "name": final_user.name,
                "email": final_user.email,
                "is_director": matches!(final_user.position, crate::domain::user::Position::Chairman | crate::domain::user::Position::Management),
                "employee_id": final_user.employee_id,
                "position": format!("{:?}", final_user.position),
                "department": final_user.department,
            }
        }
    })).into_response()
}

/// 检查用户名
async fn check_username(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
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

async fn handle_websocket(mut socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
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

// ==================== 管理API ====================

#[derive(Deserialize)]
pub struct CreateInviteCodeRequest {
    pub max_usage: Option<u32>,
    pub expires_at: Option<String>, // ISO 8601 format
}

/// 检查用户是否具有管理员权限
async fn check_admin_permission(state: &AppState, token: &str) -> Option<UserInfo> {
    match state.jwt_service.validate_token(token) {
        Ok(user_info) => {
            // 检查是否为董事长或管理层（这里暂时认为所有管理层及以上都是管理员）
            if matches!(user_info.position.as_str(), "Chairman" | "Management") {
                Some(user_info)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// 获取所有邀请码（仅管理员）
async fn get_invite_codes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());

    if let Some(auth_str) = auth_header {
        if auth_str.starts_with("Bearer ") {
            let token = auth_str.strip_prefix("Bearer ").unwrap();

            if check_admin_permission(&state, token).await.is_some() {
                match state.store.load_invitation_codes().await {
                    Ok(codes) => {
                        // 转换为前端友好的格式
                        let codes_data: Vec<serde_json::Value> = codes
                            .into_iter()
                            .map(|code| {
                                serde_json::json!({
                                    "id": code.id,
                                    "code": code.code,
                                    "created_by": code.created_by,
                                    "created_at": code.created_at,
                                    "expires_at": code.expiry_time,
                                    "usage_count": code.current_usage,
                                    "max_usage": code.max_usage,
                                    "is_active": code.is_valid(),
                                })
                            })
                            .collect();

                        return Json(serde_json::json!({
                            "success": true,
                            "data": codes_data
                        }))
                        .into_response();
                    }
                    Err(e) => {
                        error!("Failed to load invitation codes: {}", e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                error: "Failed to load invitation codes".to_string(),
                            }),
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "Insufficient permissions".to_string(),
        }),
    )
        .into_response()
}

/// 创建邀请码（仅管理员）
async fn create_invite_code(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateInviteCodeRequest>,
) -> impl IntoResponse {
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());

    if let Some(auth_str) = auth_header {
        if auth_str.starts_with("Bearer ") {
            let token = auth_str.strip_prefix("Bearer ").unwrap();

            if let Some(user_info) = check_admin_permission(&state, token).await {
                // 创建邀请码
                let mut new_code = InvitationCode::new(user_info.id.clone(), req.max_usage);

                // 如果指定了过期时间，使用指定的时间，否则使用默认的1天
                if let Some(expires_at_str) = req.expires_at {
                    if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(&expires_at_str) {
                        new_code.expiry_time = expires_at.timestamp();
                    }
                }

                match state.store.save_invitation_code(&new_code).await {
                    Ok(_) => {
                        return Json(serde_json::json!({
                            "success": true,
                            "data": {
                                "id": new_code.id,
                                "code": new_code.code,
                                "created_by": new_code.created_by,
                                "created_at": new_code.created_at,
                                "expires_at": new_code.expiry_time,
                                "max_usage": new_code.max_usage,
                                "usage_count": new_code.current_usage,
                                "is_active": new_code.is_valid(),
                            }
                        }))
                        .into_response();
                    }
                    Err(e) => {
                        error!("Failed to save invitation code: {}", e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                error: "Failed to save invitation code".to_string(),
                            }),
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "Insufficient permissions".to_string(),
        }),
    )
        .into_response()
}

/// 删除邀请码（仅管理员）
async fn delete_invite_code(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(code_id): Path<String>,
) -> impl IntoResponse {
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());

    if let Some(auth_str) = auth_header {
        if auth_str.starts_with("Bearer ") {
            let token = auth_str.strip_prefix("Bearer ").unwrap();

            if check_admin_permission(&state, token).await.is_some() {
                // 由于我们的存储接口没有直接的删除方法，我们需要先加载所有邀请码，找到要删除的，然后不保存它
                match state.store.load_invitation_codes().await {
                    Ok(mut codes) => {
                        // 查找要删除的邀请码
                        if let Some(pos) = codes.iter().position(|c| c.id == code_id) {
                            // 从数据库中移除（通过重新保存其他码）
                            codes.remove(pos);

                            // 重新保存剩余的邀请码
                            for code in codes {
                                if let Err(e) = state.store.save_invitation_code(&code).await {
                                    error!(
                                        "Failed to update invitation codes after deletion: {}",
                                        e
                                    );
                                    return (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(ErrorResponse {
                                            error: "Failed to update invitation codes".to_string(),
                                        }),
                                    )
                                        .into_response();
                                }
                            }

                            return Json(serde_json::json!({
                                "success": true,
                                "message": "Invitation code deleted successfully"
                            }))
                            .into_response();
                        } else {
                            return (
                                StatusCode::NOT_FOUND,
                                Json(ErrorResponse {
                                    error: "Invitation code not found".to_string(),
                                }),
                            )
                                .into_response();
                        }
                    }
                    Err(e) => {
                        error!("Failed to load invitation codes: {}", e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                error: "Failed to load invitation codes".to_string(),
                            }),
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "Insufficient permissions".to_string(),
        }),
    )
        .into_response()
}

/// 获取聊天会话列表
async fn list_chat_sessions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // 从组织架构中获取Agent信息来构建会话列表
    match state.store.load_organization().await {
        Ok(org) => {
            let sessions: Vec<serde_json::Value> = org
                .agents
                .into_iter()
                .map(|agent| {
                    serde_json::json!({
                        "id": agent.id,
                        "name": agent.name,
                        "participants": [{
                            "id": agent.id,
                            "name": agent.name,
                            "isAgent": true,
                            "status": "online"  // 假设Agent始终在线
                        }],
                        "lastMessage": null,
                        "unreadCount": 0,
                        "createdAt": chrono::Utc::now().timestamp(),
                        "updatedAt": chrono::Utc::now().timestamp()
                    })
                })
                .collect();

            Json(serde_json::json!({
                "success": true,
                "data": sessions
            }))
            .into_response()
        }
        Err(e) => {
            error!("Failed to load organization for chat sessions: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to load chat sessions".to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// 获取特定会话的消息
async fn get_session_messages(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    // 获取该Agent相关的所有消息
    let filter = crate::core::store::MessageFilter {
        from: Some(session_id.clone()), // 消息来自该Agent
        target_type: None,
        to: None,
        since: None,
        limit: 50, // 限制返回50条消息
    };

    match state.store.load_messages(filter).await {
        Ok(messages) => {
            // 转换消息格式以匹配前端期望
            let formatted_messages: Vec<serde_json::Value> = messages
                .into_iter()
                .map(|msg| {
                    // 获取发送者信息
                    let sender = if msg.from == session_id {
                        // 如果是Agent发送的
                        serde_json::json!({
                            "id": msg.from,
                            "name": get_agent_name_by_id(&state.agents, &msg.from),
                            "isAgent": true
                        })
                    } else {
                        // 如果是用户发送的
                        serde_json::json!({
                            "id": msg.from,
                            "name": "Unknown User",
                            "isAgent": false
                        })
                    };

                    serde_json::json!({
                        "id": msg.id,
                        "sender": sender,
                        "content": msg.content,
                        "timestamp": msg.timestamp,
                        "replyTo": msg.reply_to,
                        "mentions": msg.mentions
                    })
                })
                .collect();

            Json(serde_json::json!({
                "success": true,
                "data": formatted_messages
            }))
            .into_response()
        }
        Err(e) => {
            error!("Failed to load messages for session {}: {}", session_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to load messages".to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// 根据ID获取Agent名称的辅助函数
fn get_agent_name_by_id(agents: &[Agent], agent_id: &str) -> String {
    agents
        .iter()
        .find(|agent| agent.id == agent_id)
        .map(|agent| agent.name.clone())
        .unwrap_or_else(|| "Unknown Agent".to_string())
}

/// 获取组织架构树
async fn get_org_tree(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.store.load_organization().await {
        Ok(org) => {
            // First, convert flat department list to tree structure
            let mut departments_map: std::collections::HashMap<String, serde_json::Value> =
                std::collections::HashMap::new();

            // Step 1: Create basic structure for all departments
            for dept in &org.departments {
                let agent_leaders: Vec<serde_json::Value> = org
                    .agents
                    .iter()
                    .filter(|agent| {
                        agent.department_id.as_ref() == Some(&dept.id)
                            && agent.role.title.contains("主管")
                    })
                    .map(|agent| {
                        serde_json::json!({
                            "id": agent.id,
                            "name": agent.name,
                            "title": agent.role.title,
                            "status": "online"
                        })
                    })
                    .collect();

                let dept_agents: Vec<serde_json::Value> = org
                    .agents
                    .iter()
                    .filter(|agent| {
                        agent.department_id.as_ref() == Some(&dept.id)
                            && !agent.role.title.contains("主管")
                    })
                    .map(|agent| {
                        serde_json::json!({
                            "id": agent.id,
                            "name": agent.name,
                            "title": agent.role.title,
                            "status": "online"
                        })
                    })
                    .collect();

                let leader = if !agent_leaders.is_empty() {
                    agent_leaders.first().cloned()
                } else {
                    None
                };

                let users = [&agent_leaders[..], &dept_agents[..]].concat();

                departments_map.insert(
                    dept.id.clone(),
                    serde_json::json!({
                        "id": dept.id,
                        "name": dept.name,
                        "parentId": dept.parent_id,
                        "leader": leader,
                        "users": users,
                        "memberCount": users.len(),
                        "children": Vec::<serde_json::Value>::new() // 初始化为空数组
                    }),
                );
            }

            // Step 2: Build parent-child relationships (add child departments to parent department's children array)
            let mut processed_depts = std::collections::HashMap::new();
            for dept in &org.departments {
                if let Some(mut dept_info) = departments_map.get(&dept.id).cloned() {
                    // Find all child departments with this department as parent
                    let child_depts: Vec<serde_json::Value> = org
                        .departments
                        .iter()
                        .filter(|child| child.parent_id.as_ref() == Some(&dept.id))
                        .filter_map(|child| processed_depts.get(&child.id).cloned())
                        .collect();

                    // Update department information, add child departments
                    if !child_depts.is_empty() {
                        if let serde_json::Value::Object(ref mut obj) = dept_info {
                            obj.insert(
                                "children".to_string(),
                                serde_json::Value::Array(child_depts),
                            );
                        }
                    }

                    processed_depts.insert(dept.id.clone(), dept_info);
                }
            }

            // Finally get root departments (departments without parent)
            let root_depts: Vec<serde_json::Value> = org
                .departments
                .iter()
                .filter(|dept| dept.parent_id.is_none())
                .filter_map(|dept| processed_depts.get(&dept.id).cloned())
                .collect();

            // 同时返回扁平的agents列表用于其他用途（目前未使用）
            let _agents: Vec<serde_json::Value> = org
                .agents
                .iter()
                .map(|agent| {
                    serde_json::json!({
                        "id": agent.id,
                        "name": agent.name,
                        "title": agent.role.title,
                        "departmentId": agent.department_id,
                        "status": "online",  // 假设Agent始终在线
                        "isOnline": true
                    })
                })
                .collect();

            Json(serde_json::json!({
                "success": true,
                "data": root_depts  // Directly return department array, matching frontend expectation
            }))
            .into_response()
        }
        Err(e) => {
            error!("Failed to load organization tree: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to load organization tree".to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// 获取所有用户（仅管理员）
async fn get_users(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());

    if let Some(auth_str) = auth_header {
        if auth_str.starts_with("Bearer ") {
            let token = auth_str.strip_prefix("Bearer ").unwrap();

            if check_admin_permission(&state, token).await.is_some() {
                match state.store.load_users().await {
                    Ok(users) => {
                        // 转换为前端友好的格式
                        let users_data: Vec<serde_json::Value> = users.into_iter().map(|user| {
                            serde_json::json!({
                                "id": user.id,
                                "username": user.username,
                                "name": user.name,
                                "email": user.email,
                                "employee_id": user.employee_id,
                                "position": format!("{:?}", user.position),
                                "department": user.department,
                                "created_at": user.created_at,
                                "is_director": matches!(user.position, crate::domain::user::Position::Chairman | crate::domain::user::Position::Management),
                            })
                        }).collect();

                        return Json(serde_json::json!({
                            "success": true,
                            "data": users_data
                        }))
                        .into_response();
                    }
                    Err(e) => {
                        error!("Failed to load users: {}", e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                error: "Failed to load users".to_string(),
                            }),
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "Insufficient permissions".to_string(),
        }),
    )
        .into_response()
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
        .route(
            "/api/admin/invite-codes",
            get(get_invite_codes).post(create_invite_code),
        )
        .route("/api/admin/invite-codes/{id}", delete(delete_invite_code))
        .route("/api/chat/list", get(list_chat_sessions))
        .route("/api/chat/{session_id}/messages", get(get_session_messages))
        .route("/api/org/tree", get(get_org_tree))
        .route("/api/admin/users", get(get_users))
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
    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret_key_for_dev".to_string());
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
