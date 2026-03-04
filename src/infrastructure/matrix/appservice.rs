//! Matrix Appservice 服务器
//!
//! 监听 Homeserver 推送的事件并路由到 MessageBus

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use super::config::MatrixConfig;
use crate::domain::Message;

/// Appservice 状态
#[derive(Clone)]
pub struct AppServiceState {
    pub config: MatrixConfig,
    pub message_tx: broadcast::Sender<Message>,
}

/// Appservice 事务请求
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Transaction {
    pub txn_id: String,
    pub events: Vec<TransactionEvent>,
    pub ephemeral: Vec<Value>,
    pub device_lists: DeviceLists,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub content: Value,
    pub room_id: Option<String>,
    pub sender: Option<String>,
    pub event_id: Option<String>,
    pub origin_server_ts: Option<u64>,
    pub state_key: Option<String>,
    pub unsigned: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DeviceLists {
    pub changed: Vec<String>,
    pub left: Vec<String>,
}

/// Appservice 服务
pub struct AppService {
    state: Arc<AppServiceState>,
}

impl AppService {
    /// 创建新的 Appservice
    pub fn new(config: MatrixConfig, message_tx: broadcast::Sender<Message>) -> Self {
        let state = Arc::new(AppServiceState {
            config,
            message_tx,
        });

        Self { state }
    }

    /// 运行 Appservice 服务器
    pub async fn run(self) -> Result<()> {
        let app = self.create_router();
        let addr = format!("0.0.0.0:{}", self.state.config.appservice_port);

        info!("Starting Matrix Appservice on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    /// 创建 Axum 路由
    fn create_router(&self) -> Router {
        Router::new()
            // Appservice API
            .route(
                "/_matrix/app/v1/transactions/:txn_id",
                put(handle_transaction),
            )
            .route(
                "/_matrix/app/v1/users/:user_id",
                get(handle_user_query),
            )
            .route(
                "/_matrix/app/v1/rooms/:room_alias",
                get(handle_room_query),
            )
            // 健康检查
            .route("/health", get(health_check))
            .with_state(self.state.clone())
    }
}

/// 处理事务推送 (PUT /_matrix/app/v1/transactions/{txn_id})
async fn handle_transaction(
    State(state): State<Arc<AppServiceState>>,
    Path(txn_id): Path<String>,
    Json(transaction): Json<Transaction>,
) -> impl IntoResponse {
    debug!("Received transaction: {}", txn_id);

    let mut processed = 0;
    let mut errors = 0;

    for event in &transaction.events {
        match process_event(&state, event).await {
            Ok(_) => processed += 1,
            Err(e) => {
                error!("Failed to process event: {}", e);
                errors += 1;
            }
        }
    }

    debug!(
        "Transaction {} processed: {} events, {} errors",
        txn_id, processed, errors
    );

    // 返回空响应表示成功
    StatusCode::OK
}

/// 处理单个事件
async fn process_event(state: &Arc<AppServiceState>, event: &TransactionEvent) -> Result<()> {
    // 只处理房间消息事件
    if event.event_type != "m.room.message" {
        return Ok(());
    }

    // 获取消息内容
    let content = &event.content;
    let msgtype = content["msgtype"].as_str().unwrap_or("m.text");
    let body = content["body"].as_str().unwrap_or("");

    // 忽略打字通知等非消息内容
    if msgtype != "m.text" && msgtype != "m.notice" {
        return Ok(());
    }

    // 获取房间 ID
    let room_id = event.room_id.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Missing room_id in event")
    })?;

    // 获取发送者
    let sender = event.sender.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Missing sender in event")
    })?;

    // 忽略来自虚拟用户的消息（避免循环）
    if sender.starts_with("@_") && sender.contains(&state.config.server_name) {
        debug!("Ignoring message from virtual user: {}", sender);
        return Ok(());
    }

    // 创建内部消息
    // 注意：这里需要将 Matrix 用户 ID 映射到内部用户 ID
    // 简化处理：直接使用 Matrix 用户 ID 作为 from
    let message = Message::group(
        sender.clone(),           // from: 使用 Matrix 用户 ID
        room_id.clone(),          // to: 使用 Matrix 房间 ID
        body.to_string(),
    );

    // 发送到 MessageBus
    if let Err(e) = state.message_tx.send(message) {
        warn!("Failed to send message to MessageBus: {}", e);
        return Err(anyhow::anyhow!("Failed to send to MessageBus: {}", e));
    }

    debug!(
        "Message from {} in {}: {}",
        sender, room_id, body
    );

    Ok(())
}

/// 处理用户查询 (GET /_matrix/app/v1/users/{user_id})
///
/// Homeserver 查询 Appservice 是否管理某个用户
async fn handle_user_query(
    State(state): State<Arc<AppServiceState>>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    debug!("User query: {}", user_id);

    // 检查用户是否匹配我们的命名空间
    // 格式：@_xxx:server_name
    let expected_prefix = format!("@_");
    let expected_suffix = format!(":{}", state.config.server_name);

    if user_id.starts_with(&expected_prefix) && user_id.ends_with(&expected_suffix) {
        debug!("Appservice manages user: {}", user_id);
        Json(serde_json::json!({
            "user_id": user_id,
            "managed": true
        }))
        .into_response()
    } else {
        // 返回 404 表示不管理此用户
        StatusCode::NOT_FOUND.into_response()
    }
}

/// 处理房间查询 (GET /_matrix/app/v1/rooms/{room_alias})
///
/// Homeserver 查询 Appservice 是否管理某个房间别名
async fn handle_room_query(
    State(state): State<Arc<AppServiceState>>,
    Path(room_alias): Path<String>,
) -> impl IntoResponse {
    debug!("Room query: {}", room_alias);

    // 检查房间别名是否匹配我们的命名空间
    // 格式：#company_xxx:server_name
    let expected_prefix = format!("#company_");
    let expected_suffix = format!(":{}", state.config.server_name);

    if room_alias.starts_with(&expected_prefix) && room_alias.ends_with(&expected_suffix) {
        debug!("Appservice manages room: {}", room_alias);
        Json(serde_json::json!({
            "room_alias": room_alias,
            "managed": true
        }))
        .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

/// 健康检查
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "matrix-appservice"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_namespace_match() {
        let config = MatrixConfig {
            homeserver_url: "http://localhost:5141".to_string(),
            server_name: "localhost".to_string(),
            as_token: "test".to_string(),
            hs_token: "test".to_string(),
            sender_localpart: "_bot".to_string(),
            appservice_port: 9000,
        };

        let state = Arc::new(AppServiceState {
            config,
            message_tx: broadcast::channel(100).0,
        });

        // 应该匹配
        assert!(state.config.generate_user_id("ceo").starts_with("@_"));
        assert!(state.config.generate_user_id("ceo").ends_with(":localhost"));
    }
}
