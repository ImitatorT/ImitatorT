//! 思过崖线组织架构管理

use std::sync::Arc;
use tokio::sync::broadcast;

use crate::domain::{Agent, Group, Message, Organization, Role, LLMConfig, Department, MessageTarget};
use crate::core::store::Store;

/// 思过崖线管理器
pub struct GuiltyLineManager {
    store: Arc<dyn Store>,
    message_tx: broadcast::Sender<Message>,
}

impl GuiltyLineManager {
    pub fn new(store: Arc<dyn Store>, message_tx: broadcast::Sender<Message>) -> Self {
        Self { store, message_tx }
    }

    /// 初始化思过崖线架构
    /// 这将在组织架构中创建一个特殊的"思过崖线"部门，并添加相应的Agent
    pub async fn initialize_guilty_line(&self, org: &mut Organization) -> Result<(), Box<dyn std::error::Error>> {
        // 添加思过崖线部门
        let guilty_dept = Department {
            id: "guilty_line".to_string(),
            name: "思过崖线".to_string(),
            parent_id: None,
            leader_id: Some("guilty_chairman".to_string()), // 集团主席将成为负责人
        };

        org.add_department(guilty_dept);

        // 注意：实际的Agent添加会在用户注册后动态进行
        Ok(())
    }

    /// 当新用户注册时，根据其职位将其添加到思过崖线
    pub async fn add_user_to_guilty_line(&self, user_id: &str, position: &crate::domain::user::Position) -> Result<(), Box<dyn std::error::Error>> {
        match position {
            crate::domain::user::Position::Chairman => {
                // 集团主席成为思过崖线主管
                self.make_chairman_guilty_leader(user_id).await?;
            }
            crate::domain::user::Position::Management => {
                // 管理层直接加入思过崖线
                self.add_management_to_guilty_line(user_id).await?;
            }
            _ => {
                // 普通员工不加入思过崖线
            }
        }

        Ok(())
    }

    /// 将集团主席设为思过崖线主管
    async fn make_chairman_guilty_leader(&self, user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 这里我们会更新组织架构，将用户设为思过崖线部门的负责人
        // 实现细节取决于具体的组织架构更新机制

        // 创建或更新主席的Agent信息，使其成为思过崖线的领导
        let mut org = self.store.load_organization().await?;

        // 查找对应的Agent并更新其部门归属
        if let Some(agent) = org.agents.iter_mut().find(|a| a.id == user_id) {
            agent.department_id = Some("guilty_line".to_string());
        }

        self.store.save_organization(&org).await?;

        Ok(())
    }

    /// 将管理层加入思过崖线
    async fn add_management_to_guilty_line(&self, user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut org = self.store.load_organization().await?;

        // 查找对应的Agent并更新其部门归属
        if let Some(agent) = org.agents.iter_mut().find(|a| a.id == user_id) {
            agent.department_id = Some("guilty_line".to_string());
        }

        self.store.save_organization(&org).await?;

        Ok(())
    }

    /// 自动创建"思过崖线"群聊并添加相关人员
    pub async fn create_guilty_line_group_chat(&self, highest_level_agent_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        // 创建思过崖线群聊，包含：
        // 1. 集团主席 (00001)
        // 2. 所有管理层成员
        // 3. 用户自定义架构的最高级成员 (highest_level_agent_id)

        let users = self.store.load_users().await?;

        let mut members = vec![highest_level_agent_id.to_string()];

        // 添加集团主席和所有管理层成员
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
            name: "思过崖线".to_string(),
            creator_id: "system".to_string(), // 系统创建
            members,
            created_at: chrono::Utc::now().timestamp(),
        };

        self.store.save_group(&group).await?;

        Ok(group.id)
    }

    /// 将用户自定义架构的最高级成员加入思过崖线群聊
    pub async fn add_highest_level_to_guilty_group(&self, agent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut groups = self.store.load_groups().await?;

        // 查找思过崖线群聊
        if let Some(group) = groups.iter_mut().find(|g| g.id == "guilty_line_group") {
            if !group.members.contains(&agent_id.to_string()) {
                group.members.push(agent_id.to_string());
                self.store.save_group(group).await?;
            }
        }

        Ok(())
    }
}