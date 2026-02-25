//! Agent 领域实体
//!
//! 虚拟公司员工的基本定义

use serde::{Deserialize, Serialize};

/// Agent 唯一标识
pub type AgentId = String;

/// Agent 实体 - 统一Agent定义，唯一真理来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub role: Role,
    pub department_id: Option<String>,
    pub llm_config: LLMConfig,
}

impl Agent {
    /// 创建新的Agent
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        role: Role,
        llm_config: LLMConfig,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role,
            department_id: None,
            llm_config,
        }
    }

    /// 设置部门
    pub fn with_department(mut self, dept_id: impl Into<String>) -> Self {
        self.department_id = Some(dept_id.into());
        self
    }

    /// 生成系统提示词
    pub fn system_prompt(&self) -> String {
        self.role.system_prompt.clone()
    }
}

/// 角色定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub title: String,
    pub responsibilities: Vec<String>,
    pub expertise: Vec<String>,
    pub system_prompt: String,
}

impl Role {
    /// 创建简单角色
    pub fn simple(title: impl Into<String>, system_prompt: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            responsibilities: vec![],
            expertise: vec![],
            system_prompt: system_prompt.into(),
        }
    }

    /// 添加职责
    pub fn with_responsibilities(mut self, items: Vec<String>) -> Self {
        self.responsibilities = items;
        self
    }

    /// 添加专业领域
    pub fn with_expertise(mut self, items: Vec<String>) -> Self {
        self.expertise = items;
        self
    }
}

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub model: String,
    pub api_key: String,
    pub base_url: String,
}

impl LLMConfig {
    /// 使用OpenAI默认配置
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self {
            model: "gpt-4o-mini".to_string(),
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// 设置模型
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// 设置基础URL（用于自定义端点）
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let role = Role::simple("工程师", "你是一个软件工程师");
        let llm = LLMConfig::openai("test-key");
        let agent = Agent::new("agent-001", "张三", role, llm);

        assert_eq!(agent.id, "agent-001");
        assert_eq!(agent.name, "张三");
        assert_eq!(agent.role.title, "工程师");
    }

    #[test]
    fn test_system_prompt() {
        let role = Role::simple("研究员", "你是一个AI研究员，专注于机器学习。");
        let llm = LLMConfig::openai("test-key");
        let agent = Agent::new("a1", "李四", role, llm);

        let prompt = agent.system_prompt();
        assert!(prompt.contains("AI研究员"));
        assert!(prompt.contains("机器学习"));
    }

    #[test]
    fn test_llm_config() {
        let config = LLMConfig::openai("sk-test")
            .with_model("gpt-4")
            .with_base_url("http://localhost:8080");

        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.api_key, "sk-test");
        assert_eq!(config.base_url, "http://localhost:8080");
    }
}
