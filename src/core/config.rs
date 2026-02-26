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

        // 添加部门
        org.add_department(Department::top_level("tech", "技术部"));

        // 从环境变量获取配置
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string());
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "qwen3-coder-plus".to_string());
        let base_url = std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "http://107.173.156.228:8317/v1".to_string());

        // 添加Agent
        let agent1 = Agent::new(
            "ceo",
            "CEO",
            Role::simple("CEO", "你是公司的CEO，负责决策和管理。"),
            LLMConfig {
                api_key,
                model,
                base_url,
            },
        );

        org.add_agent(agent1);

        Self {
            name: "测试公司".to_string(),
            organization: org,
        }
    }
}

impl Default for CompanyConfig {
    fn default() -> Self {
        Self::test_config()
    }
}
