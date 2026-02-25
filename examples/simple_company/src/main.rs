//! 简单的公司示例
//!
//! 演示如何使用框架创建一个虚拟公司

use anyhow::Result;
use imitatort_stateless_company::{
    CompanyBuilder, CompanyConfig, Department, Agent, Role, LLMConfig, Organization,
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    info!("Starting Simple Company Example...");

    // 从配置文件加载（如果存在）
    let config = if let Ok(content) = std::fs::read_to_string("company_config.yaml") {
        info!("Loading config from file...");
        serde_yaml::from_str(&content)?
    } else {
        info!("Using embedded default config...");
        create_default_config()
    };

    // 构建并运行公司
    let company = CompanyBuilder::from_config(config).build();
    company.run().await?;

    Ok(())
}

/// 创建默认配置
fn create_default_config() -> CompanyConfig {
    let mut org = Organization::new();

    // 添加部门
    org.add_department(Department::top_level("tech", "技术部"));

    // 添加CEO
    let ceo = Agent::new(
        "ceo",
        "CEO",
        Role::simple("CEO", "你是公司的CEO，负责决策和管理。"),
        LLMConfig::openai("test-key"), // 需要替换为真实API key
    );
    org.add_agent(ceo);

    CompanyConfig {
        name: "Simple Tech Company".to_string(),
        organization: org,
    }
}
