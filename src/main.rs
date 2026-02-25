//! ImitatorT - CLI入口

use anyhow::Result;
use imitatort_stateless_company::{CompanyBuilder, CompanyConfig, VirtualCompany};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    info!("Starting ImitatorT...");

    // 尝试从配置文件加载新配置
    let company = if let Ok(config) = load_config() {
        info!("Loaded config from company_config.yaml");
        // 使用配置文件创建新公司，保存到SQLite
        CompanyBuilder::from_config(config)?
            .build_and_save()
            .await?
    } else {
        // 尝试从已有的SQLite数据库加载
        info!("No config file found, trying to load from SQLite...");
        match VirtualCompany::from_sqlite("imitatort.db").await {
            Ok(company) => {
                info!("Loaded company from SQLite database");
                company
            }
            Err(_) => {
                info!("No existing database found, using default test config");
                let config = CompanyConfig::test_config();
                CompanyBuilder::from_config(config)?
                    .build_and_save()
                    .await?
            }
        }
    };

    // 运行虚拟公司
    company.run().await?;

    Ok(())
}

/// 加载配置文件
fn load_config() -> Result<CompanyConfig> {
    // 尝试从 YAML 文件加载配置
    if let Ok(content) = std::fs::read_to_string("company_config.yaml") {
        let config: CompanyConfig = serde_yaml::from_str(&content)?;
        return Ok(config);
    }

    Err(anyhow::anyhow!("Config file not found"))
}
