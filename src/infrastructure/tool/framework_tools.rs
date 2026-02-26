//! 框架内置工具实现
//!
//! 提供通用的框架级工具执行能力

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::messaging::MessageBus;
use crate::core::store::{MessageFilter, Store};
use crate::core::tool::ToolRegistry;
use crate::core::tool_provider::{CompositeToolProvider, FrameworkToolProvider};
use crate::domain::{Message, MessageTarget, Organization};
use crate::domain::tool::{MatchType, ToolCallContext, ToolProvider};
use crate::infrastructure::tool::ToolResult;

/// 工具执行环境
///
/// 包含工具执行所需的所有运行时依赖
#[derive(Clone)]
pub struct ToolEnvironment {
    /// 消息总线
    pub message_bus: Arc<MessageBus>,
    /// 组织架构
    pub organization: Arc<RwLock<Organization>>,
    /// 工具注册表
    pub tool_registry: Arc<ToolRegistry>,
    /// 工具提供者（用于查询）
    pub tool_provider: Arc<CompositeToolProvider>,
    /// 消息存储
    pub message_store: Arc<dyn Store>,
}

impl ToolEnvironment {
    /// 创建新的工具环境
    pub fn new(
        message_bus: Arc<MessageBus>,
        organization: Arc<RwLock<Organization>>,
        tool_registry: Arc<ToolRegistry>,
        message_store: Arc<dyn Store>,
    ) -> Self {
        // 创建组合提供者，包含框架工具和应用工具
        let tool_provider = CompositeToolProvider::new()
            .add_provider(Box::new(FrameworkToolProvider::new()))
            .with_registry(tool_registry.clone());

        Self {
            message_bus,
            organization,
            tool_registry,
            tool_provider: Arc::new(tool_provider),
            message_store,
        }
    }
}

/// 框架工具执行器
pub struct FrameworkToolExecutor {
    env: ToolEnvironment,
}

impl FrameworkToolExecutor {
    /// 创建框架工具执行器
    pub fn new(env: ToolEnvironment) -> Self {
        Self { env }
    }

    /// 获取支持的框架工具ID列表
    pub fn supported_tool_ids() -> Vec<&'static str> {
        vec![
            // Tool 查询类
            "tool.search",
            "tool.list_categories",
            "tool.get_category_tools",
            // 消息发送类
            "message.send_direct",
            "message.send_group",
            "message.reply",
            // 时间类
            "time.now",
            // 组织架构类
            "org.get_structure",
            "org.get_department",
            "org.get_leader",
            "org.find_agents",
            "org.get_sub_departments",
            "org.get_subordinates",
        ]
    }

    /// 执行工具调用
    pub async fn execute(
        &self,
        tool_id: &str,
        params: Value,
        context: &ToolCallContext,
    ) -> Result<ToolResult> {
        match tool_id {
            // Tool 查询类
            "tool.search" => self.execute_tool_search(params).await,
            "tool.list_categories" => self.execute_tool_list_categories(params).await,
            "tool.get_category_tools" => self.execute_tool_get_category_tools(params).await,
            // 消息发送类
            "message.send_direct" => self.execute_message_send_direct(params, context).await,
            "message.send_group" => self.execute_message_send_group(params, context).await,
            "message.reply" => self.execute_message_reply(params, context).await,
            // 时间类
            "time.now" => self.execute_time_now().await,
            // 组织架构类
            "org.get_structure" => self.execute_org_get_structure().await,
            "org.get_department" => self.execute_org_get_department(params).await,
            "org.get_leader" => self.execute_org_get_leader(params).await,
            "org.find_agents" => self.execute_org_find_agents(params).await,
            "org.get_sub_departments" => self.execute_org_get_sub_departments(params).await,
            "org.get_subordinates" => self.execute_org_get_subordinates(params).await,
            _ => Ok(ToolResult::error(format!("Unknown tool: {}", tool_id))),
        }
    }

    // ==================== Tool 查询类 ====================

    async fn execute_tool_search(&self,
        params: Value,
    ) -> Result<ToolResult> {
        let query = params["query"].as_str().unwrap_or("");
        if query.is_empty() {
            return Ok(ToolResult::error("Query parameter is required"));
        }

        let match_type = match params["match_type"].as_str() {
            Some("exact") => MatchType::Exact,
            _ => MatchType::Fuzzy,
        };

        let category_filter = params["category_filter"].as_str();

        let mut results = self.env.tool_provider.search_tools(query, match_type);

        // 应用分类过滤
        if let Some(category) = category_filter {
            results.retain(|tool| {
                tool.category.to_path_string().starts_with(category)
            });
        }

        let tools_json: Vec<Value> = results.iter().map(|tool| {
            json!({
                "id": tool.id,
                "name": tool.name,
                "description": tool.description,
                "category": tool.category.to_path_string(),
            })
        }).collect();

        Ok(ToolResult::success(json!({
            "query": query,
            "match_type": if match_type == MatchType::Exact { "exact" } else { "fuzzy" },
            "count": tools_json.len(),
            "tools": tools_json,
        })))
    }

    async fn execute_tool_list_categories(
        &self,
        params: Value,
    ) -> Result<ToolResult> {
        let parent_category = params["parent_category"].as_str().unwrap_or("");

        let tree = self.env.tool_provider.get_category_tree();

        // 如果指定了父分类，找到对应节点
        let target_node = if parent_category.is_empty() {
            tree
        } else {
            find_category_node(&tree, parent_category)
                .unwrap_or_else(|| crate::domain::tool::CategoryNodeInfo::new("empty", ""))
        };

        Ok(ToolResult::success(json!({
            "parent": parent_category,
            "categories": target_node.children.iter().map(|c| {
                json!({
                    "name": c.name,
                    "path": c.path,
                    "tool_count": c.tool_count,
                })
            }).collect::<Vec<_>>(),
        })))
    }

    async fn execute_tool_get_category_tools(
        &self,
        params: Value,
    ) -> Result<ToolResult> {
        let category = params["category"].as_str().unwrap_or("");
        if category.is_empty() {
            return Ok(ToolResult::error("Category parameter is required"));
        }

        let _recursive = params["recursive"].as_bool().unwrap_or(true);

        let tools = self.env.tool_provider.list_tools_by_category(category);

        let tools_json: Vec<Value> = tools.iter().map(|tool| {
            json!({
                "id": tool.id,
                "name": tool.name,
                "description": tool.description,
                "category": tool.category.to_path_string(),
                "parameters": tool.parameters,
            })
        }).collect();

        Ok(ToolResult::success(json!({
            "category": category,
            "count": tools_json.len(),
            "tools": tools_json,
        })))
    }

    // ==================== 消息发送类 ====================

    async fn execute_message_send_direct(
        &self,
        params: Value,
        context: &ToolCallContext,
    ) -> Result<ToolResult> {
        let to_agent_id = params["to_agent_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("to_agent_id is required"))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("content is required"))?;
        let reply_to = params["reply_to_message_id"].as_str();

        let mut message = Message::private(&context.caller_id,
            to_agent_id,
            content
        );

        if let Some(reply_id) = reply_to {
            message = message.with_reply_to(reply_id);
        }

        self.env.message_bus.send(message).await?;

        Ok(ToolResult::success(json!({ "sent": true })))
    }

    async fn execute_message_send_group(
        &self,
        params: Value,
        context: &ToolCallContext,
    ) -> Result<ToolResult> {
        let group_id = params["group_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("group_id is required"))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("content is required"))?;

        let mut message = Message::group(
            &context.caller_id,
            group_id,
            content
        );

        // 处理 @ 列表
        if let Some(mentions) = params["mention_agent_ids"].as_array() {
            for mention in mentions {
                if let Some(id) = mention.as_str() {
                    message = message.with_mention(id);
                }
            }
        }

        // 处理回复
        if let Some(reply_id) = params["reply_to_message_id"].as_str() {
            message = message.with_reply_to(reply_id);
        }

        self.env.message_bus.send(message).await?;

        Ok(ToolResult::success(json!({ "sent": true })))
    }

    async fn execute_message_reply(
        &self,
        params: Value,
        context: &ToolCallContext,
    ) -> Result<ToolResult> {
        let message_id = params["message_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("message_id is required"))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("content is required"))?;

        // 从消息存储中查找原消息
        let original_messages = self.env.message_store.load_messages(
            MessageFilter::new().limit(1).to(message_id)
        ).await?;

        let reply_message = if let Some(orig_msg) = original_messages.first() {
            // 如果找到了原始消息，则根据原始消息的目标创建回复
            let reply_content = format!("[回复消息 {}] {}", message_id, content);
            let mut message = match &orig_msg.to {
                MessageTarget::Direct(sender_id) => {
                    // 如果原始消息是私聊，回复给对方
                    if *sender_id == context.caller_id {
                        // 如果原始消息发送者就是当前调用者，回复给原消息的发送者
                        Message::private(&context.caller_id, &orig_msg.from, reply_content)
                    } else {
                        // 否则回复给原始消息发送者
                        Message::private(&context.caller_id, sender_id, reply_content)
                    }
                },
                MessageTarget::Group(group_id) => {
                    // 如果原始消息是群聊，回复到同一群组
                    Message::group(&context.caller_id, group_id, reply_content)
                }
            };

            // 设置回复关系
            message = message.with_reply_to(message_id);

            // 处理 @ 列表
            if let Some(mentions) = params["mention_agent_ids"].as_array() {
                for mention in mentions {
                    if let Some(id) = mention.as_str() {
                        message = message.with_mention(id);
                    }
                }
            }

            let message_id_clone = message.id.clone();
            let target_clone = format!("{:?}", message.to);

            // 发送消息
            self.env.message_bus.send(message).await?;
            Ok(ToolResult::success(json!({
                "sent": true,
                "message_id": message_id_clone,
                "reply_to": message_id,
                "target": target_clone,
            })))
        } else {
            // 如果没有找到原始消息，返回错误
            Ok(ToolResult::error(format!("Original message not found: {}", message_id)))
        };

        reply_message
    }

    // ==================== 时间类 ====================

    async fn execute_time_now(&self,
    ) -> Result<ToolResult> {
        let now = chrono::Utc::now();

        Ok(ToolResult::success(json!({
            "timestamp": now.timestamp(),
            "iso": now.to_rfc3339(),
            "date": now.format("%Y-%m-%d").to_string(),
            "time": now.format("%H:%M:%S").to_string(),
            "timezone": "UTC",
        })))
    }

    // ==================== 组织架构类 ====================

    async fn execute_org_get_structure(&self,
    ) -> Result<ToolResult> {
        let org = self.env.organization.read().await;
        let tree = org.build_tree();

        fn convert_node(node: &crate::domain::org::DepartmentNode) -> Value {
            json!({
                "id": node.department.id,
                "name": node.department.name,
                "leader_id": node.department.leader_id,
                "members": node.members,
                "children": node.children.iter().map(convert_node).collect::<Vec<_>>(),
            })
        }

        let departments: Vec<Value> = tree.iter().map(convert_node).collect();

        let agents: Vec<Value> = org.agents.iter().map(|a| {
            json!({
                "id": a.id,
                "name": a.name,
                "role": a.role.title,
                "department_id": a.department_id,
            })
        }).collect();

        Ok(ToolResult::success(json!({
            "departments": departments,
            "agents": agents,
            "total_departments": org.departments.len(),
            "total_agents": org.agents.len(),
        })))
    }

    async fn execute_org_get_department(
        &self,
        params: Value,
    ) -> Result<ToolResult> {
        let dept_id = params["department_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("department_id is required"))?;

        let org = self.env.organization.read().await;

        let dept = org.find_department(dept_id)
            .ok_or_else(|| anyhow::anyhow!("Department not found: {}", dept_id))?;

        let members: Vec<&crate::domain::Agent> = org.get_department_members(dept_id);

        Ok(ToolResult::success(json!({
            "id": dept.id,
            "name": dept.name,
            "parent_id": dept.parent_id,
            "leader_id": dept.leader_id,
            "members": members.iter().map(|m| {
                json!({
                    "id": m.id,
                    "name": m.name,
                    "role": m.role.title,
                })
            }).collect::<Vec<_>>(),
            "member_count": members.len(),
        })))
    }

    async fn execute_org_get_leader(
        &self,
        params: Value,
    ) -> Result<ToolResult> {
        let dept_id = params["department_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("department_id is required"))?;

        let org = self.env.organization.read().await;

        let leader = org.get_department_leader(dept_id)
            .ok_or_else(|| anyhow::anyhow!("No leader found for department: {}", dept_id))?;

        Ok(ToolResult::success(json!({
            "id": leader.id,
            "name": leader.name,
            "role": leader.role.title,
            "department_id": leader.department_id,
        })))
    }

    async fn execute_org_find_agents(
        &self,
        params: Value,
    ) -> Result<ToolResult> {
        let query_type = params["query_type"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("query_type is required"))?;
        let query_value = params["query_value"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("query_value is required"))?;
        let fuzzy = params["fuzzy_match"].as_bool().unwrap_or(false);

        let org = self.env.organization.read().await;
        let query_lower = query_value.to_lowercase();

        let results: Vec<&crate::domain::Agent> = org.agents.iter().filter(|agent| {
            match query_type {
                "id" => {
                    if fuzzy {
                        agent.id.to_lowercase().contains(&query_lower)
                    } else {
                        agent.id.to_lowercase() == query_lower
                    }
                }
                "name" => {
                    if fuzzy {
                        agent.name.to_lowercase().contains(&query_lower)
                    } else {
                        agent.name.to_lowercase() == query_lower
                    }
                }
                "role" | "position" => {
                    if fuzzy {
                        agent.role.title.to_lowercase().contains(&query_lower)
                    } else {
                        agent.role.title.to_lowercase() == query_lower
                    }
                }
                "department" => {
                    agent.department_id.as_ref().map_or(false, |d| {
                        if fuzzy {
                            d.to_lowercase().contains(&query_lower)
                        } else {
                            d.to_lowercase() == query_lower
                        }
                    })
                }
                "description" => {
                    if fuzzy {
                        agent.role.system_prompt.to_lowercase().contains(&query_lower)
                    } else {
                        agent.role.system_prompt.to_lowercase() == query_lower
                    }
                }
                _ => false,
            }
        }).collect();

        let agents_json: Vec<Value> = results.iter().map(|a| {
            json!({
                "id": a.id,
                "name": a.name,
                "role": a.role.title,
                "department_id": a.department_id,
                "expertise": a.role.expertise,
            })
        }).collect();

        Ok(ToolResult::success(json!({
            "query_type": query_type,
            "query_value": query_value,
            "fuzzy_match": fuzzy,
            "count": agents_json.len(),
            "agents": agents_json,
        })))
    }

    async fn execute_org_get_sub_departments(
        &self,
        params: Value,
    ) -> Result<ToolResult> {
        let dept_id = params["department_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("department_id is required"))?;

        let org = self.env.organization.read().await;

        let sub_depts = org.get_sub_departments(dept_id);

        let depts_json: Vec<Value> = sub_depts.iter().map(|d| {
            json!({
                "id": d.id,
                "name": d.name,
                "leader_id": d.leader_id,
            })
        }).collect();

        Ok(ToolResult::success(json!({
            "parent_id": dept_id,
            "count": depts_json.len(),
            "departments": depts_json,
        })))
    }

    async fn execute_org_get_subordinates(
        &self,
        params: Value,
    ) -> Result<ToolResult> {
        let agent_id = params["agent_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("agent_id is required"))?;

        let org = self.env.organization.read().await;

        // 查找该 Agent 所在的部门，检查是否为领导
        let agent = org.find_agent(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;

        let mut subordinates = Vec::new();

        if let Some(dept_id) = &agent.department_id {
            if let Some(dept) = org.find_department(dept_id) {
                // 检查是否为部门领导
                if dept.leader_id.as_ref() == Some(&agent_id.to_string()) {
                    // 获取部门其他成员作为下属
                    subordinates = org.get_department_members(dept_id)
                        .into_iter()
                        .filter(|a| a.id != agent_id)
                        .cloned()
                        .collect::<Vec<_>>();
                }
            }
        }

        let subordinates_json: Vec<Value> = subordinates.iter().map(|a| {
            json!({
                "id": a.id,
                "name": a.name,
                "role": a.role.title,
            })
        }).collect();

        Ok(ToolResult::success(json!({
            "agent_id": agent_id,
            "is_leader": !subordinates.is_empty(),
            "count": subordinates_json.len(),
            "subordinates": subordinates_json,
        })))
    }
}

/// 使用 domain::tool::CategoryNodeInfo
fn find_category_node(tree: &crate::domain::tool::CategoryNodeInfo, path: &str) -> Option<crate::domain::tool::CategoryNodeInfo> {
    if tree.path == path {
        return Some(tree.clone());
    }

    for child in &tree.children {
        if let Some(found) = find_category_node(child, path) {
            return Some(found);
        }
    }

    None
}

// ==================== ToolExecutor Trait Implementation ====================

use async_trait::async_trait;
use crate::infrastructure::tool::ToolExecutor as ToolExecutorTrait;

#[async_trait]
impl ToolExecutorTrait for FrameworkToolExecutor {
    async fn execute(&self, tool_id: &str, params: Value, context: &ToolCallContext) -> Result<Value> {
        let result = Self::execute(self, tool_id, params, context).await?;

        if result.success {
            Ok(result.data)
        } else {
            Err(anyhow::anyhow!(
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ))
        }
    }

    fn can_execute(&self, tool_id: &str) -> bool {
        Self::supported_tool_ids().contains(&tool_id)
    }

    fn supported_tools(&self) -> Vec<String> {
        Self::supported_tool_ids()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}

// Tests moved to tests/infrastructure_framework_tools.rs
