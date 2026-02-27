//! Organizational Structure Domain Entity
//!
//! Simplified organizational structure definition

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::Agent;

/// Organization Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub departments: Vec<Department>,
    pub agents: Vec<Agent>,
}

impl Organization {
    /// Create empty organization structure
    pub fn new() -> Self {
        Self {
            departments: vec![],
            agents: vec![],
        }
    }

    /// Create from configuration
    pub fn from_config(config: OrgConfig) -> Self {
        Self {
            departments: config.departments,
            agents: config.agents,
        }
    }

    /// Add department
    pub fn add_department(&mut self, dept: Department) {
        self.departments.push(dept);
    }

    /// Add Agent
    pub fn add_agent(&mut self, agent: Agent) {
        self.agents.push(agent);
    }

    /// Find Agent
    pub fn find_agent(&self, id: &str) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    /// Find Department
    pub fn find_department(&self, id: &str) -> Option<&Department> {
        self.departments.iter().find(|d| d.id == id)
    }

    /// Get department members
    pub fn get_department_members(&self, dept_id: &str) -> Vec<&Agent> {
        self.agents
            .iter()
            .filter(|a| a.department_id.as_ref() == Some(&dept_id.to_string()))
            .collect()
    }

    /// Get department leader
    pub fn get_department_leader(&self, dept_id: &str) -> Option<&Agent> {
        let dept = self.find_department(dept_id)?;
        dept.leader_id.as_ref().and_then(|id| self.find_agent(id))
    }

    /// Get sub-departments
    pub fn get_sub_departments(&self, parent_id: &str) -> Vec<&Department> {
        self.departments
            .iter()
            .filter(|d| d.parent_id.as_ref() == Some(&parent_id.to_string()))
            .collect()
    }

    /// Build department tree
    pub fn build_tree(&self) -> Vec<DepartmentNode> {
        let mut roots: Vec<DepartmentNode> = vec![];
        let mut children_map: HashMap<String, Vec<DepartmentNode>> = HashMap::new();

        for dept in &self.departments {
            let node = DepartmentNode {
                department: dept.clone(),
                children: vec![],
                members: self.get_department_members(&dept.id)
                    .into_iter()
                    .map(|a| a.id.clone())
                    .collect(),
            };

            if let Some(parent_id) = &dept.parent_id {
                children_map
                    .entry(parent_id.clone())
                    .or_default()
                    .push(node);
            } else {
                roots.push(node);
            }
        }

        // Attach sub-departments
        fn attach_children(node: &mut DepartmentNode, map: &HashMap<String, Vec<DepartmentNode>>) {
            if let Some(children) = map.get(&node.department.id) {
                node.children = children.clone();
                for child in &mut node.children {
                    attach_children(child, map);
                }
            }
        }

        for root in &mut roots {
            attach_children(root, &children_map);
        }

        roots
    }
}

impl Default for Organization {
    fn default() -> Self {
        Self::new()
    }
}

/// Department Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Department {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub leader_id: Option<String>,
}

impl Department {
    /// Create top-level department
    pub fn top_level(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent_id: None,
            leader_id: None,
        }
    }

    /// Create child department
    pub fn child(
        id: impl Into<String>,
        name: impl Into<String>,
        parent_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent_id: Some(parent_id.into()),
            leader_id: None,
        }
    }

    /// Set leader
    pub fn with_leader(mut self, leader_id: impl Into<String>) -> Self {
        self.leader_id = Some(leader_id.into());
        self
    }
}

/// Organization Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgConfig {
    pub departments: Vec<Department>,
    pub agents: Vec<Agent>,
}

/// Department Tree Node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentNode {
    pub department: Department,
    pub children: Vec<DepartmentNode>,
    pub members: Vec<String>,
}
