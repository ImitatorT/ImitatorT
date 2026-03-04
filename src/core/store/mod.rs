//! 存储接口定义
//!
//! **设计说明**：Store trait 提供持久化能力的抽象接口
//!
//! **方法分类**：
//!
//! **核心方法（必须实现）**：
//! - `save_organization` / `load_organization` - 组织架构持久化
//! - `save_group` / `load_groups` / `delete_group` - 群聊管理
//! - `save_message` / `load_messages` - 消息存储和查询
//!
//! **便捷方法（有默认实现，可选择性重写）**：
//! - `save_messages` - 批量保存消息（默认遍历调用 save_message）
//! - `load_messages_by_agent` - 按 Agent 查询消息（默认实现组合 load_messages）
//! - `load_messages_by_group` - 按群聊查询消息（默认实现调用 load_messages）
//!
//! **扩展方法（可选实现，用于用户和邀请码管理）**：
//! - 用户管理：`save_user` / `load_user_by_username` / `load_users` / `delete_user`
//! - 邀请码管理：`save_invitation_code` / `load_invitation_code_by_code` 等
//!
//! **注意**：扩展方法提供默认空实现，子类可以选择性重写以支持相应功能

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::invitation_code::InvitationCode;
use crate::domain::{Group, Message, Organization};

/// 消息查询过滤器
#[derive(Debug, Clone, Default)]
pub struct MessageFilter {
    /// 发送者 ID
    pub from: Option<String>,
    /// 接收者 ID（私聊）或群 ID（群聊）
    pub to: Option<String>,
    /// 目标类型："direct", "group", "broadcast"
    pub target_type: Option<String>,
    /// 起始时间戳（包含）
    pub since: Option<i64>,
    /// 最大返回数量
    pub limit: usize,
}

impl MessageFilter {
    /// 创建新的过滤器
    pub fn new() -> Self {
        Self {
            limit: 100,
            ..Default::default()
        }
    }

    /// 设置发送者
    pub fn from(mut self, agent_id: impl Into<String>) -> Self {
        self.from = Some(agent_id.into());
        self
    }

    /// 设置接收者
    pub fn to(mut self, target_id: impl Into<String>) -> Self {
        self.to = Some(target_id.into());
        self
    }

    /// 设置目标类型
    pub fn target_type(mut self, type_: impl Into<String>) -> Self {
        self.target_type = Some(type_.into());
        self
    }

    /// 设置起始时间
    pub fn since(mut self, timestamp: i64) -> Self {
        self.since = Some(timestamp);
        self
    }

    /// 设置返回数量限制
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = n;
        self
    }
}

/// 存储接口
///
/// 提供组织架构、群聊、消息的持久化能力
#[async_trait]
pub trait Store: Send + Sync {
    // ==================== 核心方法（必须实现）====================

    /// 保存组织架构（完全覆盖）
    async fn save_organization(&self, org: &Organization) -> Result<()>;

    /// 加载组织架构
    ///
    /// 如果存储中没有组织架构，返回空 Organization
    async fn load_organization(&self) -> Result<Organization>;

    /// 保存群聊
    async fn save_group(&self, group: &Group) -> Result<()>;

    /// 加载所有群聊
    async fn load_groups(&self) -> Result<Vec<Group>>;

    /// 删除群聊
    async fn delete_group(&self, group_id: &str) -> Result<()>;

    /// 保存消息
    async fn save_message(&self, message: &Message) -> Result<()>;

    // ==================== 便捷方法（有默认实现）====================

    /// 批量保存消息
    async fn save_messages(&self, messages: &[Message]) -> Result<()> {
        for msg in messages {
            self.save_message(msg).await?;
        }
        Ok(())
    }

    /// 根据过滤器查询消息
    async fn load_messages(&self, filter: MessageFilter) -> Result<Vec<Message>>;

    /// 加载与指定 Agent 相关的消息
    ///
    /// 默认实现：组合查询发送和接收的消息
    async fn load_messages_by_agent(&self, agent_id: &str, limit: usize) -> Result<Vec<Message>> {
        // 查询从指定 Agent 发送的消息
        let from_filter = MessageFilter::new().limit(limit).from(agent_id);
        let from_msgs = self.load_messages(from_filter).await?;

        // 查询发送给指定 Agent 的私聊消息
        let to_filter = MessageFilter::new()
            .limit(limit)
            .to(agent_id)
            .target_type("direct"); // 只查询私聊消息
        let to_msgs = self.load_messages(to_filter).await?;

        // 合并并去重
        let mut all_msgs = from_msgs;
        for msg in to_msgs {
            if !all_msgs.iter().any(|m| m.id == msg.id) {
                all_msgs.push(msg);
            }
        }

        // 按时间戳排序，最新的在前
        all_msgs.sort_by_key(|msg| std::cmp::Reverse(msg.timestamp));
        all_msgs.truncate(limit);

        Ok(all_msgs)
    }

    /// 加载指定群聊的消息
    async fn load_messages_by_group(&self, group_id: &str, limit: usize) -> Result<Vec<Message>> {
        let filter = MessageFilter::new()
            .to(group_id)
            .target_type("group")
            .limit(limit);
        self.load_messages(filter).await
    }

    // ==================== 扩展方法（可选实现）====================

    /// 保存用户
    ///
    /// 默认实现：空操作
    /// 子类可以重写以支持用户持久化
    async fn save_user(&self, _user: &crate::domain::user::User) -> Result<()> {
        Ok(())
    }

    /// 根据用户名加载用户
    ///
    /// 默认实现：返回 None
    /// 子类可以重写以支持用户查询
    async fn load_user_by_username(
        &self,
        _username: &str,
    ) -> Result<Option<crate::domain::user::User>> {
        Ok(None)
    }

    /// 加载所有用户
    ///
    /// 默认实现：返回空列表
    /// 子类可以重写以支持用户列表查询
    async fn load_users(&self) -> Result<Vec<crate::domain::user::User>> {
        Ok(vec![])
    }

    /// 删除用户
    ///
    /// 默认实现：空操作
    /// 子类可以重写以支持用户删除
    async fn delete_user(&self, _user_id: &str) -> Result<()> {
        Ok(())
    }

    /// 保存邀请码
    ///
    /// 默认实现：空操作
    /// 子类可以重写以支持邀请码持久化
    async fn save_invitation_code(&self, _code: &InvitationCode) -> Result<()> {
        Ok(())
    }

    /// 根据邀请码字符串查找邀请码
    ///
    /// 默认实现：返回 None
    /// 子类可以重写以支持邀请码验证
    async fn load_invitation_code_by_code(&self, _code: &str) -> Result<Option<InvitationCode>> {
        Ok(None)
    }

    /// 加载所有邀请码
    ///
    /// 默认实现：返回空列表
    /// 子类可以重写以支持邀请码管理
    async fn load_invitation_codes(&self) -> Result<Vec<InvitationCode>> {
        Ok(vec![])
    }

    /// 更新邀请码（主要用于标记为已使用）
    ///
    /// 默认实现：空操作
    /// 子类可以重写以支持邀请码状态更新
    async fn update_invitation_code(&self, _code: &InvitationCode) -> Result<()> {
        Ok(())
    }

    /// 根据创建者 ID 查找邀请码
    ///
    /// 默认实现：返回空列表
    /// 子类可以重写以支持邀请码查询
    async fn load_invitation_codes_by_creator(
        &self,
        _creator_id: &str,
    ) -> Result<Vec<InvitationCode>> {
        Ok(vec![])
    }
}
