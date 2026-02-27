use imitatort_stateless_company::{
    application::autonomous::AutonomousAgent, CompanyConfig, VirtualCompany,
    domain::{Agent, LLMConfig, Role, MessageTarget}
};
use std::sync::Arc;

#[tokio::test]
async fn test_group_creation() {
    // 创建一个简单的测试配置
    let mut org = imitatort_stateless_company::Organization::new();

    // Add a department
    org.add_department(imitatort_stateless_company::Department::top_level("tech", "Technology Department"));

    // 添加几个Agent
    let agent1 = Agent::new(
        "ceo",
        "CEO",
        Role::simple("CEO", "You are the CEO of the company, responsible for decision-making and management."),
        LLMConfig::openai("test-key"),
    );

    let agent2 = Agent::new(
        "developer",
        "Developer",
        Role::simple("Developer", "You are a developer, responsible for development work."),
        LLMConfig::openai("test-key"),
    );

    org.add_agent(agent1);
    org.add_agent(agent2);

    let config = CompanyConfig {
        name: "Test Company".to_string(),
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