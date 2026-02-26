//! CapabilityProvider 实现
//!
//! 提供框架功能和应用功能的解耦查询机制

use crate::core::capability::{CapabilityNode, CapabilityRegistry};
use crate::domain::capability::{Capability, CapabilityNodeInfo, CapabilityProvider, MatchType};
use std::sync::Arc;

/// 组合 CapabilityProvider
///
/// 同时查询多个 Provider，合并结果
pub struct CompositeCapabilityProvider {
    providers: Vec<Box<dyn CapabilityProvider>>,
}

impl CompositeCapabilityProvider {
    /// 创建空的组合提供者
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// 添加提供者
    pub fn add_provider(mut self, provider: Box<dyn CapabilityProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// 添加应用功能提供者（从 CapabilityRegistry 创建）
    pub fn with_registry(self, registry: Arc<CapabilityRegistry>) -> Self {
        self.add_provider(Box::new(RegistryCapabilityProvider::new(registry)))
    }
}

impl Default for CompositeCapabilityProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityProvider for CompositeCapabilityProvider {
    fn list_capabilities(&self) -> Vec<Capability> {
        let mut capabilities = Vec::new();
        for provider in &self.providers {
            capabilities.extend(provider.list_capabilities());
        }
        capabilities
    }

    fn search_capabilities(&self, query: &str, match_type: MatchType) -> Vec<Capability> {
        let mut capabilities = Vec::new();
        for provider in &self.providers {
            capabilities.extend(provider.search_capabilities(query, match_type));
        }
        capabilities
    }

    fn list_capabilities_by_path(&self, path: &str) -> Vec<Capability> {
        let mut capabilities = Vec::new();
        for provider in &self.providers {
            capabilities.extend(provider.list_capabilities_by_path(path));
        }
        capabilities
    }

    fn get_capability_tree(&self) -> CapabilityNodeInfo {
        let mut root = CapabilityNodeInfo::new("root", "");

        // 收集所有提供者的分类
        for provider in &self.providers {
            let provider_tree = provider.get_capability_tree();
            merge_capability_tree(&mut root, provider_tree);
        }

        root
    }
}

/// 从 CapabilityRegistry 创建的提供者
pub struct RegistryCapabilityProvider {
    registry: Arc<CapabilityRegistry>,
}

impl RegistryCapabilityProvider {
    /// 从 CapabilityRegistry 创建提供者
    pub fn new(registry: Arc<CapabilityRegistry>) -> Self {
        Self { registry }
    }
}

impl CapabilityProvider for RegistryCapabilityProvider {
    fn list_capabilities(&self) -> Vec<Capability> {
        self.registry.list_all()
    }

    fn search_capabilities(&self, query: &str, match_type: MatchType) -> Vec<Capability> {
        let all_capabilities = self.registry.list_all();
        let query_lower = query.to_lowercase();

        all_capabilities
            .into_iter()
            .filter(|capability| {
                match match_type {
                    MatchType::Exact => {
                        // 精确匹配：ID、名称或完整分类路径
                        capability.id.to_lowercase() == query_lower
                            || capability.name.to_lowercase() == query_lower
                            || capability
                                .capability_path
                                .to_path_string()
                                .to_lowercase()
                                == query_lower
                    }
                    MatchType::Fuzzy => {
                        // 模糊匹配：ID、名称、描述、分类包含查询词
                        capability.id.to_lowercase().contains(&query_lower)
                            || capability.name.to_lowercase().contains(&query_lower)
                            || capability.description.to_lowercase().contains(&query_lower)
                            || capability
                                .capability_path
                                .to_path_string()
                                .to_lowercase()
                                .contains(&query_lower)
                    }
                }
            })
            .collect()
    }

    fn list_capabilities_by_path(&self, path: &str) -> Vec<Capability> {
        // 在同步 trait 方法中执行异步代码
        // 注意：这需要在 tokio 运行时上下文中调用
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(handle) => {
                tokio::task::block_in_place(|| {
                    handle.block_on(async {
                        self.registry.find_by_path(path).await
                    })
                })
            }
            Err(_) => {
                // 如果不在运行时中，返回空列表
                Vec::new()
            }
        }
    }

    fn get_capability_tree(&self) -> CapabilityNodeInfo {
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(handle) => {
                tokio::task::block_in_place(|| {
                    handle.block_on(async {
                        let root = self.registry.get_capability_tree().await;
                        convert_to_info(&root, "")
                    })
                })
            }
            Err(_) => {
                CapabilityNodeInfo::new("root", "")
            }
        }
    }
}

/// 将 CapabilityNode 转换为 CapabilityNodeInfo
fn convert_to_info(node: &CapabilityNode, parent_path: &str) -> CapabilityNodeInfo {
    let path = if parent_path.is_empty() {
        node.name.clone()
    } else {
        format!("{}/{}", parent_path, node.name)
    };

    let mut info = CapabilityNodeInfo::new(&node.name, &path);
    info.capability_count = node.capabilities().len();

    for child in node.children().values() {
        info.children.push(convert_to_info(child, &path));
    }

    info
}

/// 合并两个分类树
fn merge_capability_tree(target: &mut CapabilityNodeInfo, source: CapabilityNodeInfo) {
    target.capability_count += source.capability_count;

    for source_child in source.children {
        let mut found = false;
        for target_child in &mut target.children {
            if target_child.name == source_child.name {
                merge_capability_tree(target_child, source_child.clone());
                found = true;
                break;
            }
        }

        if !found {
            target.children.push(source_child);
        }
    }
}

/// 框架内置功能提供者
///
/// 提供框架级别的功能定义（不执行，只提供元数据）
pub struct FrameworkCapabilityProvider;

impl FrameworkCapabilityProvider {
    /// 创建框架功能提供者
    pub fn new() -> Self {
        Self
    }

    /// 获取所有框架功能定义
    pub fn get_framework_capabilities() -> Vec<Capability> {
        vec![
            // Capability Discovery 类
            Self::create_capability_discovery(),
            Self::create_capability_list(),
            Self::create_capability_info(),
            // MCP Protocol 类
            Self::create_mcp_ping(),
            Self::create_mcp_initialize(),
            Self::create_mcp_server_notification(),
            Self::create_mcp_server_request(),
        ]
    }

    fn create_capability_discovery() -> Capability {
        use crate::domain::capability::{CapabilityPath, InputSchema, OutputSchema};
        use serde_json::json;

        Capability::new(
            "mcp.discover",
            "MCP Capability Discovery",
            "Discover available MCP capabilities",
            CapabilityPath::from_str("mcp/discovery"),
            InputSchema::object()
                .property(
                    "requested",
                    InputSchema::string_array()
                        .description("List of requested capability names")
                        .optional(),
                )
                .build(),
            OutputSchema::object()
                .property(
                    "capabilities",
                    OutputSchema::array(json!({
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "version": {"type": "string"},
                            "documentation": {"type": "string"}
                        }
                    }))
                    .description("Discovered capabilities"),
                )
                .build(),
            "http".to_string(),
            Some("/mcp/discover".to_string()),
        )
    }

    fn create_capability_list() -> Capability {
        use crate::domain::capability::{CapabilityPath, InputSchema, OutputSchema};
        use serde_json::json;

        Capability::new(
            "mcp.list",
            "List Capabilities",
            "List all available capabilities",
            CapabilityPath::from_str("mcp/discovery"),
            InputSchema::object().build(),
            OutputSchema::object()
                .property(
                    "capabilities",
                    OutputSchema::array(json!({
                        "type": "object",
                        "properties": {
                            "id": {"type": "string"},
                            "name": {"type": "string"},
                            "description": {"type": "string"},
                            "protocol": {"type": "string"},
                            "endpoint": {"type": "string"}
                        }
                    }))
                    .description("List of capabilities"),
                )
                .build(),
            "http".to_string(),
            Some("/mcp/list".to_string()),
        )
    }

    fn create_capability_info() -> Capability {
        use crate::domain::capability::{CapabilityPath, InputSchema, OutputSchema};

        Capability::new(
            "mcp.info",
            "Get Capability Info",
            "Get detailed information about a specific capability",
            CapabilityPath::from_str("mcp/discovery"),
            InputSchema::object()
                .property(
                    "capability_id",
                    InputSchema::string().description("ID of the capability to get info for"),
                )
                .build(),
            OutputSchema::object()
                .raw_property("capability", OutputSchema::object().build(), false)
                .build(),
            "http".to_string(),
            Some("/mcp/info".to_string()),
        )
    }

    fn create_mcp_ping() -> Capability {
        use crate::domain::capability::{CapabilityPath, InputSchema, OutputSchema};

        Capability::new(
            "mcp.ping",
            "MCP Ping",
            "Ping the MCP server to check connectivity",
            CapabilityPath::from_str("mcp/protocol"),
            InputSchema::object().build(),
            OutputSchema::object()
                .property("result", OutputSchema::string().description("Ping result"))
                .build(),
            "http".to_string(),
            Some("/mcp/ping".to_string()),
        )
    }

    fn create_mcp_initialize() -> Capability {
        use crate::domain::capability::{CapabilityPath, InputSchema, OutputSchema};
        use serde_json::json;

        Capability::new(
            "mcp.initialize",
            "MCP Initialize",
            "Initialize the MCP connection",
            CapabilityPath::from_str("mcp/protocol"),
            InputSchema::object()
                .raw_property(
                    "client_info",
                    InputSchema::object()
                        .property("name", InputSchema::string().description("Client name"))
                        .property(
                            "version",
                            InputSchema::string().description("Client version").optional(),
                        )
                        .build(),
                    true
                )
                .property(
                    "capabilities",
                    InputSchema::array(json!({
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "version": {"type": "string"}
                        }
                    }))
                    .description("Requested capabilities")
                    .optional(),
                )
                .build(),
            OutputSchema::object()
                .raw_property(
                    "server_info",
                    OutputSchema::object()
                        .property("name", OutputSchema::string().description("Server name"))
                        .property(
                            "version",
                            OutputSchema::string().description("Server version"),
                        )
                        .build(),
                    true
                )
                .build(),
            "http".to_string(),
            Some("/mcp/initialize".to_string()),
        )
    }

    fn create_mcp_server_notification() -> Capability {
        use crate::domain::capability::{CapabilityPath, InputSchema, OutputSchema};

        Capability::new(
            "mcp.server.notification",
            "MCP Server Notification",
            "Handle server-initiated notifications",
            CapabilityPath::from_str("mcp/protocol"),
            InputSchema::object()
                .property("method", InputSchema::string().description("Notification method"))
                .raw_property(
                    "params",
                    InputSchema::object()
                        .property("type", InputSchema::string().description("Parameter type"))
                        .property("value", InputSchema::string().description("Parameter value"))
                        .build(),
                    true
                )
                .build(),
            OutputSchema::object()
                .property("success", OutputSchema::boolean().description("Success status"))
                .build(),
            "websocket".to_string(),
            None,
        )
    }

    fn create_mcp_server_request() -> Capability {
        use crate::domain::capability::{CapabilityPath, InputSchema, OutputSchema};

        Capability::new(
            "mcp.server.request",
            "MCP Server Request",
            "Handle server-initiated requests",
            CapabilityPath::from_str("mcp/protocol"),
            InputSchema::object()
                .property("method", InputSchema::string().description("Request method"))
                .raw_property(
                    "params",
                    InputSchema::object()
                        .property("type", InputSchema::string().description("Parameter type"))
                        .property("value", InputSchema::string().description("Parameter value"))
                        .build(),
                    true
                )
                .build(),
            OutputSchema::object()
                .raw_property("result", OutputSchema::object().build(), false)
                .build(),
            "websocket".to_string(),
            None,
        )
    }
}

impl Default for FrameworkCapabilityProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityProvider for FrameworkCapabilityProvider {
    fn list_capabilities(&self) -> Vec<Capability> {
        Self::get_framework_capabilities()
    }

    fn search_capabilities(&self, query: &str, match_type: MatchType) -> Vec<Capability> {
        let all_capabilities = Self::get_framework_capabilities();
        let query_lower = query.to_lowercase();

        all_capabilities
            .into_iter()
            .filter(|capability| {
                match match_type {
                    MatchType::Exact => {
                        capability.id.to_lowercase() == query_lower
                            || capability.name.to_lowercase() == query_lower
                    }
                    MatchType::Fuzzy => {
                        capability.id.to_lowercase().contains(&query_lower)
                            || capability.name.to_lowercase().contains(&query_lower)
                            || capability
                                .description
                                .to_lowercase()
                                .contains(&query_lower)
                    }
                }
            })
            .collect()
    }

    fn list_capabilities_by_path(&self, path: &str) -> Vec<Capability> {
        let all_capabilities = Self::get_framework_capabilities();
        let capability_path = crate::domain::capability::CapabilityPath::from_str(path);

        all_capabilities
            .into_iter()
            .filter(|capability| {
                capability.capability_path == capability_path
                    || capability.capability_path.contains(&capability_path)
            })
            .collect()
    }

    fn get_capability_tree(&self) -> CapabilityNodeInfo {
        let mut root = CapabilityNodeInfo::new("root", "");

        for capability in Self::get_framework_capabilities() {
            add_capability_to_tree(&mut root, &capability);
        }

        root
    }
}

/// 将功能添加到分类树
fn add_capability_to_tree(root: &mut CapabilityNodeInfo, capability: &Capability) {
    let segments = capability.capability_path.segments();

    let mut current = root;
    let mut current_path = String::new();

    for segment in segments.iter() {
        if !current_path.is_empty() {
            current_path.push('/');
        }
        current_path.push_str(segment);

        // 查找或创建子节点
        let child_index = current
            .children
            .iter()
            .position(|c| c.name == *segment);

        match child_index {
            Some(index) => {
                current = &mut current.children[index];
            }
            None => {
                let new_child = CapabilityNodeInfo::new(segment, &current_path);
                current.children.push(new_child);
                let index = current.children.len() - 1;
                current = &mut current.children[index];
            }
        }
    }

    current.capability_count += 1;
}