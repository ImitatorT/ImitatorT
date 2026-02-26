//! Tool 注册表
//!
//! 提供工具的多级分类管理和查询能力

use anyhow::{Context, Result};
use dashmap::DashMap;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::debug;

use crate::domain::tool::{CategoryPath, Tool};

/// 工具注册表 - 支持多级分类查询
pub struct ToolRegistry {
    /// 所有工具按ID存储（快速查找）
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

        debug!("Registered tool: {} in category: {}", tool_id, category.to_path_string());
        Ok(())
    }

    /// 通过ID获取工具
    pub fn get(&self, id: &str) -> Option<Tool> {
        self.tools.get(id).map(|t| t.clone())
    }

    /// 检查工具是否存在
    pub fn contains(&self, id: &str) -> bool {
        self.tools.contains_key(id)
    }

    /// 通过分类路径查找工具
    /// - "file" -> 所有file分类下的工具（包括子分类）
    /// - "file/read" -> 仅file/read分类下的工具
    pub async fn find_by_category(&self, path: &str) -> Vec<Tool> {
        let path = CategoryPath::from_str(path);
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

    /// 递归收集分类节点下的所有工具ID
    fn collect_tool_ids(node: &CategoryNode, result: &mut Vec<String>) {
        result.extend(node.tools.iter().cloned());
        for child in node.children.values() {
            Self::collect_tool_ids(child, result);
        }
    }

    /// 获取直接属于某分类的工具（不包括子分类）
    pub async fn find_direct_by_category(&self, path: &str) -> Vec<Tool> {
        let path = CategoryPath::from_str(path);
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
        let path = CategoryPath::from_str(path);
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
        Self::collect_category_paths(&*root, String::new(), &mut paths);
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
    /// 直接属于该分类的工具ID
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

    /// 获取直接属于该分类的工具ID
    pub fn tools(&self) -> &[String] {
        &self.tools
    }
}
