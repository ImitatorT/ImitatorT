use imitatort_stateless_company::{
    application::autonomous::AutonomousAgent, CompanyConfig, VirtualCompany,
    domain::{Agent, LLMConfig, Role, MessageTarget}
};
use std::sync::Arc;

#[tokio::test]
async fn test_group_creation() {
    // 创建一个简单的测试配置
    let mut org = imitatort_stateless_company::Organization::new();

    // 添加一个部门
    org.add_department(imitatort_stateless_company::Department::top_level("tech", "技术部"));

    // 添加几个Agent
    let agent1 = Agent::new(
        "ceo",
        "CEO",
        Role::simple("CEO", "你是公司的CEO，负责决策和管理。"),
        LLMConfig::openai("test-key"),
    );

    let agent2 = Agent::new(
        "developer",
        "开发者",
        Role::simple("Developer", "你是开发者，负责开发工作。"),
        LLMConfig::openai("test-key"),
    );

    org.add_agent(agent1);
    org.add_agent(agent2);

    let config = CompanyConfig {
        name: "测试公司".to_string(),
        organization: org,
    };

    // 创建虚拟公司
    let company = VirtualCompany::with_store(
        config,
        Arc::new(imitatort_stateless_company::infrastructure::store::SqliteStore::new_in_memory().unwrap())
    );

    // 启动公司
    let handle = tokio::spawn(async move {
        let _ = company.run().await;
    });

    // 等待一段时间让Agent启动
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // 停止任务
    handle.abort();
}