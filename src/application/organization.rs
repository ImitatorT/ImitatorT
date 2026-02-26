//! 组织架构管理模块
//!
//! 提供组织架构创建、管理以及特殊架构（如思过崖线）的功能

use anyhow::Result;
use std::sync::Arc;

use crate::domain::{Group, Organization};
use crate::core::store::Store;
use crate::domain::user::{User, Position};

/// 组织架构管理器
pub struct OrganizationManager {
    store: Arc<dyn Store>,
}

impl OrganizationManager {
    /// 创建新的组织架构管理器
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }

    /// 初始化思过崖线组织架构
    ///
    /// 思过崖线是一个特殊的组织架构，与用户自定义架构平行存在
    /// - 集团主席成为思过崖线主管
    /// - 管理层直接加入思过崖线
    /// - 创建"思过崖线"群聊
    /// - 自动将用户自定义架构的最高级成员加入群聊
    pub async fn initialize_guilty_cliff_line(&self, _org: &mut Organization, users: &[User]) -> Result<()> {
        // 创建思过崖线群聊
        let guilty_cliff_group = Group {
            id: "guilty-cliff-line".to_string(),
            name: "思过崖线".to_string(),
            creator_id: String::new(), // 将在下面设置
            members: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
        };

        // 查找集团主席
        let chairman = users.iter().find(|user| matches!(user.position, Position::Chairman));

        if let Some(chairman_user) = chairman {
            // 将集团主席添加为群聊创建者和成员
            let mut updated_group = guilty_cliff_group;
            updated_group.creator_id = chairman_user.id.clone();
            updated_group.members.push(chairman_user.id.clone());

            // 添加所有管理层成员
            for user in users.iter() {
                if matches!(user.position, Position::Management) {
                    if !updated_group.members.contains(&user.id) {
                        updated_group.members.push(user.id.clone());
                    }
                }
            }

            // 保存思过崖线群聊
            self.store.save_group(&updated_group).await?;
        }

        Ok(())
    }

    /// 添加用户到思过崖线
    pub async fn add_user_to_guilty_cliff_line(&self, user_id: &str) -> Result<()> {
        // 加载现有的思过崖线群聊
        let mut groups = self.store.load_groups().await?;
        let guilty_cliff_group = groups.iter_mut().find(|g| g.id == "guilty-cliff-line");

        if let Some(group) = guilty_cliff_group {
            // 检查用户是否已在群组中
            if !group.members.contains(&user_id.to_string()) {
                group.members.push(user_id.to_string());
                self.store.save_group(group).await?;
            }
        } else {
            // 如果没有找到思过崖线群聊，则创建一个
            let new_group = Group {
                id: "guilty-cliff-line".to_string(),
                name: "思过崖线".to_string(),
                creator_id: user_id.to_string(),
                members: vec![user_id.to_string()],
                created_at: chrono::Utc::now().timestamp(),
            };
            self.store.save_group(&new_group).await?;
        }

        Ok(())
    }

    /// 从思过崖线移除用户
    pub async fn remove_user_from_guilty_cliff_line(&self, user_id: &str) -> Result<()> {
        let mut groups = self.store.load_groups().await?;
        let guilty_cliff_group = groups.iter_mut().find(|g| g.id == "guilty-cliff-line");

        if let Some(group) = guilty_cliff_group {
            group.members.retain(|member_id| member_id != user_id);
            self.store.save_group(group).await?;
        }

        Ok(())
    }

    /// 检查用户是否在思过崖线中
    pub async fn is_user_in_guilty_cliff_line(&self, user_id: &str) -> Result<bool> {
        let groups = self.store.load_groups().await?;
        if let Some(group) = groups.iter().find(|g| g.id == "guilty-cliff-line") {
            Ok(group.members.contains(&user_id.to_string()))
        } else {
            Ok(false)
        }
    }

    /// 获取思过崖线所有成员
    pub async fn get_guilty_cliff_line_members(&self) -> Result<Vec<String>> {
        let groups = self.store.load_groups().await?;
        if let Some(group) = groups.iter().find(|g| g.id == "guilty-cliff-line") {
            Ok(group.members.clone())
        } else {
            Ok(Vec::new())
        }
    }

    /// 根据组织架构找出最高级成员（CEO、总裁等）
    pub fn find_highest_level_agents(&self, org: &Organization) -> Vec<String> {
        let mut highest_level_agents = Vec::new();

        // 定义高级职位关键词
        let high_level_titles = [
            "CEO", "总裁", "总经理", "Chief Executive Officer", "President",
            "Director", "主管", "Leader", "Founder", "董事长", "董事会主席"
        ];

        for agent in &org.agents {
            for title in &high_level_titles {
                if agent.role.title.to_uppercase().contains(&title.to_uppercase()) ||
                   agent.name.to_uppercase().contains(&title.to_uppercase()) {
                    highest_level_agents.push(agent.id.clone());
                    break;
                }
            }
        }

        highest_level_agents
    }

    /// 将组织架构的最高级成员添加到思过崖线
    pub async fn add_highest_level_agents_to_guilty_cliff_line(&self, org: &Organization) -> Result<()> {
        let highest_level_agents = self.find_highest_level_agents(org);

        for agent_id in &highest_level_agents {
            self.add_user_to_guilty_cliff_line(agent_id).await?;
        }

        Ok(())
    }
}