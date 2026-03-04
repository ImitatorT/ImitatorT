//! Tool 核心层
//!
//! 包含 ToolRegistry 和 ToolExecutor 接口定义

use anyhow::{Context, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

use crate::domain::tool::{CategoryPath, Tool, ToolCallContext};
use crate::core::skill::SkillManager;

/// 工具注册表 - 支持多级分类查询
pub struct ToolRegistry {
    /// 所有工具按 ID 存储（快速查找）
    tools: DashMap<String, Tool>,
    /// 分类树根节点（层级遍历）
    category_root: RwLock<CategoryNode>,
}

impl ToolRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
            category_root: RwLock::new(CategoryNode::new("root")),
        }
    }

    /// 注册工具
    pub async fn register(&self, tool: Tool) -> Result<()> {
        let tool_id = tool.id.clone();
        let category = tool.category.clone();

        // 检查是否已存在
        if self.tools.contains_key(&tool_id) {
            return Err(anyhow::anyhow!("Tool already registered: {}", tool_id));
        }

        // 插入工具
        self.tools.insert(tool_id.clone(), tool);

        // 更新分类树
        let mut root = self.category_root.write().await;
        root.add_tool(&category, &tool_id);

        debug!(
            "Registered tool: {} in category: {}",
            tool_id,
            category.to_path_string()
        );
        Ok(())
    }

    /// 通过 ID 获取工具
    pub fn get(&self, id: &str) -> Option<Tool> {
        self.tools.get(id).map(|t| t.clone())
    }

    /// 检查工具是否存在
    pub fn contains(&self, id: &str) -> bool {
        self.tools.contains_key(id)
    }

    /// 通过分类路径查找工具
    /// - "file" -> 所有 file 分类下的工具（包括子分类）
    /// - "file/read" -> 仅 file/read 分类下的工具
    pub async fn find_by_category(&self, path: &str) -> Vec<Tool> {
        let path = CategoryPath::from_string(path);
        let root = self.category_root.read().await;

        // 查找目标分类节点
        let mut node = &*root;
        for segment in path.segments() {
            match node.children.get(segment) {
                Some(child) => node = child,
                None => return Vec::new(),
            }
        }

        // 收集该节点及其所有子节点的工具
        let mut tool_ids = Vec::new();
        Self::collect_tool_ids(node, &mut tool_ids);

        // 获取工具详情
        tool_ids
            .iter()
            .filter_map(|id| self.tools.get(id).map(|t| t.clone()))
            .collect()
    }

    /// 递归收集分类节点下的所有工具 ID
    fn collect_tool_ids(node: &CategoryNode, result: &mut Vec<String>) {
        result.extend(node.tools.iter().cloned());
        for child in node.children.values() {
            Self::collect_tool_ids(child, result);
        }
    }

    /// 获取直接属于某分类的工具（不包括子分类）
    pub async fn find_direct_by_category(&self, path: &str) -> Vec<Tool> {
        let path = CategoryPath::from_string(path);
        let root = self.category_root.read().await;

        let mut node = &*root;
        for segment in path.segments() {
            match node.children.get(segment) {
                Some(child) => node = child,
                None => return Vec::new(),
            }
        }

        node.tools
            .iter()
            .filter_map(|id| self.tools.get(id).map(|t| t.clone()))
            .collect()
    }

    /// 列出某分类下的子分类
    pub async fn list_subcategories(&self, path: &str) -> Vec<String> {
        let path = CategoryPath::from_string(path);
        let root = self.category_root.read().await;

        let mut node = &*root;
        for segment in path.segments() {
            match node.children.get(segment) {
                Some(child) => node = child,
                None => return Vec::new(),
            }
        }

        node.children.keys().cloned().collect()
    }

    /// 获取工具的分类路径
    pub fn get_tool_category(&self, tool_id: &str) -> Option<CategoryPath> {
        self.tools.get(tool_id).map(|t| t.category.clone())
    }

    /// 获取分类树（深拷贝，用于展示）
    pub async fn get_category_tree(&self) -> CategoryNode {
        let root = self.category_root.read().await;
        root.clone()
    }

    /// 获取所有工具
    pub fn list_all(&self) -> Vec<Tool> {
        self.tools.iter().map(|t| t.clone()).collect()
    }

    /// 获取所有分类路径
    pub async fn list_all_categories(&self) -> Vec<String> {
        let root = self.category_root.read().await;
        let mut paths = Vec::new();
        Self::collect_category_paths(&root, String::new(), &mut paths);
        paths
    }

    /// 递归收集所有分类路径
    fn collect_category_paths(node: &CategoryNode, prefix: String, result: &mut Vec<String>) {
        for (name, child) in &node.children {
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", prefix, name)
            };
            result.push(path.clone());
            Self::collect_category_paths(child, path, result);
        }
    }

    /// 注销工具
    pub async fn unregister(&self, tool_id: &str) -> Result<()> {
        let tool = self
            .tools
            .remove(tool_id)
            .map(|(_, t)| t)
            .context("Tool not found")?;

        let mut root = self.category_root.write().await;
        root.remove_tool(&tool.category, tool_id);

        debug!("Unregistered tool: {}", tool_id);
        Ok(())
    }

    /// 获取工具数量
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 分类树节点
#[derive(Debug, Clone)]
pub struct CategoryNode {
    pub name: String,
    /// 直接属于该分类的工具 ID
    tools: Vec<String>,
    /// 子分类
    children: HashMap<String, CategoryNode>,
}

impl CategoryNode {
    /// 创建新节点
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tools: Vec::new(),
            children: HashMap::new(),
        }
    }

    /// 添加工具到指定分类路径
    pub fn add_tool(&mut self, path: &CategoryPath, tool_id: &str) {
        let mut node = self;
        for segment in path.segments() {
            node = node
                .children
                .entry(segment.to_string())
                .or_insert_with(|| CategoryNode::new(segment));
        }
        node.tools.push(tool_id.to_string());
    }

    /// 从指定分类路径移除工具
    pub fn remove_tool(&mut self, path: &CategoryPath, tool_id: &str) {
        let mut node = self;
        for segment in path.segments() {
            match node.children.get_mut(segment) {
                Some(child) => node = child,
                None => return,
            }
        }
        node.tools.retain(|id| id != tool_id);
    }

    /// 获取子分类
    pub fn children(&self) -> &HashMap<String, CategoryNode> {
        &self.children
    }

    /// 获取直接属于该分类的工具 ID
    pub fn tools(&self) -> &[String] {
        &self.tools
    }
}

// =============================================================================
// ToolExecutor 接口定义 - 从 infrastructure/tool/mod.rs 移动而来
// =============================================================================

/// 工具执行器接口 - 由具体实现者提供
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// 执行工具调用
    ///
    /// # Arguments
    /// * `tool_id` - 工具 ID
    /// * `params` - 工具参数（JSON 对象）
    /// * `context` - 工具调用上下文
    ///
    /// # Returns
    /// 工具执行结果（JSON 值）
    async fn execute(
        &self,
        tool_id: &str,
        params: Value,
        context: &ToolCallContext,
    ) -> Result<Value>;

    /// 检查是否支持某工具
    fn can_execute(&self, tool_id: &str) -> bool;

    /// 检查是否可以在特定技能上下文中执行工具
    fn can_execute_with_skills(&self, _tool_id: &str, _skills: &[String]) -> bool {
        true // 默认实现允许所有技能
    }

    /// 获取执行器支持的所有工具 ID
    fn supported_tools(&self) -> Vec<String> {
        Vec::new()
    }
}

/// 工具调用结果
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// 是否成功
    pub success: bool,
    /// 结果数据
    pub data: Value,
    /// 错误信息（如果失败）
    pub error: Option<String>,
}

impl ToolResult {
    /// 创建成功结果
    pub fn success(data: Value) -> Self {
        Self {
            success: true,
            data,
            error: None,
        }
    }

    /// 创建失败结果
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: Value::Null,
            error: Some(msg.into()),
        }
    }
}

/// 工具执行器注册表
///
/// 管理多个执行器，根据工具 ID 路由到对应的执行器
pub struct ToolExecutorRegistry {
    executors: Vec<Box<dyn ToolExecutor>>,
    skill_manager: Arc<SkillManager>,
}

impl ToolExecutorRegistry {
    /// 创建注册表
    pub fn new(skill_manager: Arc<SkillManager>) -> Self {
        Self {
            executors: Vec::new(),
            skill_manager,
        }
    }

    /// 创建注册表（使用默认技能管理器）
    pub fn with_default_skill_manager(tool_registry: Arc<ToolRegistry>) -> Self {
        let skill_manager = Arc::new(SkillManager::new_with_tool_registry(tool_registry));
        Self::new(skill_manager)
    }

    /// 注册执行器
    pub fn register(&mut self, executor: Box<dyn ToolExecutor>) {
        self.executors.push(executor);
    }

    /// 查找可以执行指定工具的执行器
    pub fn find_executor(&self, tool_id: &str) -> Option<&dyn ToolExecutor> {
        self.executors
            .iter()
            .find(|e| e.can_execute(tool_id))
            .map(|e| e.as_ref())
    }

    /// 查找可以执行指定工具且具备相应技能的执行器
    fn find_executor_with_skills(
        &self,
        tool_id: &str,
        skills: &[String],
    ) -> Option<&dyn ToolExecutor> {
        self.executors
            .iter()
            .find(|e| e.can_execute(tool_id) && e.can_execute_with_skills(tool_id, skills))
            .map(|e| e.as_ref())
    }

    /// 执行工具调用（自动路由到合适的执行器）
    pub async fn execute(
        &self,
        tool_id: &str,
        params: Value,
        context: &ToolCallContext,
    ) -> Result<ToolResult> {
        match self.find_executor(tool_id) {
            Some(executor) => {
                let result = executor.execute(tool_id, params, context).await?;
                Ok(ToolResult::success(result))
            }
            None => Ok(ToolResult::error(format!(
                "No executor found for tool: {}",
                tool_id
            ))),
        }
    }

    /// 执行工具调用（带技能验证）
    pub async fn execute_with_skills(
        &self,
        tool_id: &str,
        params: Value,
        context: &ToolCallContext,
        caller_skills: &[String],
    ) -> Result<ToolResult> {
        // 首先检查技能权限
        if !self.skill_manager.can_call_tool(tool_id, caller_skills) {
            return Ok(ToolResult::error(format!(
                "Insufficient skills to execute tool: {}",
                tool_id
            )));
        }

        // 查找可以执行的执行器
        match self.find_executor_with_skills(tool_id, caller_skills) {
            Some(executor) => {
                let result = executor.execute(tool_id, params, context).await?;
                Ok(ToolResult::success(result))
            }
            None => Ok(ToolResult::error(format!(
                "No executor found for tool: {}",
                tool_id
            ))),
        }
    }

    /// 检查是否有执行器支持该工具
    pub fn can_execute(&self, tool_id: &str) -> bool {
        self.find_executor(tool_id).is_some()
    }

    /// 检查是否有执行器支持该工具（带技能验证）
    pub fn can_execute_with_skills(&self, tool_id: &str, skills: &[String]) -> bool {
        self.find_executor_with_skills(tool_id, skills).is_some()
            && self.skill_manager.can_call_tool(tool_id, skills)
    }

    /// 获取所有支持的工具 ID
    pub fn list_supported_tools(&self) -> Vec<String> {
        self.executors
            .iter()
            .flat_map(|e| e.supported_tools())
            .collect()
    }
}

/// 函数式工具执行器包装
///
/// Tool executor function type alias
type ToolHandlerFn = dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send>>
    + Send
    + Sync;

/// 允许将普通异步函数包装为 ToolExecutor
pub struct FnToolExecutor {
    tool_id: String,
    handler: Box<ToolHandlerFn>,
}

impl FnToolExecutor {
    /// 创建函数式执行器
    pub fn new<F, Fut>(tool_id: impl Into<String>, handler: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Value>> + Send + 'static,
    {
        Self {
            tool_id: tool_id.into(),
            handler: Box::new(move |v| Box::pin(handler(v))),
        }
    }
}

#[async_trait]
impl ToolExecutor for FnToolExecutor {
    async fn execute(
        &self,
        tool_id: &str,
        params: Value,
        _context: &ToolCallContext,
    ) -> Result<Value> {
        if tool_id != self.tool_id {
            return Err(anyhow::anyhow!(
                "Tool ID mismatch: {} != {}",
                tool_id,
                self.tool_id
            ));
        }
        (self.handler)(params).await
    }

    fn can_execute(&self, tool_id: &str) -> bool {
        tool_id == self.tool_id
    }

    fn supported_tools(&self) -> Vec<String> {
        vec![self.tool_id.clone()]
    }
}
