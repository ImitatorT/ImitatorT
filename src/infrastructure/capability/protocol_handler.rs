//! MCP 协议处理器
//!
//! 处理 MCP (Model Context Protocol) 请求/响应

use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

use crate::core::capability::CapabilityRegistry;
use crate::domain::capability::CapabilityCallContext;

pub struct McpProtocolHandler {
    capability_registry: Arc<CapabilityRegistry>,
    // 可以在这里添加其他协议相关的状态
}

impl McpProtocolHandler {
    pub fn new(capability_registry: Arc<CapabilityRegistry>) -> Self {
        Self {
            capability_registry,
        }
    }

    /// 处理 MCP 请求
    pub async fn handle_request(&self, method: &str, params: Value) -> Result<Value> {
        match method {
            "capabilities/list" => self.handle_capabilities_list(params).await,
            "capabilities/discover" => self.handle_capabilities_discover(params).await,
            "capabilities/call" => self.handle_capabilities_call(params).await,
            "ping" => self.handle_ping().await,
            _ => self.handle_custom_capability(method, params).await,
        }
    }

    /// 列出所有可用的功能
    async fn handle_capabilities_list(&self, _params: Value) -> Result<Value> {
        let capabilities = self.capability_registry.list_all();

        let capabilities_json: Vec<Value> = capabilities
            .iter()
            .map(|cap| {
                serde_json::json!({
                    "id": &cap.id,
                    "name": &cap.name,
                    "description": &cap.description,
                    "path": cap.capability_path.to_path_string(),
                    "protocol": &cap.protocol,
                    "endpoint": &cap.endpoint,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "capabilities": capabilities_json,
            "count": capabilities_json.len(),
        }))
    }

    /// 发现特定的功能
    async fn handle_capabilities_discover(&self, params: Value) -> Result<Value> {
        let requested = params
            .get("requested")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        let capabilities = if requested.is_empty() {
            self.capability_registry.list_all()
        } else {
            let mut caps = Vec::new();
            for req_cap in requested {
                if let Some(cap) = self.capability_registry.get(&req_cap) {
                    caps.push(cap);
                }
            }
            caps
        };

        let capabilities_json: Vec<Value> = capabilities
            .iter()
            .map(|cap| {
                serde_json::json!({
                    "name": &cap.name,
                    "version": "1.0.0", // 默认版本
                    "documentation": &cap.description,
                    "input_schema": &cap.input_schema,
                    "output_schema": &cap.output_schema,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "capabilities": capabilities_json,
        }))
    }

    /// 调用特定功能
    async fn handle_capabilities_call(&self, params: Value) -> Result<Value> {
        let capability_id = params
            .get("capability_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("capability_id is required"))?;

        let capability_params = params
            .get("params")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

        // 创建调用上下文
        let _context = CapabilityCallContext::new(
            "mcp_client".to_string(), // 从 MCP 客户端调用
            capability_params.clone(),
        );

        // 这里应该调用实际的功能执行器，但现在我们返回一个模拟响应
        // 在实际实现中，这里会查找并调用对应的 CapabilityExecutor
        Ok(serde_json::json!({
            "result": "Capability call handled by protocol handler",
            "capability_id": capability_id,
            "params": capability_params,
        }))
    }

    /// 处理 ping 请求
    async fn handle_ping(&self) -> Result<Value> {
        Ok(serde_json::json!({
            "result": "pong",
        }))
    }

    /// 处理自定义功能请求
    async fn handle_custom_capability(&self, method: &str, params: Value) -> Result<Value> {
        // 检查是否有一个匹配的功能ID
        if let Some(capability) = self.capability_registry.get(method) {
            // 创建调用上下文
            let _context = CapabilityCallContext::new("mcp_client".to_string(), params.clone());

            // 在实际实现中，这里会调用对应的 CapabilityExecutor
            // 现在我们返回一个模拟响应
            Ok(serde_json::json!({
                "result": format!("Handled custom capability: {}", capability.name),
                "capability_id": capability.id,
                "params": params,
            }))
        } else {
            Err(anyhow::anyhow!("Unknown method: {}", method))
        }
    }

    /// 获取协议信息
    pub fn get_protocol_info(&self) -> Value {
        serde_json::json!({
            "protocol": "MCP",
            "version": "1.0.0",
            "features": [
                "capabilities/list",
                "capabilities/discover",
                "capabilities/call",
                "ping"
            ],
        })
    }
}
