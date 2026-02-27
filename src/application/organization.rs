//! Organization Management Module
//!
//! Provides organization structure creation, management, and special architectures (such as Cliff of Contemplation Line)

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

    /// Initialize Cliff of Contemplation Line organization structure
    ///
    /// Cliff of Contemplation Line is a special organization structure that exists parallel to user-defined architecture
    /// - Corporate chairman becomes Cliff of Contemplation Line supervisor
    /// - Management directly joins Cliff of Contemplation Line
    /// - Create "Cliff of Contemplation Line" group chat
    /// - Automatically add highest level members of user-defined architecture to group chat
    pub async fn initialize_guilty_cliff_line(&self, _org: &mut Organization, users: &[User]) -> Result<()> {
        // Create Cliff of Contemplation Line group chat
        let guilty_cliff_group = Group {
            id: "guilty-cliff-line".to_string(),
            name: "Cliff of Contemplation Line".to_string(),
            creator_id: String::new(), // Will be set below
            members: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
        };

        // Find corporate chairman
        let chairman = users.iter().find(|user| matches!(user.position, Position::Chairman));

        if let Some(chairman_user) = chairman {
            // Add corporate chairman as group chat creator and member
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

            // Save Cliff of Contemplation Line group chat
            self.store.save_group(&updated_group).await?;
        }

        Ok(())
    }

    /// Add user to Cliff of Contemplation Line
    pub async fn add_user_to_guilty_cliff_line(&self, user_id: &str) -> Result<()> {
        // Load existing Cliff of Contemplation Line group chat
        let mut groups = self.store.load_groups().await?;
        let guilty_cliff_group = groups.iter_mut().find(|g| g.id == "guilty-cliff-line");

        if let Some(group) = guilty_cliff_group {
            // Check if user is already in the group
            if !group.members.contains(&user_id.to_string()) {
                group.members.push(user_id.to_string());
                self.store.save_group(group).await?;
            }
        } else {
            // If Cliff of Contemplation Line group chat is not found, create one
            let new_group = Group {
                id: "guilty-cliff-line".to_string(),
                name: "Cliff of Contemplation Line".to_string(),
                creator_id: user_id.to_string(),
                members: vec![user_id.to_string()],
                created_at: chrono::Utc::now().timestamp(),
            };
            self.store.save_group(&new_group).await?;
        }

        Ok(())
    }

    /// Remove user from Cliff of Contemplation Line
    pub async fn remove_user_from_guilty_cliff_line(&self, user_id: &str) -> Result<()> {
        let mut groups = self.store.load_groups().await?;
        let guilty_cliff_group = groups.iter_mut().find(|g| g.id == "guilty-cliff-line");

        if let Some(group) = guilty_cliff_group {
            group.members.retain(|member_id| member_id != user_id);
            self.store.save_group(group).await?;
        }

        Ok(())
    }

    /// Check if user is in Cliff of Contemplation Line
    pub async fn is_user_in_guilty_cliff_line(&self, user_id: &str) -> Result<bool> {
        let groups = self.store.load_groups().await?;
        if let Some(group) = groups.iter().find(|g| g.id == "guilty-cliff-line") {
            Ok(group.members.contains(&user_id.to_string()))
        } else {
            Ok(false)
        }
    }

    /// Get all Cliff of Contemplation Line members
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

        // Define high-level position keywords
        let high_level_titles = [
            "CEO", "President", "General Manager", "Chief Executive Officer", "President",
            "Director", "Supervisor", "Leader", "Founder", "Chairman", "Board Chairman"
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

    /// Add highest level members of organization structure to Cliff of Contemplation Line
    pub async fn add_highest_level_agents_to_guilty_cliff_line(&self, org: &Organization) -> Result<()> {
        let highest_level_agents = self.find_highest_level_agents(org);

        for agent_id in &highest_level_agents {
            self.add_user_to_guilty_cliff_line(agent_id).await?;
        }

        Ok(())
    }
}