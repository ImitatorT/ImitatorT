//! Cliff of Contemplation Line Organization Management

use std::sync::Arc;
use tokio::sync::broadcast;

use crate::domain::{Agent, Group, Message, Organization, Role, LLMConfig, Department, MessageTarget};
use crate::core::store::Store;

/// Cliff of Contemplation Line Manager
pub struct GuiltyLineManager {
    store: Arc<dyn Store>,
    message_tx: broadcast::Sender<Message>,
}

impl GuiltyLineManager {
    pub fn new(store: Arc<dyn Store>, message_tx: broadcast::Sender<Message>) -> Self {
        Self { store, message_tx }
    }

    /// Initialize Cliff of Contemplation Line architecture
    /// This will create a special "Cliff of Contemplation Line" department in the organization structure and add corresponding Agents
    pub async fn initialize_guilty_line(&self, org: &mut Organization) -> Result<(), Box<dyn std::error::Error>> {
        // Add Cliff of Contemplation Line department
        let guilty_dept = Department {
            id: "guilty_line".to_string(),
            name: "Cliff of Contemplation Line".to_string(),
            parent_id: None,
            leader_id: Some("guilty_chairman".to_string()), // Corporate chairman will become the leader
        };

        org.add_department(guilty_dept);

        // 注意：实际的Agent添加会在用户注册后动态进行
        Ok(())
    }

    /// When a new user registers, add them to the Cliff of Contemplation Line based on their position
    pub async fn add_user_to_guilty_line(&self, user_id: &str, position: &crate::domain::user::Position) -> Result<(), Box<dyn std::error::Error>> {
        match position {
            crate::domain::user::Position::Chairman => {
                // Corporate chairman becomes Cliff of Contemplation Line supervisor
                self.make_chairman_guilty_leader(user_id).await?;
            }
            crate::domain::user::Position::Management => {
                // Management directly joins the Cliff of Contemplation Line
                self.add_management_to_guilty_line(user_id).await?;
            }
            _ => {
                // Regular employees do not join the Cliff of Contemplation Line
            }
        }

        Ok(())
    }

    /// Make corporate chairman the Cliff of Contemplation Line supervisor
    async fn make_chairman_guilty_leader(&self, user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Here we will update the organization structure, making the user the leader of the Cliff of Contemplation Line department
        // 实现细节取决于具体的组织架构更新机制

        // Create or update chairman's Agent information, making them the leader of the Cliff of Contemplation Line
        let mut org = self.store.load_organization().await?;

        // Find the corresponding Agent and update their department assignment
        if let Some(agent) = org.agents.iter_mut().find(|a| a.id == user_id) {
            agent.department_id = Some("guilty_line".to_string());
        }

        self.store.save_organization(&org).await?;

        Ok(())
    }

    /// 将管理层加入思过崖线
    async fn add_management_to_guilty_line(&self, user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut org = self.store.load_organization().await?;

        // Find the corresponding Agent and update their department assignment
        if let Some(agent) = org.agents.iter_mut().find(|a| a.id == user_id) {
            agent.department_id = Some("guilty_line".to_string());
        }

        self.store.save_organization(&org).await?;

        Ok(())
    }

    /// Automatically create "Cliff of Contemplation Line" group chat and add relevant personnel
    pub async fn create_guilty_line_group_chat(&self, highest_level_agent_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Create Cliff of Contemplation Line group chat, including:
        // 1. Corporate chairman (00001)
        // 2. All management members
        // 3. Highest level member of user-defined architecture (highest_level_agent_id)

        let users = self.store.load_users().await?;

        let mut members = vec![highest_level_agent_id.to_string()];

        // Add corporate chairman and all management members
        for user in users {
            match user.position {
                crate::domain::user::Position::Chairman => {
                    members.push(user.id.clone());
                }
                crate::domain::user::Position::Management => {
                    members.push(user.id.clone());
                }
                _ => {}
            }
        }

        // 去重
        members.sort();
        members.dedup();

        let group = Group {
            id: "guilty_line_group".to_string(),
            name: "Cliff of Contemplation Line".to_string(),
            creator_id: "system".to_string(), // 系统创建
            members,
            created_at: chrono::Utc::now().timestamp(),
        };

        self.store.save_group(&group).await?;

        Ok(group.id)
    }

    /// Add the highest level member of user-defined architecture to the Cliff of Contemplation Line group chat
    pub async fn add_highest_level_to_guilty_group(&self, agent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut groups = self.store.load_groups().await?;

        // Find Cliff of Contemplation Line group chat
        if let Some(group) = groups.iter_mut().find(|g| g.id == "guilty_line_group") {
            if !group.members.contains(&agent_id.to_string()) {
                group.members.push(agent_id.to_string());
                self.store.save_group(group).await?;
            }
        }

        Ok(())
    }
}