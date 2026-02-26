//! 测试工具和辅助函数
//!
//! 提供通用的测试辅助函数和测试数据生成器

use imitatort_stateless_company::{
    Agent, Department, LLMConfig, Organization, Role, Message, MessageTarget,
    CompanyConfig, VirtualCompany
};
use std::sync::Arc;

/// 测试辅助函数集合
pub struct TestHelper;

impl TestHelper {
    /// 创建测试用的组织架构
    pub fn create_test_organization() -> Organization {
        let mut org = Organization::new();

        // 添加部门
        org.add_department(Department::top_level("tech", "技术部"));
        org.add_department(Department::top_level("hr", "人事部"));

        // 添加测试Agent
        let ceo = Agent::new(
            "ceo",
            "CEO",
            Role::simple("CEO", "你是公司的CEO，负责决策和管理。"),
            LLMConfig::openai("test-key"),
        )
        .with_department("tech");

        let employee = Agent::new(
            "employee",
            "员工",
            Role::simple("员工", "你是一名普通员工，执行日常任务。"),
            LLMConfig::openai("test-key"),
        )
        .with_department("tech");

        org.add_agent(ceo);
        org.add_agent(employee);

        org
    }

    /// 创建测试用的公司配置
    pub fn create_test_config() -> CompanyConfig {
        CompanyConfig {
            name: "测试公司".to_string(),
            organization: Self::create_test_organization(),
        }
    }

    /// 创建测试用的消息
    pub fn create_test_message(from: &str, to: &str, content: &str) -> Message {
        Message::private(from, to, content.to_string())
    }

    /// 创建测试用的群聊消息
    pub fn create_test_group_message(from: &str, group_id: &str, content: &str) -> Message {
        Message::group(from, group_id, content.to_string())
    }

    /// 创建测试用的虚拟公司
    pub async fn create_test_company() -> VirtualCompany {
        let config = Self::create_test_config();

        // 使用内存存储进行测试
        VirtualCompany::from_config(config).unwrap()
    }

    /// 创建测试用的LLM配置
    pub fn create_test_llm_config() -> LLMConfig {
        LLMConfig::openai("test-key")
    }
}

