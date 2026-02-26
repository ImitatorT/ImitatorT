//! 内存存储实现
//!
//! 默认的存储实现，数据仅在内存中，重启后丢失

use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::domain::{Group, Message, MessageTarget, Organization};

use super::{MessageFilter, Store};

/// 内存存储
///
/// 使用内存数据结构存储所有数据，适合测试和无需持久化的场景
pub struct MemoryStore {
    organization: RwLock<Option<Organization>>,
    groups: RwLock<HashMap<String, Group>>,
    messages: RwLock<Vec<Message>>,
}

impl MemoryStore {
    /// 创建新的内存存储
    pub fn new() -> Self {
        Self {
            organization: RwLock::new(None),
            groups: RwLock::new(HashMap::new()),
            messages: RwLock::new(Vec::new()),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn save_organization(&self, org: &Organization) -> Result<()> {
        let mut stored = self.organization.write().await;
        *stored = Some(org.clone());
        Ok(())
    }

    async fn load_organization(&self) -> Result<Organization> {
        let stored = self.organization.read().await;
        Ok(stored.clone().unwrap_or_default())
    }

    async fn save_group(&self, group: &Group) -> Result<()> {
        let mut groups = self.groups.write().await;
        groups.insert(group.id.clone(), group.clone());
        Ok(())
    }

    async fn load_groups(&self) -> Result<Vec<Group>> {
        let groups = self.groups.read().await;
        Ok(groups.values().cloned().collect())
    }

    async fn delete_group(&self, group_id: &str) -> Result<()> {
        let mut groups = self.groups.write().await;
        groups.remove(group_id);
        Ok(())
    }

    async fn save_message(&self, message: &Message) -> Result<()> {
        let mut messages = self.messages.write().await;
        messages.push(message.clone());
        Ok(())
    }

    async fn save_messages(&self, new_messages: &[Message]) -> Result<()> {
        let mut messages = self.messages.write().await;
        messages.extend(new_messages.iter().cloned());
        Ok(())
    }

    async fn load_messages(&self, filter: MessageFilter) -> Result<Vec<Message>> {
        let messages = self.messages.read().await;

        let mut result: Vec<Message> = messages
            .iter()
            .filter(|m| {
                // 发送者过滤
                if let Some(ref from) = filter.from {
                    if m.from != *from {
                        return false;
                    }
                }

                // 时间戳过滤
                if let Some(since) = filter.since {
                    if m.timestamp < since {
                        return false;
                    }
                }

                // 目标类型和接收者过滤
                match &m.to {
                    MessageTarget::Direct(agent_id) => {
                        if let Some(ref target_type) = filter.target_type {
                            if target_type != "direct" {
                                return false;
                            }
                        }
                        if let Some(ref to) = filter.to {
                            if agent_id != to {
                                return false;
                            }
                        }
                    }
                    MessageTarget::Group(group_id) => {
                        if let Some(ref target_type) = filter.target_type {
                            if target_type != "group" {
                                return false;
                            }
                        }
                        if let Some(ref to) = filter.to {
                            if group_id != to {
                                return false;
                            }
                        }
                    }
                }

                true
            })
            .cloned()
            .collect();

        // 按时间戳降序排序（最新的在前）
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // 应用数量限制
        result.truncate(filter.limit);

        Ok(result)
    }
}
