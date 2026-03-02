//! MCP 服务器
//!
//! 基于 WebSocket、HTTP、SSE 等协议的 MCP 服务端实现

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info};

use crate::core::capability::CapabilityRegistry;
use crate::infrastructure::capability::McpProtocolHandler;

#[derive(Clone)]
pub struct McpServerState {
    #[allow(dead_code)]
    pub capability_registry: Arc<CapabilityRegistry>,
    pub protocol_handler: Arc<McpProtocolHandler>,
    pub client_broadcast: broadcast::Sender<String>,
}

pub struct McpServer {
    bind_addr: String,
    state: Arc<McpServerState>,
}

impl McpServer {
    pub fn new(bind_addr: String, capability_registry: Arc<CapabilityRegistry>) -> Self {
        let protocol_handler = Arc::new(McpProtocolHandler::new(capability_registry.clone()));
        let (client_broadcast, _) = broadcast::channel(100);

        let state = Arc::new(McpServerState {
            capability_registry,
            protocol_handler,
            client_broadcast,
        });

        Self { bind_addr, state }
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting MCP server on {}", self.bind_addr);

        let cors = CorsLayer::permissive();
        let app = Router::new()
            // HTTP endpoints
            .route("/mcp/capabilities/list", get(list_capabilities))
            .route("/mcp/capabilities/discover", post(discover_capabilities))
            .route("/mcp/call", post(call_capability))
            .route("/mcp/ping", get(ping))
            .route("/mcp/protocol/info", get(protocol_info))
            // WebSocket endpoint
            .route("/mcp/ws", get(websocket_handler))
            .layer(cors)
            .with_state(self.state.clone());

        let listener = tokio::net::TcpListener::bind(&self.bind_addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

// ==================== HTTP Handlers ====================

async fn list_capabilities(
    State(state): State<Arc<McpServerState>>,
) -> Result<impl IntoResponse, McpServerError> {
    let result = state
        .protocol_handler
        .handle_request("capabilities/list", serde_json::json!({}))
        .await
        .map_err(|e| McpServerError::Internal(e.to_string()))?;

    Ok(Json(result))
}

async fn discover_capabilities(
    State(state): State<Arc<McpServerState>>,
    Json(params): Json<Value>,
) -> Result<impl IntoResponse, McpServerError> {
    let result = state
        .protocol_handler
        .handle_request("capabilities/discover", params)
        .await
        .map_err(|e| McpServerError::Internal(e.to_string()))?;

    Ok(Json(result))
}

async fn call_capability(
    State(state): State<Arc<McpServerState>>,
    Json(params): Json<Value>,
) -> Result<impl IntoResponse, McpServerError> {
    let result = state
        .protocol_handler
        .handle_request("capabilities/call", params)
        .await
        .map_err(|e| McpServerError::Internal(e.to_string()))?;

    Ok(Json(result))
}

async fn ping(
    State(state): State<Arc<McpServerState>>,
) -> Result<impl IntoResponse, McpServerError> {
    let result = state
        .protocol_handler
        .handle_request("ping", serde_json::json!({}))
        .await
        .map_err(|e| McpServerError::Internal(e.to_string()))?;

    Ok(Json(result))
}

async fn protocol_info(State(state): State<Arc<McpServerState>>) -> impl IntoResponse {
    let info = state.protocol_handler.get_protocol_info();
    Json(info)
}

// ==================== WebSocket Handler ====================

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<McpServerState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: Arc<McpServerState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 创建一个接收来自其他客户端广播的消息的任务
    let mut broadcast_rx = state.client_broadcast.subscribe();
    let _client_broadcast = state.client_broadcast.clone();

    // 处理 WebSocket 消息的循环
    loop {
        tokio::select! {
            // 接收来自客户端的消息
            msg_option = ws_receiver.next() => {
                match msg_option {
                    Some(Ok(message)) => {
                        if let Err(e) = handle_client_message(message, &mut ws_sender, &state).await {
                            error!("Error handling client message: {}", e);
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        error!("WebSocket receive error: {}", e);
                        break;
                    }
                    None => {
                        info!("Client disconnected");
                        break;
                    }
                }
            }
            // 接收来自广播的消息
            broadcast_msg = broadcast_rx.recv() => {
                match broadcast_msg {
                    Ok(msg) => {
                        if let Err(e) = ws_sender.send(Message::Text(msg.into())).await {
                            error!("WebSocket send error: {}", e);
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                    Err(_) => {
                        // 继续等待下一个广播消息
                        continue;
                    }
                }
            }
        }
    }
}

async fn handle_client_message(
    message: Message,
    ws_sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    state: &McpServerState,
) -> Result<()> {
    match message {
        Message::Text(text) => {
            debug!("Received WebSocket text message: {}", text);

            // 解析 MCP 协议消息
            let parsed: Value =
                serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

            // 提取方法和参数
            let method = parsed
                .get("method")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Method is required"))?;

            let params = parsed.get("params").cloned().unwrap_or_default();

            // 处理请求
            match state.protocol_handler.handle_request(method, params).await {
                Ok(response) => {
                    let response_msg = serde_json::json!({
                        "id": parsed.get("id"), // 回显请求ID
                        "result": response,
                    });

                    let response_text = response_msg.to_string();
                    ws_sender.send(Message::Text(response_text.into())).await?;
                }
                Err(e) => {
                    let error_msg = serde_json::json!({
                        "id": parsed.get("id"),
                        "error": {
                            "message": e.to_string(),
                            "code": -32603, // Internal error code
                        },
                    });

                    let error_text = error_msg.to_string();
                    ws_sender.send(Message::Text(error_text.into())).await?;
                }
            }
        }
        Message::Binary(_) => {
            // MCP 主要使用文本协议，二进制消息不处理
            let error_msg = serde_json::json!({
                "error": {
                    "message": "Binary messages not supported",
                    "code": -32602, // Invalid params
                },
            });

            ws_sender
                .send(Message::Text(error_msg.to_string().into()))
                .await?;
        }
        Message::Ping(ping) => {
            ws_sender.send(Message::Pong(ping)).await?;
        }
        Message::Pong(_) => {
            // 忽略 Pong 消息
        }
        Message::Close(_) => {
            info!("WebSocket connection closed by client");
            return Ok(());
        }
    }

    Ok(())
}

// ==================== Error Types ====================

#[derive(Debug)]
#[allow(dead_code)]
pub enum McpServerError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl IntoResponse for McpServerError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            McpServerError::BadRequest(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
            McpServerError::NotFound(msg) => (axum::http::StatusCode::NOT_FOUND, msg),
            McpServerError::Internal(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = serde_json::json!({
            "error": {
                "message": error_message,
                "status": status.as_u16(),
            }
        });

        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for McpServerError {
    fn from(err: anyhow::Error) -> Self {
        McpServerError::Internal(err.to_string())
    }
}

// ==================== Client Implementation ====================

pub struct McpClient {
    server_url: String,
    client: reqwest::Client,
}

impl McpClient {
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            client: reqwest::Client::new(),
        }
    }

    /// 通过 HTTP 调用 MCP 功能
    pub async fn call_capability_http(&self, method: &str, params: Value) -> Result<Value> {
        let url = format!("{}/mcp/call", self.server_url);

        let payload = serde_json::json!({
            "method": method,
            "params": params
        });

        let response = self.client.post(&url).json(&payload).send().await?;

        let result: Value = response.json().await?;
        Ok(result)
    }

    /// 列出服务器上的功能
    pub async fn list_capabilities(&self) -> Result<Value> {
        let url = format!("{}/mcp/capabilities/list", self.server_url);

        let response = self.client.get(&url).send().await?;
        let result: Value = response.json().await?;
        Ok(result)
    }

    /// 发现特定功能
    pub async fn discover_capabilities(&self, requested: Option<Vec<String>>) -> Result<Value> {
        let url = format!("{}/mcp/capabilities/discover", self.server_url);

        let payload = match requested {
            Some(req) => serde_json::json!({ "requested": req }),
            None => serde_json::json!({}),
        };

        let response = self.client.post(&url).json(&payload).send().await?;
        let result: Value = response.json().await?;
        Ok(result)
    }
}
