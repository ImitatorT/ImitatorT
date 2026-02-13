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
        default_value = "你是运行在 Matrix 网络中的无状态虚拟公司智能体。所有记忆来自传入上下文。信息不足时明确提出所需信息。你可以使用工具来执行命令或获取网页内容。"
    )]
    pub system_prompt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // Test that default values are set correctly
        let config = AppConfig::parse_from([
            "test",
            "--matrix-homeserver",
            "http://localhost:8008",
            "--matrix-access-token",
            "test_token",
            "--matrix-room-id",
            "!test:matrix.org",
            "--openai-api-key",
            "test_key",
        ]);

        assert_eq!(config.matrix_homeserver, "http://localhost:8008");
        assert_eq!(config.matrix_access_token, "test_token");
        assert_eq!(config.matrix_room_id, "!test:matrix.org");
        assert_eq!(config.openai_api_key, "test_key");
        assert_eq!(config.openai_model, "gpt-4o-mini"); // default
        assert_eq!(config.context_limit, 50); // default
        assert!(config.system_prompt.contains("Matrix")); // default contains Chinese text
    }

    #[test]
    fn test_config_custom_values() {
        let config = AppConfig::parse_from([
            "test",
            "--matrix-homeserver",
            "https://matrix.example.com",
            "--matrix-access-token",
            "custom_token",
            "--matrix-room-id",
            "!room:example.com",
            "--openai-api-key",
            "sk-custom",
            "--openai-model",
            "gpt-4",
            "--context-limit",
            "100",
            "--system-prompt",
            "Custom prompt",
        ]);

        assert_eq!(config.matrix_homeserver, "https://matrix.example.com");
        assert_eq!(config.matrix_access_token, "custom_token");
        assert_eq!(config.matrix_room_id, "!room:example.com");
        assert_eq!(config.openai_api_key, "sk-custom");
        assert_eq!(config.openai_model, "gpt-4");
        assert_eq!(config.context_limit, 100);
        assert_eq!(config.system_prompt, "Custom prompt");
    }

    #[test]
    fn test_config_debug() {
        let config = AppConfig::parse_from([
            "test",
            "--matrix-homeserver",
            "http://localhost:8008",
            "--matrix-access-token",
            "test_token",
            "--matrix-room-id",
            "!test:matrix.org",
            "--openai-api-key",
            "test_key",
        ]);

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("AppConfig"));
        assert!(debug_str.contains("http://localhost:8008"));
    }

    #[test]
    fn test_config_clone() {
        let config = AppConfig::parse_from([
            "test",
            "--matrix-homeserver",
            "http://localhost:8008",
            "--matrix-access-token",
            "test_token",
            "--matrix-room-id",
            "!test:matrix.org",
            "--openai-api-key",
            "test_key",
        ]);

        let cloned = config.clone();
        assert_eq!(cloned.matrix_homeserver, config.matrix_homeserver);
        assert_eq!(cloned.matrix_access_token, config.matrix_access_token);
        assert_eq!(cloned.matrix_room_id, config.matrix_room_id);
        assert_eq!(cloned.openai_api_key, config.openai_api_key);
    }
}
