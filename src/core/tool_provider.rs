//! ToolProvider 实现
//!
//! 提供框架工具和应用工具的解耦查询机制

use crate::core::tool::{CategoryNode, ToolRegistry};
use crate::domain::tool::{CategoryNodeInfo, MatchType, Tool, ToolProvider};
use std::sync::Arc;

/// 组合 ToolProvider
///
/// 同时查询多个 Provider，合并结果
pub struct CompositeToolProvider {
    providers: Vec<Box<dyn ToolProvider>>,
}

impl CompositeToolProvider {
    /// 创建空的组合提供者
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// 添加提供者
    pub fn add_provider(mut self, provider: Box<dyn ToolProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// 添加应用工具提供者（从 ToolRegistry 创建）
    pub fn with_registry(self, registry: Arc<ToolRegistry>) -> Self {
        self.add_provider(Box::new(RegistryToolProvider::new(registry)))
    }
}

impl Default for CompositeToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolProvider for CompositeToolProvider {
    fn list_tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();
        for provider in &self.providers {
            tools.extend(provider.list_tools());
        }
        tools
    }

    fn search_tools(&self, query: &str, match_type: MatchType) -> Vec<Tool> {
        let mut tools = Vec::new();
        for provider in &self.providers {
            tools.extend(provider.search_tools(query, match_type));
        }
        tools
    }

    fn list_tools_by_category(&self, category: &str) -> Vec<Tool> {
        let mut tools = Vec::new();
        for provider in &self.providers {
            tools.extend(provider.list_tools_by_category(category));
        }
        tools
    }

    fn get_category_tree(&self) -> CategoryNodeInfo {
        let mut root = CategoryNodeInfo::new("root", "");

        // 收集所有提供者的分类
        for provider in &self.providers {
            let provider_tree = provider.get_category_tree();
            merge_category_tree(&mut root, provider_tree);
        }

        root
    }
}

/// 从 ToolRegistry 创建的提供者
pub struct RegistryToolProvider {
    registry: Arc<ToolRegistry>,
}

impl RegistryToolProvider {
    /// 从 ToolRegistry 创建提供者
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }
}

impl ToolProvider for RegistryToolProvider {
    fn list_tools(&self) -> Vec<Tool> {
        self.registry.list_all()
    }

    fn search_tools(&self, query: &str, match_type: MatchType) -> Vec<Tool> {
        let all_tools = self.registry.list_all();
        let query_lower = query.to_lowercase();

        all_tools
            .into_iter()
            .filter(|tool| {
                match match_type {
                    MatchType::Exact => {
                        // 精确匹配：ID、名称或完整分类路径
                        tool.id.to_lowercase() == query_lower
                            || tool.name.to_lowercase() == query_lower
                            || tool.category.to_path_string().to_lowercase() == query_lower
                    }
                    MatchType::Fuzzy => {
                        // 模糊匹配：ID、名称、描述、分类包含查询词
                        tool.id.to_lowercase().contains(&query_lower)
                            || tool.name.to_lowercase().contains(&query_lower)
                            || tool.description.to_lowercase().contains(&query_lower)
                            || tool.category.to_path_string().to_lowercase().contains(&query_lower)
                    }
                }
            })
            .collect()
    }

    fn list_tools_by_category(&self, category: &str) -> Vec<Tool> {
        // 在同步 trait 方法中执行异步代码
        // 注意：这需要在 tokio 运行时上下文中调用
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(handle) => {
                tokio::task::block_in_place(|| {
                    handle.block_on(async {
                        self.registry.find_by_category(category).await
                    })
                })
            }
            Err(_) => {
                // 如果不在运行时中，返回空列表
                Vec::new()
            }
        }
    }

    fn get_category_tree(&self) -> CategoryNodeInfo {
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(handle) => {
                tokio::task::block_in_place(|| {
                    handle.block_on(async {
                        let root = self.registry.get_category_tree().await;
                        convert_to_info(&root, "")
                    })
                })
            }
            Err(_) => {
                CategoryNodeInfo::new("root", "")
            }
        }
    }
}

/// 将 CategoryNode 转换为 CategoryNodeInfo
fn convert_to_info(node: &CategoryNode, parent_path: &str) -> CategoryNodeInfo {
    let path = if parent_path.is_empty() {
        node.name.clone()
    } else {
        format!("{}/{}", parent_path, node.name)
    };

    let mut info = CategoryNodeInfo::new(&node.name, &path);
    info.tool_count = node.tools().len();

    for child in node.children().values() {
        info.children.push(convert_to_info(child, &path));
    }

    info
}

/// 合并两个分类树
fn merge_category_tree(target: &mut CategoryNodeInfo, source: CategoryNodeInfo) {
    target.tool_count += source.tool_count;

    for source_child in source.children {
        let mut found = false;
        for target_child in &mut target.children {
            if target_child.name == source_child.name {
                merge_category_tree(target_child, source_child.clone());
                found = true;
                break;
            }
        }

        if !found {
            target.children.push(source_child);
        }
    }
}

/// 框架内置工具提供者
///
/// 提供框架级别的工具定义（不执行，只提供元数据）
pub struct FrameworkToolProvider;

impl FrameworkToolProvider {
    /// 创建框架工具提供者
    pub fn new() -> Self {
        Self
    }

    /// 获取所有框架工具定义
    pub fn get_framework_tools() -> Vec<Tool> {
        vec![
            // Tool 查询类
            Self::create_tool_search(),
            Self::create_tool_list_categories(),
            Self::create_tool_get_category_tools(),
            // 消息发送类
            Self::create_message_send_direct(),
            Self::create_message_send_group(),
            Self::create_message_reply(),
            // 时间类
            Self::create_time_now(),
            // 组织架构类
            Self::create_org_get_structure(),
            Self::create_org_get_department(),
            Self::create_org_get_leader(),
            Self::create_org_find_agents(),
            Self::create_org_get_sub_departments(),
            Self::create_org_get_subordinates(),
        ]
    }

    fn create_tool_search() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "tool.search",
            "搜索工具",
            "按名称、描述、分类搜索工具，支持精确匹配和模糊匹配",
            CategoryPath::from_string("tool/query"),
            JsonSchema::object()
                .property("query", JsonSchema::string().description("搜索关键词"))
                .property(
                    "match_type",
                    JsonSchema::string()
                        .description("匹配类型：exact(精确) 或 fuzzy(模糊)")
                        .optional(),
                )
                .property(
                    "category_filter",
                    JsonSchema::string()
                        .description("限制在指定分类下搜索")
                        .optional(),
                )
                .build(),
        )
        .with_returns(ReturnType::new(
            "匹配的工具列表",
            json!({"type": "array", "items": {"type": "object"}}),
        ))
    }

    fn create_tool_list_categories() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "tool.list_categories",
            "列出工具分类",
            "获取所有工具的分类层级结构",
            CategoryPath::from_string("tool/query"),
            JsonSchema::object()
                .property(
                    "parent_category",
                    JsonSchema::string()
                        .description("父分类路径，为空则返回所有根分类")
                        .optional(),
                )
                .build(),
        )
        .with_returns(ReturnType::new(
            "分类树列表",
            json!({"type": "array", "items": {"type": "object"}}),
        ))
    }

    fn create_tool_get_category_tools() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "tool.get_category_tools",
            "获取分类工具",
            "获取指定分类下的所有工具",
            CategoryPath::from_string("tool/query"),
            JsonSchema::object()
                .property(
                    "category",
                    JsonSchema::string().description("分类路径，如 'tool/query'"),
                )
                .property(
                    "recursive",
                    JsonSchema::boolean()
                        .description("是否包含子分类的工具")
                        .optional(),
                )
                .build(),
        )
        .with_returns(ReturnType::new(
            "工具列表",
            json!({"type": "array", "items": {"type": "object"}}),
        ))
    }

    fn create_message_send_direct() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "message.send_direct",
            "发送私聊消息",
            "向指定 Agent 发送私聊消息",
            CategoryPath::from_string("message/send"),
            JsonSchema::object()
                .property("to_agent_id", JsonSchema::string().description("接收者 Agent ID"))
                .property("content", JsonSchema::string().description("消息内容"))
                .property(
                    "reply_to_message_id",
                    JsonSchema::string()
                        .description("回复的消息ID")
                        .optional(),
                )
                .build(),
        )
        .with_returns(ReturnType::new("发送结果", json!({"type": "boolean"})))
    }

    fn create_message_send_group() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "message.send_group",
            "发送群聊消息",
            "向指定群组发送消息，支持 @ 他人",
            CategoryPath::from_string("message/send"),
            JsonSchema::object()
                .property("group_id", JsonSchema::string().description("群组 ID"))
                .property("content", JsonSchema::string().description("消息内容"))
                .property(
                    "mention_agent_ids",
                    JsonSchema::string_array()
                        .description("需要 @ 的 Agent ID 列表")
                        .optional(),
                )
                .property(
                    "reply_to_message_id",
                    JsonSchema::string()
                        .description("回复的消息ID")
                        .optional(),
                )
                .build(),
        )
        .with_returns(ReturnType::new("发送结果", json!({"type": "boolean"})))
    }

    fn create_message_reply() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "message.reply",
            "回复消息",
            "引用并回复指定消息，可同时 @ 他人",
            CategoryPath::from_string("message/send"),
            JsonSchema::object()
                .property("message_id", JsonSchema::string().description("要回复的消息ID"))
                .property("content", JsonSchema::string().description("回复内容"))
                .property(
                    "mention_agent_ids",
                    JsonSchema::string_array()
                        .description("需要 @ 的 Agent ID 列表")
                        .optional(),
                )
                .build(),
        )
        .with_returns(ReturnType::new("发送结果", json!({"type": "boolean"})))
    }

    fn create_time_now() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "time.now",
            "获取当前时间",
            "获取系统当前时间",
            CategoryPath::from_string("time/query"),
            JsonSchema::object().build(),
        )
        .with_returns(ReturnType::new(
            "当前时间信息",
            json!({"type": "object"}),
        ))
    }

    fn create_org_get_structure() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "org.get_structure",
            "获取组织架构",
            "获取完整的组织架构树",
            CategoryPath::from_string("org/query"),
            JsonSchema::object().build(),
        )
        .with_returns(ReturnType::new("组织架构树", json!({"type": "object"})))
    }

    fn create_org_get_department() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "org.get_department",
            "Get Department Info",
            "Get detailed information of specified department",
            CategoryPath::from_string("org/query"),
            JsonSchema::object()
                .property("department_id", JsonSchema::string().description("Department ID"))
                .build(),
        )
        .with_returns(ReturnType::new("Department Info", json!({"type": "object"})))
    }

    fn create_org_get_leader() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "org.get_leader",
            "Get Department Leader",
            "Get the leader information of specified department",
            CategoryPath::from_string("org/query"),
            JsonSchema::object()
                .property("department_id", JsonSchema::string().description("Department ID"))
                .build(),
        )
        .with_returns(ReturnType::new("领导信息", json!({"type": "object"})))
    }

    fn create_org_find_agents() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "org.find_agents",
            "查找 Agent",
            "Multi-dimensional Agent search by ID, name, position, department, description, etc.",
            CategoryPath::from_string("org/query"),
            JsonSchema::object()
                .property(
                    "query_type",
                    JsonSchema::string()
                        .description("查询类型：id, name, role, department, description"),
                )
                .property("query_value", JsonSchema::string().description("查询值"))
                .property(
                    "fuzzy_match",
                    JsonSchema::boolean()
                        .description("是否模糊匹配")
                        .optional(),
                )
                .build(),
        )
        .with_returns(ReturnType::new(
            "匹配的 Agent 列表",
            json!({"type": "array", "items": {"type": "object"}}),
        ))
    }

    fn create_org_get_sub_departments() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "org.get_sub_departments",
            "Get Sub-departments",
            "Get the list of direct sub-departments of specified department",
            CategoryPath::from_string("org/query"),
            JsonSchema::object()
                .property("department_id", JsonSchema::string().description("Department ID"))
                .build(),
        )
        .with_returns(ReturnType::new(
            "Sub-department List",
            json!({"type": "array", "items": {"type": "object"}}),
        ))
    }

    fn create_org_get_subordinates() -> Tool {
        use crate::domain::tool::{CategoryPath, JsonSchema, ReturnType};
        use serde_json::json;

        Tool::new(
            "org.get_subordinates",
            "Get Subordinates",
            "Get the list of subordinates for specified Agent (based on department leadership relationship)",
            CategoryPath::from_string("org/query"),
            JsonSchema::object()
                .property("agent_id", JsonSchema::string().description("Agent ID"))
                .build(),
        )
        .with_returns(ReturnType::new(
            "Subordinate Agent List",
            json!({"type": "array", "items": {"type": "object"}}),
        ))
    }
}

impl Default for FrameworkToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolProvider for FrameworkToolProvider {
    fn list_tools(&self) -> Vec<Tool> {
        Self::get_framework_tools()
    }

    fn search_tools(&self, query: &str, match_type: MatchType) -> Vec<Tool> {
        let all_tools = Self::get_framework_tools();
        let query_lower = query.to_lowercase();

        all_tools
            .into_iter()
            .filter(|tool| {
                match match_type {
                    MatchType::Exact => {
                        tool.id.to_lowercase() == query_lower
                            || tool.name.to_lowercase() == query_lower
                    }
                    MatchType::Fuzzy => {
                        tool.id.to_lowercase().contains(&query_lower)
                            || tool.name.to_lowercase().contains(&query_lower)
                            || tool.description.to_lowercase().contains(&query_lower)
                    }
                }
            })
            .collect()
    }

    fn list_tools_by_category(&self, category: &str) -> Vec<Tool> {
        let all_tools = Self::get_framework_tools();
        let category_path = crate::domain::tool::CategoryPath::from_string(category);

        all_tools
            .into_iter()
            .filter(|tool| {
                tool.category == category_path || tool.category.contains(&category_path)
            })
            .collect()
    }

    fn get_category_tree(&self) -> CategoryNodeInfo {
        let mut root = CategoryNodeInfo::new("root", "");

        for tool in Self::get_framework_tools() {
            add_tool_to_tree(&mut root, &tool);
        }

        root
    }
}

/// 将工具添加到分类树
fn add_tool_to_tree(root: &mut CategoryNodeInfo, tool: &Tool) {
    let segments = tool.category.segments();

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
                let new_child = CategoryNodeInfo::new(segment, &current_path);
                current.children.push(new_child);
                let index = current.children.len() - 1;
                current = &mut current.children[index];
            }
        }
    }

    current.tool_count += 1;
}
