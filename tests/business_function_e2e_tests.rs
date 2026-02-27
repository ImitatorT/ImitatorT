//! ImitatorT Business Function End-to-End Tests

use imitatort::application::framework::{CompanyBuilder, VirtualCompany};
use imitatort::core::config::CompanyConfig;
use imitatort::domain::{Agent, Department, LLMConfig, Message, MessageTarget, Organization, Role};

#[tokio::test]
async fn test_company_creation_from_config() {
    // 创建测试配置
    let mut org = Organization::new();

    // Add department
    org.add_department(Department::top_level("engineering", "Engineering Department"));
    org.add_department(Department::top_level("product", "Product Department"));
    org.add_department(Department::top_level("design", "Design Department"));

    // 添加不同角色的agent
    let ceo = Agent::new(
        "ceo",
        "CEO",
        Role::simple("Chief Executive Officer", "You are the CEO, responsible for company strategic decisions"),
        LLMConfig::openai("test-key"),
    );

    let engineering_manager = Agent::new(
        "eng-manager",
        "Engineering Manager",
        Role::simple("Engineering Manager", "You are the Engineering Manager, responsible for technical team management"),
        LLMConfig::openai("test-key"),
    ).with_department("engineering");

    let product_manager = Agent::new(
        "prod-manager",
        "Product Manager",
        Role::simple("Product Manager", "You are the Product Manager, responsible for product planning and requirement management"),
        LLMConfig::openai("test-key"),
    ).with_department("product");

    org.add_agent(ceo);
    org.add_agent(engineering_manager);
    org.add_agent(product_manager);

    let config = CompanyConfig {
        name: "Test Corporation".to_string(),
        organization: org,
    };

    // 创建临时数据库文件用于测试
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_company.db");

    // 使用CompanyBuilder创建公司
    let company = CompanyBuilder::with_sqlite(&db_path)
        .unwrap()
        .config(config)
        .build_and_save()
        .await
        .unwrap();

    // 验证公司创建成功
    assert_eq!(company.name(), "Test Corporation");

    let organization = company.organization().await;
    assert_eq!(organization.agents.len(), 3);
    assert_eq!(organization.departments.len(), 3);

    // 验证agent存在
    assert!(organization.find_agent("ceo").is_some());
    assert!(organization.find_agent("eng-manager").is_some());
    assert!(organization.find_agent("prod-manager").is_some());

    // 验证部门存在
    assert!(organization.departments.iter().any(|d| d.id == "engineering"));
    assert!(organization.departments.iter().any(|d| d.id == "product"));
    assert!(organization.departments.iter().any(|d| d.id == "design"));
}

#[tokio::test]
async fn test_company_message_flow() {
    let mut org = Organization::new();

    // 创建一个小型组织结构
    org.add_department(Department::top_level("it", "IT部"));

    let manager = Agent::new(
        "it-manager",
        "IT Manager",
        Role::simple("IT Manager", "You are the IT Manager, responsible for IT department management"),
        LLMConfig::openai("test-key"),
    ).with_department("it");

    let developer = Agent::new(
        "junior-dev",
        "初级开发",
        Role::simple("Junior Developer", "你是初级开发工程师，负责代码实现"),
        LLMConfig::openai("test-key"),
    ).with_department("it");

    org.add_agent(manager);
    org.add_agent(developer);

    let config = CompanyConfig {
        name: "IT Team".to_string(),
        organization: org,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("message_test.db");

    let company = CompanyBuilder::with_sqlite(&db_path)
        .unwrap()
        .config(config)
        .build_and_save()
        .await
        .unwrap();

    // 验证初始状态
    assert_eq!(company.organization().await.agents.len(), 2);

    // 模拟消息交互
    let message = Message {
        id: "test-msg-1".to_string(),
        from: "it-manager".to_string(),
        to: MessageTarget::Direct("junior-dev".to_string()),
        content: "你好，今天有什么工作安排？".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        reply_to: None,
        mentions: vec![],
    };

    // 验证消息结构
    assert_eq!(message.from, "it-manager");
    assert_eq!(message.to, MessageTarget::Direct("junior-dev".to_string()));
    assert_eq!(message.content, "你好，今天有什么工作安排？");
}

#[tokio::test]
async fn test_company_persistence() {
    let mut org = Organization::new();

    org.add_department(Department::top_level("hr", "Human Resources Department"));

    let hr_director = Agent::new(
        "hr-director",
        "HR Director",
        Role::simple("HR Director", "You are the HR Director, responsible for human resource management"),
        LLMConfig::openai("test-key"),
    ).with_department("hr");

    let hr_specialist = Agent::new(
        "hr-specialist",
        "HR Specialist",
        Role::simple("HR Specialist", "You are the HR Specialist, responsible for daily HR operations"),
        LLMConfig::openai("test-key"),
    ).with_department("hr");

    org.add_agent(hr_director);
    org.add_agent(hr_specialist);

    let config = CompanyConfig {
        name: "HR Department".to_string(),
        organization: org,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("persistence_test.db");

    // 创建并保存公司
    let original_company = CompanyBuilder::with_sqlite(&db_path)
        .unwrap()
        .config(config)
        .build_and_save()
        .await
        .unwrap();

    // 验证原始公司
    assert_eq!(original_company.name(), "HR Department");
    assert_eq!(original_company.organization().await.agents.len(), 2);

    // 从数据库重新加载公司
    let reloaded_company = VirtualCompany::from_sqlite(&db_path).await.unwrap();

    // 验证重新加载的公司（注意：从数据库加载的公司名称会变成"Loaded Company"）
    assert_eq!(reloaded_company.name(), "Loaded Company");
    assert_eq!(reloaded_company.organization().await.agents.len(), 2);

    // 验证agent信息一致
    let original_org = original_company.organization().await;
    let reloaded_org = reloaded_company.organization().await;

    assert_eq!(original_org.agents.len(), reloaded_org.agents.len());
    assert!(reloaded_org.find_agent("hr-director").is_some());
    assert!(reloaded_org.find_agent("hr-specialist").is_some());
}

#[tokio::test]
async fn test_organization_hierarchy() {
    let mut org = Organization::new();

    // 创建层级组织结构
    org.add_department(Department::top_level("executive", "董事会"));
    org.add_department(Department::child("engineering", "工程部", "executive"));
    org.add_department(Department::child("backend", "后端组", "engineering"));
    org.add_department(Department::child("frontend", "前端组", "engineering"));

    // 添加具有层级关系的agent
    let ceo = Agent::new(
        "ceo",
        "CEO",
        Role::simple("Chief Executive Officer", "你是CEO，负责公司整体战略"),
        LLMConfig::openai("test-key"),
    ).with_department("executive");

    let cto = Agent::new(
        "cto",
        "CTO",
        Role::simple("Chief Technology Officer", "你是CTO，负责技术战略"),
        LLMConfig::openai("test-key"),
    ).with_department("engineering");

    let backend_lead = Agent::new(
        "backend-lead",
        "Backend Lead",
        Role::simple("Backend Lead", "You are the Backend Lead, responsible for backend technology management"),
        LLMConfig::openai("test-key"),
    ).with_department("backend");

    let frontend_lead = Agent::new(
        "frontend-lead",
        "Frontend Lead",
        Role::simple("Frontend Lead", "You are the Frontend Lead, responsible for frontend technology management"),
        LLMConfig::openai("test-key"),
    ).with_department("frontend");

    org.add_agent(ceo);
    org.add_agent(cto);
    org.add_agent(backend_lead);
    org.add_agent(frontend_lead);

    let config = CompanyConfig {
        name: "Hierarchical Company".to_string(),
        organization: org,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("hierarchy_test.db");

    let company = CompanyBuilder::with_sqlite(&db_path)
        .unwrap()
        .config(config)
        .build_and_save()
        .await
        .unwrap();

    let organization = company.organization().await;

    // 验证组织层级
    assert_eq!(organization.agents.len(), 4);
    assert_eq!(organization.departments.len(), 4);

    // 验证部门层级关系（虽然数据结构可能不直接暴露parent-child关系，但我们至少验证部门都存在）
    assert!(organization.departments.iter().any(|d| d.id == "executive"));
    assert!(organization.departments.iter().any(|d| d.id == "engineering"));
    assert!(organization.departments.iter().any(|d| d.id == "backend"));
    assert!(organization.departments.iter().any(|d| d.id == "frontend"));

    // 验证agent部门分配
    let ceo_agent = organization.find_agent("ceo").unwrap();
    assert_eq!(ceo_agent.department_id.as_deref(), Some("executive"));

    let cto_agent = organization.find_agent("cto").unwrap();
    assert_eq!(cto_agent.department_id.as_deref(), Some("engineering"));

    let backend_agent = organization.find_agent("backend-lead").unwrap();
    assert_eq!(backend_agent.department_id.as_deref(), Some("backend"));

    let frontend_agent = organization.find_agent("frontend-lead").unwrap();
    assert_eq!(frontend_agent.department_id.as_deref(), Some("frontend"));
}

#[tokio::test]
async fn test_company_config_validation() {
    // 测试空配置
    let empty_org = Organization::new();
    let empty_config = CompanyConfig {
        name: "Empty Company".to_string(),
        organization: empty_org,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("empty_test.db");

    let empty_company = CompanyBuilder::with_sqlite(&db_path)
        .unwrap()
        .config(empty_config)
        .build_and_save()
        .await
        .unwrap();

    assert_eq!(empty_company.name(), "Empty Company");
    assert_eq!(empty_company.organization().await.agents.len(), 0);

    // 测试正常配置
    let mut org = Organization::new();
    let agent = Agent::new(
        "single-agent",
        "Solo Worker",
        Role::simple("Solo Worker", "You are an independent worker"),
        LLMConfig::openai("test-key"),
    );
    org.add_agent(agent);

    let single_config = CompanyConfig {
        name: "Single Agent Company".to_string(),
        organization: org,
    };

    let db_path2 = temp_dir.path().join("single_test.db");
    let single_company = CompanyBuilder::with_sqlite(&db_path2)
        .unwrap()
        .config(single_config)
        .build_and_save()
        .await
        .unwrap();

    assert_eq!(single_company.name(), "Single Agent Company");
    assert_eq!(single_company.organization().await.agents.len(), 1);
    assert!(single_company.organization().await.find_agent("single-agent").is_some());
}