//! 配置管理

use serde::{Deserialize, Serialize};

use crate::domain::{Agent, Department, LLMConfig, Organization, Role};

/// 公司配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyConfig {
    pub name: String,
    pub organization: Organization,
}

impl CompanyConfig {
    /// 创建简单的测试配置
    pub fn test_config() -> Self {
        let mut org = Organization::new();

        // Add department
        org.add_department(Department::top_level("tech", "Technology Department"));

        // 从环境变量获取配置，提供更合理的默认值
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
            eprintln!("警告: OPENAI_API_KEY 环境变量未设置，使用测试密钥");
            "sk-your-api-key-here".to_string()
        });
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        // 添加Agent
        let agent1 = Agent::new(
            "ceo",
            "CEO",
            Role::simple(
                "CEO",
                "You are the CEO of the company, responsible for decision-making and management.",
            ),
            LLMConfig {
                api_key,
                model,
                base_url,
            },
        );

        org.add_agent(agent1);

        Self {
            name: "Test Company".to_string(),
            organization: org,
        }
    }
}

impl Default for CompanyConfig {
    fn default() -> Self {
        Self::test_config()
    }
}
