use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Stateless virtual company worker")]
pub struct AppConfig {
    #[arg(long, env = "MATRIX_HOMESERVER")]
    pub matrix_homeserver: String,

    #[arg(long, env = "MATRIX_ACCESS_TOKEN")]
    pub matrix_access_token: String,

    #[arg(long, env = "MATRIX_ROOM_ID")]
    pub matrix_room_id: String,

    #[arg(long, env = "OPENAI_API_KEY")]
    pub openai_api_key: String,

    #[arg(long, env = "OPENAI_MODEL", default_value = "gpt-4o-mini")]
    pub openai_model: String,

    #[arg(long, env = "CONTEXT_LIMIT", default_value_t = 50)]
    pub context_limit: usize,

    #[arg(
        long,
        env = "SYSTEM_PROMPT",
        default_value = "你是运行在 Matrix 网络中的无状态虚拟公司智能体。所有记忆来自传入上下文。信息不足时明确提出所需信息。"
    )]
    pub system_prompt: String,

    #[arg(long, env = "MCP_TOOL_COMMAND", default_value = "")]
    pub mcp_tool_command: String,
}
