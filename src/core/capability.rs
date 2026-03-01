//! Capability 注册表
//!
//! 提供功能的多级分类管理和查询能力，兼容 MCP (Model Context Protocol)

use anyhow::{Context, Result};
use dashmap::DashMap;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::debug;

use crate::domain::capability::{Capability, CapabilityPath};

/// 功能注册表 - 支持多级分类查询
pub struct CapabilityRegistry {
    /// 所有功能按ID存储（快速查找）
    capabilities: DashMap<String, Capability>,
    /// 分类树根节点（层级遍历）
    capability_root: RwLock<CapabilityNode>,
}

impl CapabilityRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self {
            capabilities: DashMap::new(),
            capability_root: RwLock::new(CapabilityNode::new("root")),
        }
    }

    /// 注册功能
    pub async fn register(&self, capability: Capability) -> Result<()> {
        let cap_id = capability.id.clone();
        let capability_path = capability.capability_path.clone();

        // 检查是否已存在
        if self.capabilities.contains_key(&cap_id) {
            return Err(anyhow::anyhow!("Capability already registered: {}", cap_id));
        }

        // 插入功能
        self.capabilities.insert(cap_id.clone(), capability);

        // 更新分类树
        let mut root = self.capability_root.write().await;
        root.add_capability(&capability_path, &cap_id);

        debug!(
            "Registered capability: {} in path: {}",
            cap_id,
            capability_path.to_path_string()
        );
        Ok(())
    }

    /// 通过ID获取功能
    pub fn get(&self, id: &str) -> Option<Capability> {
        self.capabilities.get(id).map(|c| c.clone())
    }

    /// 检查功能是否存在
    pub fn contains(&self, id: &str) -> bool {
        self.capabilities.contains_key(id)
    }

    /// 通过分类路径查找功能
    /// - "file" -> 所有file分类下的功能（包括子分类）
    /// - "file/read" -> 仅file/read分类下的功能
    pub async fn find_by_path(&self, path: &str) -> Vec<Capability> {
        let path = CapabilityPath::from_string(path);
        let root = self.capability_root.read().await;

        // 查找目标分类节点
        let mut node = &*root;
        for segment in path.segments() {
            match node.children.get(segment) {
                Some(child) => node = child,
                None => return Vec::new(),
            }
        }

        // 收集该节点及其所有子节点的功能
        let mut capability_ids = Vec::new();
        Self::collect_capability_ids(node, &mut capability_ids);

        // 获取功能详情
        capability_ids
            .iter()
            .filter_map(|id| self.capabilities.get(id).map(|c| c.clone()))
            .collect()
    }

    /// 递归收集分类节点下的所有功能ID
    fn collect_capability_ids(node: &CapabilityNode, result: &mut Vec<String>) {
        result.extend(node.capabilities.iter().cloned());
        for child in node.children.values() {
            Self::collect_capability_ids(child, result);
        }
    }

    /// 获取直接属于某分类的功能（不包括子分类）
    pub async fn find_direct_by_path(&self, path: &str) -> Vec<Capability> {
        let path = CapabilityPath::from_string(path);
        let root = self.capability_root.read().await;

        let mut node = &*root;
        for segment in path.segments() {
            match node.children.get(segment) {
                Some(child) => node = child,
                None => return Vec::new(),
            }
        }

        node.capabilities
            .iter()
            .filter_map(|id| self.capabilities.get(id).map(|c| c.clone()))
            .collect()
    }

    /// 列出某分类下的子分类
    pub async fn list_sub_paths(&self, path: &str) -> Vec<String> {
        let path = CapabilityPath::from_string(path);
        let root = self.capability_root.read().await;

        let mut node = &*root;
        for segment in path.segments() {
            match node.children.get(segment) {
                Some(child) => node = child,
                None => return Vec::new(),
            }
        }

        node.children.keys().cloned().collect()
    }

    /// 获取功能的分类路径
    pub fn get_capability_path(&self, capability_id: &str) -> Option<CapabilityPath> {
        self.capabilities
            .get(capability_id)
            .map(|c| c.capability_path.clone())
    }

    /// 获取分类树（深拷贝，用于展示）
    pub async fn get_capability_tree(&self) -> CapabilityNode {
        let root = self.capability_root.read().await;
        root.clone()
    }

    /// 获取所有功能
    pub fn list_all(&self) -> Vec<Capability> {
        self.capabilities.iter().map(|c| c.clone()).collect()
    }

    /// 获取所有分类路径
    pub async fn list_all_paths(&self) -> Vec<String> {
        let root = self.capability_root.read().await;
        let mut paths = Vec::new();
        Self::collect_capability_paths(&root, String::new(), &mut paths);
        paths
    }

    /// 递归收集所有分类路径
    fn collect_capability_paths(node: &CapabilityNode, prefix: String, result: &mut Vec<String>) {
        for (name, child) in &node.children {
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", prefix, name)
            };
            result.push(path.clone());
            Self::collect_capability_paths(child, path, result);
        }
    }

    /// 注销功能
    pub async fn unregister(&self, capability_id: &str) -> Result<()> {
        let capability = self
            .capabilities
            .remove(capability_id)
            .map(|(_, c)| c)
            .context("Capability not found")?;

        let mut root = self.capability_root.write().await;
        root.remove_capability(&capability.capability_path, capability_id);

        debug!("Unregistered capability: {}", capability_id);
        Ok(())
    }

    /// 获取功能数量
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 分类树节点
#[derive(Debug, Clone)]
pub struct CapabilityNode {
    pub name: String,
    /// 直接属于该分类的功能ID
    capabilities: Vec<String>,
    /// 子分类
    children: HashMap<String, CapabilityNode>,
}

impl CapabilityNode {
    /// 创建新节点
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            capabilities: Vec::new(),
            children: HashMap::new(),
        }
    }

    /// 添加功能到指定分类路径
    pub fn add_capability(&mut self, path: &CapabilityPath, capability_id: &str) {
        let mut node = self;
        for segment in path.segments() {
            node = node
                .children
                .entry(segment.to_string())
                .or_insert_with(|| CapabilityNode::new(segment));
        }
        node.capabilities.push(capability_id.to_string());
    }

    /// 从指定分类路径移除功能
    pub fn remove_capability(&mut self, path: &CapabilityPath, capability_id: &str) {
        let mut node = self;
        for segment in path.segments() {
            match node.children.get_mut(segment) {
                Some(child) => node = child,
                None => return,
            }
        }
        node.capabilities.retain(|id| id != capability_id);
    }

    /// 获取子分类
    pub fn children(&self) -> &HashMap<String, CapabilityNode> {
        &self.children
    }

    /// 获取直接属于该分类的功能ID
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }
}
