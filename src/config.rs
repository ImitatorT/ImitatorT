use clap::Parser;

/// 输出模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputMode {
    /// Matrix 前端
    Matrix,
    /// 命令行输出
    Cli,
    /// A2A 协议
    A2A,
    /// 混合模式（Matrix + A2A）
    Hybrid,
}

impl std::str::FromStr for OutputMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "matrix" => Ok(OutputMode::Matrix),
            "cli" => Ok(OutputMode::Cli),
            "a2a" => Ok(OutputMode::A2A),
            "hybrid" => Ok(OutputMode::Hybrid),
            _ => Err(format!("Unknown output mode: {}", s)),
        }
    }
}

impl std::fmt::Display for OutputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputMode::Matrix => write!(f, "matrix"),
            OutputMode::Cli => write!(f, "cli"),
            OutputMode::A2A => write!(f, "a2a"),
            OutputMode::Hybrid => write!(f, "hybrid"),
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(
    author,
    version,
    about = "Stateless virtual company worker with A2A protocol"
)]
pub struct AppConfig {
    /// 输出模式: matrix, cli, a2a, hybrid
    #[arg(long, env = "OUTPUT_MODE", default_value = "cli")]
    pub output_mode: OutputMode,

    // Matrix 配置（在 matrix/hybrid 模式下需要）
    #[arg(long, env = "MATRIX_HOMESERVER")]
    pub matrix_homeserver: Option<String>,

    #[arg(long, env = "MATRIX_ACCESS_TOKEN")]
    pub matrix_access_token: Option<String>,

    #[arg(long, env = "MATRIX_ROOM_ID")]
    pub matrix_room_id: Option<String>,

    // A2A 配置
    /// Agent ID
    #[arg(long, env = "AGENT_ID", default_value = "agent-001")]
    pub agent_id: String,

    /// Agent 名称
    #[arg(long, env = "AGENT_NAME", default_value = "Virtual Agent")]
    pub agent_name: String,

    /// A2A 默认目标 Agent ID
    #[arg(long, env = "A2A_TARGET_AGENT")]
    pub a2a_target_agent: Option<String>,

    /// 注册为 Peer 的其他 Agents（逗号分隔的 Agent ID 列表）
    #[arg(long, env = "A2A_PEER_AGENTS")]
    pub a2a_peer_agents: Option<String>,

    // LLM 配置
    #[arg(long, env = "OPENAI_API_KEY")]
    pub openai_api_key: String,

    #[arg(long, env = "OPENAI_MODEL", default_value = "gpt-4o-mini")]
    pub openai_model: String,

    // 存储配置
    /// 消息存储类型: memory, persistent
    #[arg(long, env = "STORE_TYPE", default_value = "memory")]
    pub store_type: StoreType,

    /// 持久化存储路径（仅在 persistent 存储类型下使用）
    #[arg(long, env = "STORE_PATH", default_value = "./data")]
    pub store_path: String,

    /// 上下文消息数量限制
    #[arg(long, env = "CONTEXT_LIMIT", default_value_t = 50)]
    pub context_limit: usize,

    /// 存储消息数量上限
    #[arg(long, env = "STORE_MAX_SIZE", default_value_t = 1000)]
    pub store_max_size: usize,

    /// 系统提示词
    #[arg(
        long,
        env = "SYSTEM_PROMPT",
        default_value = "你是虚拟公司中的无状态智能体。所有记忆来自传入上下文。信息不足时明确提出所需信息。你可以使用工具来执行命令或获取网页内容。"
    )]
    pub system_prompt: String,

    /// 输入消息（CLI 模式单次执行时使用）
    #[arg(short, long, env = "INPUT_MESSAGE")]
    pub input_message: Option<String>,

    /// 是否以交互模式运行（CLI 模式）
    #[arg(long, env = "INTERACTIVE")]
    pub interactive: bool,

    /// 是否在 CLI 模式下回显消息
    #[arg(long, env = "CLI_ECHO", default_value_t = true)]
    pub cli_echo: bool,
}

/// 存储类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StoreType {
    Memory,
    #[cfg(feature = "persistent-store")]
    Persistent,
}

impl std::str::FromStr for StoreType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "memory" => Ok(StoreType::Memory),
            #[cfg(feature = "persistent-store")]
            "persistent" => Ok(StoreType::Persistent),
            #[cfg(not(feature = "persistent-store"))]
            "persistent" => Err("persistent store requires 'persistent-store' feature".to_string()),
            _ => Err(format!("Unknown store type: {}", s)),
        }
    }
}

impl std::fmt::Display for StoreType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreType::Memory => write!(f, "memory"),
            #[cfg(feature = "persistent-store")]
            StoreType::Persistent => write!(f, "persistent"),
        }
    }
}

impl AppConfig {
    /// 验证配置的有效性
    pub fn validate(&self) -> anyhow::Result<()> {
        match self.output_mode {
            OutputMode::Matrix | OutputMode::Hybrid => {
                if self.matrix_homeserver.is_none() {
                    anyhow::bail!("MATRIX_HOMESERVER is required for matrix/hybrid output mode");
                }
                if self.matrix_access_token.is_none() {
                    anyhow::bail!("MATRIX_ACCESS_TOKEN is required for matrix/hybrid output mode");
                }
                if self.matrix_room_id.is_none() {
                    anyhow::bail!("MATRIX_ROOM_ID is required for matrix/hybrid output mode");
                }
            }
            _ => {}
        }

        if self.openai_api_key.is_empty() {
            anyhow::bail!("OPENAI_API_KEY is required");
        }

        Ok(())
    }

    /// 获取 Matrix 配置（如果可用）
    pub fn matrix_config(&self) -> Option<(&str, &str, &str)> {
        match (
            self.matrix_homeserver.as_ref(),
            self.matrix_access_token.as_ref(),
            self.matrix_room_id.as_ref(),
        ) {
            (Some(hs), Some(token), Some(room)) => {
                Some((hs.as_str(), token.as_str(), room.as_str()))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_mode_parse() {
        assert_eq!("matrix".parse::<OutputMode>().unwrap(), OutputMode::Matrix);
        assert_eq!("cli".parse::<OutputMode>().unwrap(), OutputMode::Cli);
        assert_eq!("a2a".parse::<OutputMode>().unwrap(), OutputMode::A2A);
        assert_eq!("hybrid".parse::<OutputMode>().unwrap(), OutputMode::Hybrid);
        assert!("unknown".parse::<OutputMode>().is_err());
    }

    #[test]
    fn test_store_type_parse() {
        assert_eq!("memory".parse::<StoreType>().unwrap(), StoreType::Memory);
        #[cfg(feature = "persistent-store")]
        assert_eq!(
            "persistent".parse::<StoreType>().unwrap(),
            StoreType::Persistent
        );
    }

    #[test]
    fn test_config_defaults() {
        let config = AppConfig::parse_from(["test", "--openai-api-key", "test_key"]);

        assert_eq!(config.output_mode, OutputMode::Cli);
        assert_eq!(config.agent_id, "agent-001");
        assert_eq!(config.agent_name, "Virtual Agent");
        assert_eq!(config.openai_model, "gpt-4o-mini");
        assert_eq!(config.context_limit, 50);
        assert_eq!(config.store_max_size, 1000);
        assert!(config.cli_echo);
    }

    #[test]
    fn test_config_custom_values() {
        let config = AppConfig::parse_from([
            "test",
            "--output-mode",
            "a2a",
            "--agent-id",
            "custom-agent",
            "--agent-name",
            "Custom Name",
            "--openai-api-key",
            "sk-custom",
            "--openai-model",
            "gpt-4",
            "--context-limit",
            "100",
            "--interactive",
        ]);

        assert_eq!(config.output_mode, OutputMode::A2A);
        assert_eq!(config.agent_id, "custom-agent");
        assert_eq!(config.agent_name, "Custom Name");
        assert_eq!(config.openai_api_key, "sk-custom");
        assert_eq!(config.openai_model, "gpt-4");
        assert_eq!(config.context_limit, 100);
        assert!(config.interactive);
    }

    #[test]
    fn test_matrix_config_validation() {
        // CLI 模式不需要 Matrix 配置
        let cli_config = AppConfig::parse_from([
            "test",
            "--output-mode",
            "cli",
            "--openai-api-key",
            "test_key",
        ]);
        assert!(cli_config.validate().is_ok());

        // Matrix 模式需要 Matrix 配置
        let matrix_config = AppConfig::parse_from([
            "test",
            "--output-mode",
            "matrix",
            "--openai-api-key",
            "test_key",
        ]);
        assert!(matrix_config.validate().is_err());
    }

    #[test]
    fn test_config_debug() {
        let config = AppConfig::parse_from(["test", "--openai-api-key", "test_key"]);

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("AppConfig"));
        assert!(debug_str.contains("agent-001"));
    }

    #[test]
    fn test_config_clone() {
        let config = AppConfig::parse_from(["test", "--openai-api-key", "test_key"]);

        let cloned = config.clone();
        assert_eq!(cloned.agent_id, config.agent_id);
        assert_eq!(cloned.openai_api_key, config.openai_api_key);
    }
}
