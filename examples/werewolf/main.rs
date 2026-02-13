//! 狼人杀游戏示例
//!
//! 展示如何使用虚拟公司框架实现狼人杀游戏
//! 这是一个完全独立于框架的应用示例

use anyhow::{Context, Result};
use clap::Parser;
use imitatort_stateless_company::{AgentConfig, AppBuilder, VirtualCompany};
use tracing::{info, warn};

mod game;
mod roles;

use game::WerewolfGame;
use roles::{get_role_system_prompt, Role};

/// 命令行参数
#[derive(Parser)]
#[command(name = "werewolf")]
#[command(about = "狼人杀游戏示例 - 基于虚拟公司框架")]
struct Args {
    /// 本节点监听的地址
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    bind: String,

    /// 种子节点地址（用于发现其他 Agent）
    #[arg(short, long)]
    seed: Option<String>,

    /// 只运行特定角色的 Agent（用于分布式部署）
    #[arg(short, long)]
    role: Option<String>,

    /// 是否只运行主持人
    #[arg(long)]
    host_only: bool,
}

/// 创建狼人杀角色配置
fn create_werewolf_agents(api_key: &str, base_url: &str, model: &str) -> Vec<AgentConfig> {
    vec![
        // 主持人
        AgentConfig {
            id: "host-001".to_string(),
            name: "主持人".to_string(),
            system_prompt: get_role_system_prompt(Role::Host),
            model: model.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            metadata: serde_json::json!({
                "role": "host",
                "faction": "neutral"
            })
            .as_object()
            .unwrap()
            .clone(),
        },
        // 狼人
        AgentConfig {
            id: "werewolf-001".to_string(),
            name: "狼人1号".to_string(),
            system_prompt: get_role_system_prompt(Role::Werewolf),
            model: model.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            metadata: serde_json::json!({
                "role": "werewolf",
                "faction": "werewolf"
            })
            .as_object()
            .unwrap()
            .clone(),
        },
        AgentConfig {
            id: "werewolf-002".to_string(),
            name: "狼人2号".to_string(),
            system_prompt: get_role_system_prompt(Role::Werewolf),
            model: model.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            metadata: serde_json::json!({
                "role": "werewolf",
                "faction": "werewolf"
            })
            .as_object()
            .unwrap()
            .clone(),
        },
        // 村民
        AgentConfig {
            id: "villager-001".to_string(),
            name: "村民1号".to_string(),
            system_prompt: get_role_system_prompt(Role::Villager),
            model: model.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            metadata: serde_json::json!({
                "role": "villager",
                "faction": "good"
            })
            .as_object()
            .unwrap()
            .clone(),
        },
        AgentConfig {
            id: "villager-002".to_string(),
            name: "村民2号".to_string(),
            system_prompt: get_role_system_prompt(Role::Villager),
            model: model.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            metadata: serde_json::json!({
                "role": "villager",
                "faction": "good"
            })
            .as_object()
            .unwrap()
            .clone(),
        },
        AgentConfig {
            id: "villager-003".to_string(),
            name: "村民3号".to_string(),
            system_prompt: get_role_system_prompt(Role::Villager),
            model: model.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            metadata: serde_json::json!({
                "role": "villager",
                "faction": "good"
            })
            .as_object()
            .unwrap()
            .clone(),
        },
        AgentConfig {
            id: "villager-004".to_string(),
            name: "村民4号".to_string(),
            system_prompt: get_role_system_prompt(Role::Villager),
            model: model.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            metadata: serde_json::json!({
                "role": "villager",
                "faction": "good"
            })
            .as_object()
            .unwrap()
            .clone(),
        },
        // 预言家
        AgentConfig {
            id: "seer-001".to_string(),
            name: "预言家".to_string(),
            system_prompt: get_role_system_prompt(Role::Seer),
            model: model.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            metadata: serde_json::json!({
                "role": "seer",
                "faction": "good"
            })
            .as_object()
            .unwrap()
            .clone(),
        },
        // 女巫
        AgentConfig {
            id: "witch-001".to_string(),
            name: "女巫".to_string(),
            system_prompt: get_role_system_prompt(Role::Witch),
            model: model.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            metadata: serde_json::json!({
                "role": "witch",
                "faction": "good"
            })
            .as_object()
            .unwrap()
            .clone(),
        },
        // 猎人
        AgentConfig {
            id: "hunter-001".to_string(),
            name: "猎人".to_string(),
            system_prompt: get_role_system_prompt(Role::Hunter),
            model: model.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            metadata: serde_json::json!({
                "role": "hunter",
                "faction": "good"
            })
            .as_object()
            .unwrap()
            .clone(),
        },
    ]
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 解析参数
    let args = Args::parse();

    // 从环境变量获取 LLM 配置
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "sk-bb-gdsc90zzy".to_string());
    let base_url =
        std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "http://localhost:8317/v1".to_string());
    let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "qwen3-coder-plus".to_string());

    info!("=== 狼人杀游戏启动 ===");
    info!("绑定地址: {}", args.bind);
    info!("LLM 模型: {}", model);

    // 使用 AppBuilder 快速搭建虚拟公司环境
    let mut builder = AppBuilder::new(format!("http://{}", args.bind))
        .bind(args.bind.parse().context("Invalid bind address")?);

    // 如果有种子节点，添加到配置
    if let Some(seed) = &args.seed {
        builder = builder.seed(seed.clone());
        info!("种子节点: {}", seed);
    }

    // 构建并启动虚拟公司
    let company = builder.build().await?;

    // 创建角色配置
    let all_agents = create_werewolf_agents(&api_key, &base_url, &model);

    // 根据参数过滤要创建的角色
    let agents_to_create: Vec<_> = if args.host_only {
        all_agents
            .into_iter()
            .filter(|a| a.id == "host-001")
            .collect()
    } else if let Some(ref role_filter) = args.role {
        all_agents
            .into_iter()
            .filter(|a| {
                a.metadata
                    .get("role")
                    .map(|r| r.as_str() == Some(&role_filter))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        all_agents
    };

    info!("将创建 {} 个 Agent", agents_to_create.len());

    // 创建所有 Agent
    for config in agents_to_create {
        company.create_agent(config).await?;
    }

    // 广播自己的存在
    let node_info = imitatort_stateless_company::AgentInfo {
        id: format!("werewolf-node-{}", args.bind.replace(':', "-")),
        name: "Werewolf Game Node".to_string(),
        endpoint: format!("http://{}", args.bind),
        capabilities: vec!["werewolf".to_string()],
        metadata: None,
    };
    company.announce_presence(&node_info).await?;

    // 如果是完整节点（非仅主持人），启动游戏
    if !args.host_only && args.role.is_none() {
        info!("启动狼人杀游戏...");

        let mut game = WerewolfGame::new(company);
        game.initialize().await?;
        game.run().await?;
    } else {
        info!("节点已启动，等待游戏指令...");
        // 保持运行
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
