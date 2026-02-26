//! MCP 客户端
//!
//! 连接到外部 MCP 服务器的客户端实现

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct McpHttpClient {
    base_url: String,
    client: reqwest::Client,
}

impl McpHttpClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    /// 通过 HTTP 调用 MCP 功能
    pub async fn call_capability(&self, method: &str, params: Value) -> Result<Value> {
        let url = format!("{}/mcp/call", self.base_url);

        let payload = serde_json::json!({
            "method": method,
            "params": params
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        let result: Value = response.json().await?;
        Ok(result)
    }

    /// 列出服务器上的功能
    pub async fn list_capabilities(&self) -> Result<Value> {
        let url = format!("{}/mcp/capabilities/list", self.base_url);

        let response = self.client.get(&url).send().await?;
        let result: Value = response.json().await?;
        Ok(result)
    }

    /// 发现特定功能
    pub async fn discover_capabilities(&self, requested: Option<Vec<String>>) -> Result<Value> {
        let url = format!("{}/mcp/capabilities/discover", self.base_url);

        let payload = match requested {
            Some(req) => serde_json::json!({ "requested": req }),
            None => serde_json::json!({}),
        };

        let response = self.client.post(&url).json(&payload).send().await?;
        let result: Value = response.json().await?;
        Ok(result)
    }

    /// Ping 服务器
    pub async fn ping(&self) -> Result<Value> {
        let url = format!("{}/mcp/ping", self.base_url);

        let response = self.client.get(&url).send().await?;
        let result: Value = response.json().await?;
        Ok(result)
    }
}

pub struct McpWebSocketClient {
    ws_url: String,
}

impl McpWebSocketClient {
    pub fn new(ws_url: String) -> Self {
        Self { ws_url }
    }

    /// 通过 WebSocket 调用 MCP 功能
    pub async fn call_capability(&self, method: &str, params: Value) -> Result<Value> {
        let (mut ws_stream, _) = connect_async(&self.ws_url).await?;

        let request_id = uuid::Uuid::new_v4().to_string();
        let request = serde_json::json!({
            "id": request_id,
            "method": method,
            "params": params
        });

        // 发送请求
        ws_stream.send(Message::Text(request.to_string())).await?;

        // 等待响应
        if let Some(msg) = ws_stream.next().await {
            match msg? {
                Message::Text(text) => {
                    let response: Value = serde_json::from_str(&text)?;

                    // 检查是否有错误
                    if let Some(error_obj) = response.get("error") {
                        return Err(anyhow::anyhow!("MCP Error: {:?}", error_obj));
                    }

                    if let Some(result) = response.get("result") {
                        return Ok(result.clone());
                    } else {
                        return Ok(response);
                    }
                }
                Message::Close(_) => {
                    return Err(anyhow::anyhow!("Connection closed by server"));
                }
                _ => {
                    return Err(anyhow::anyhow!("Unexpected message type"));
                }
            }
        } else {
            return Err(anyhow::anyhow!("No response received"));
        }
    }

    /// 建立 WebSocket 连接并监听通知
    pub async fn connect_and_listen<F>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(Value) -> Result<()> + Send + 'static,
    {
        let (mut ws_stream, _) = connect_async(&self.ws_url).await?;

        // 启动一个任务来持续监听消息
        while let Some(msg) = ws_stream.next().await {
            match msg? {
                Message::Text(text) => {
                    let value: Value = serde_json::from_str(&text)?;
                    handler(value)?;
                }
                Message::Close(frame) => {
                    if let Some(close_frame) = frame {
                        println!("Connection closed: {}", close_frame.reason);
                    } else {
                        println!("Connection closed by server");
                    }
                    break;
                }
                _ => {
                    // 忽略其他类型的消息
                }
            }
        }

        Ok(())
    }
}

pub struct McpSseClient {
    sse_url: String,
    client: reqwest::Client,
}

impl McpSseClient {
    pub fn new(sse_url: String) -> Self {
        Self {
            sse_url,
            client: reqwest::Client::new(),
        }
    }

    /// 通过 SSE 订阅 MCP 事件
    pub async fn subscribe_events<F>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(Value) -> Result<()> + Send + 'static,
    {
        use eventsource_stream::Eventsource;

        let response = self.client.get(&self.sse_url).send().await?;
        let stream = response.bytes_stream().eventsource();

        tokio::pin!(stream);

        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    let data: Value = serde_json::from_str(&event.data)?;
                    handler(data)?;
                }
                Err(e) => {
                    eprintln!("SSE Error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

pub struct McpStdioClient;

impl McpStdioClient {
    pub fn new() -> Self {
        Self
    }

    /// 通过标准输入输出与 MCP 服务器通信
    pub fn call_capability_sync(&self, method: &str, params: Value) -> Result<Value> {
        // 发送请求到 stdout
        let request = serde_json::json!({
            "method": method,
            "params": params
        });

        println!("{}", request.to_string());

        // 从 stdin 读取响应
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let response: Value = serde_json::from_str(&input.trim())?;

        if let Some(error_obj) = response.get("error") {
            return Err(anyhow::anyhow!("MCP Error: {:?}", error_obj));
        }

        if let Some(result) = response.get("result") {
            Ok(result.clone())
        } else {
            Ok(response)
        }
    }
}

pub enum McpTransport {
    Http(McpHttpClient),
    WebSocket(McpWebSocketClient),
    Sse(McpSseClient),
    Stdio(McpStdioClient),
}

impl McpTransport {
    pub fn new_http(base_url: String) -> Self {
        McpTransport::Http(McpHttpClient::new(base_url))
    }

    pub fn new_websocket(ws_url: String) -> Self {
        McpTransport::WebSocket(McpWebSocketClient::new(ws_url))
    }

    pub fn new_sse(sse_url: String) -> Self {
        McpTransport::Sse(McpSseClient::new(sse_url))
    }

    pub fn new_stdio() -> Self {
        McpTransport::Stdio(McpStdioClient::new())
    }

    pub async fn call_capability(&self, method: &str, params: Value) -> Result<Value> {
        match self {
            McpTransport::Http(client) => client.call_capability(method, params).await,
            McpTransport::WebSocket(client) => client.call_capability(method, params).await,
            McpTransport::Sse(_) => Err(anyhow::anyhow!("SSE transport doesn't support direct calls")),
            McpTransport::Stdio(client) => client.call_capability_sync(method, params),
        }
    }

    pub async fn list_capabilities(&self) -> Result<Value> {
        match self {
            McpTransport::Http(client) => client.list_capabilities().await,
            McpTransport::WebSocket(_) => Err(anyhow::anyhow!("WebSocket transport doesn't support listing capabilities directly")),
            McpTransport::Sse(_) => Err(anyhow::anyhow!("SSE transport doesn't support listing capabilities")),
            McpTransport::Stdio(_) => Err(anyhow::anyhow!("Stdio transport doesn't support listing capabilities")),
        }
    }

    pub async fn discover_capabilities(&self, requested: Option<Vec<String>>) -> Result<Value> {
        match self {
            McpTransport::Http(client) => client.discover_capabilities(requested).await,
            McpTransport::WebSocket(_) => Err(anyhow::anyhow!("WebSocket transport doesn't support discovery directly")),
            McpTransport::Sse(_) => Err(anyhow::anyhow!("SSE transport doesn't support discovery")),
            McpTransport::Stdio(_) => Err(anyhow::anyhow!("Stdio transport doesn't support discovery")),
        }
    }

    pub async fn ping(&self) -> Result<Value> {
        match self {
            McpTransport::Http(client) => client.ping().await,
            _ => Err(anyhow::anyhow!("Ping only supported via HTTP transport")),
        }
    }
}