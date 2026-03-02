//! 虚拟公司框架 API 测试

use imitatort::application::framework::{CompanyBuilder, VirtualCompany};
use imitatort::core::config::CompanyConfig;
use imitatort::domain::{Agent, Department, LLMConfig, Organization, Role};

#[tokio::test]
async fn test_company_builder() {
    let org = Organization::new();
    let config = CompanyConfig {
        name: "Test Co".to_string(),
        organization: org,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let _company = CompanyBuilder::with_sqlite(&db_path)
        .unwrap()
        .config(config)
        .build()
        .unwrap();
    // Company created successfully
}

#[tokio::test]
async fn test_agent_creation() {
    let mut org = Organization::new();
    org.add_department(Department::top_level("tech", "技术部"));

    let agent = Agent::new(
        "test-agent",
        "测试员",
        Role::simple("测试", "你是测试员"),
        LLMConfig::openai("test"),
    );
    org.add_agent(agent);

    let config = CompanyConfig {
        name: "Test".to_string(),
        organization: org,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let company = VirtualCompany::with_sqlite(config, &db_path).unwrap();

    // 注意：不调用run()，因为需要真实的LLM
    assert_eq!(company.organization().await.agents.len(), 1);
}

#[tokio::test]
async fn test_company_with_sqlite_store() {
    // 使用临时目录
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // 创建配置
    let mut org = Organization::new();
    org.add_department(Department::top_level("tech", "技术部"));

    let agent = Agent::new(
        "ceo",
        "CEO",
        Role::simple("CEO", "你是CEO"),
        LLMConfig::openai("test"),
    );
    org.add_agent(agent);

    let config = CompanyConfig {
        name: "Test Co".to_string(),
        organization: org,
    };

    // 使用 SQLite 构建
    let company = CompanyBuilder::with_sqlite(&db_path)
        .unwrap()
        .config(config)
        .build_and_save()
        .await
        .unwrap();

    assert_eq!(company.organization().await.agents.len(), 1);

    // 从存储重新加载
    let company2 = CompanyBuilder::with_sqlite(&db_path)
        .unwrap()
        .load()
        .await
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(company2.organization().await.agents.len(), 1);
    assert!(company2.organization().await.find_agent("ceo").is_some());
}

#[tokio::test]
async fn test_company_load_from_sqlite() {
    // 使用临时目录
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // 创建配置并保存
    let mut org = Organization::new();
    org.add_department(Department::top_level("tech", "技术部"));

    let agent = Agent::new(
        "dev1",
        "开发者",
        Role::simple("Dev", "你是开发者"),
        LLMConfig::openai("test"),
    )
    .with_department("tech");
    org.add_agent(agent);

    let config = CompanyConfig {
        name: "Tech Co".to_string(),
        organization: org,
    };

    // 创建并保存
    let company = CompanyBuilder::with_sqlite(&db_path)
        .unwrap()
        .config(config)
        .build_and_save()
        .await
        .unwrap();

    assert_eq!(company.organization().await.agents.len(), 1);

    // 使用 VirtualCompany::from_sqlite 加载
    let company2 = VirtualCompany::from_sqlite(&db_path).await.unwrap();
    assert_eq!(company2.organization().await.agents.len(), 1);
    assert_eq!(
        company2
            .organization()
            .await
            .find_agent("dev1")
            .unwrap()
            .name,
        "开发者"
    );
}
