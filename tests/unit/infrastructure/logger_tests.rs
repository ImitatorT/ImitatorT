//! Logger 模块单元测试

use imitatort_stateless_company::infrastructure::logger::{LogConfig, LogFormat, RequestContext, Sanitizer, Timer};

#[test]
fn test_log_format_parse() {
    assert_eq!("pretty".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
    assert_eq!("compact".parse::<LogFormat>().unwrap(), LogFormat::Compact);
    assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);

    // 大小写不敏感
    assert_eq!("PRETTY".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
    assert_eq!("Compact".parse::<LogFormat>().unwrap(), LogFormat::Compact);

    // 无效值
    assert!("invalid".parse::<LogFormat>().is_err());
    assert!("".parse::<LogFormat>().is_err());
}

#[test]
fn test_request_context() {
    let ctx = RequestContext::new();
    assert!(!ctx.request_id.is_empty());
    assert!(ctx.elapsed().as_nanos() > 0);
}

#[test]
fn test_request_context_with_id() {
    let ctx = RequestContext::with_id("test-123");
    assert_eq!(ctx.request_id, "test-123");
}

#[test]
fn test_request_context_with_metadata() {
    let ctx = RequestContext::new()
        .with_metadata("key1", "value1")
        .with_metadata("key2", "value2");
    assert_eq!(
        ctx.metadata.get("key1"),
        Some(&"value1".to_string())
    );
    assert_eq!(
        ctx.metadata.get("key2"),
        Some(&"value2".to_string())
    );
}

#[test]
fn test_request_context_clone() {
    let ctx = RequestContext::with_id("test-id");
    let cloned = ctx.clone();
    assert_eq!(ctx.request_id, cloned.request_id);
}

#[test]
fn test_sanitizer_api_key() {
    let key = "sk-abcdefghijklmnopqrstuvwxyz123456";
    let sanitized = Sanitizer::api_key(key);
    assert!(sanitized.starts_with("sk-abcde"));
    assert!(sanitized.ends_with("3456"));
    assert!(sanitized.contains("..."));

    // 短 key
    let short_key = "short";
    assert_eq!(Sanitizer::api_key(short_key), "***");

    // 空 key
    assert_eq!(Sanitizer::api_key(""), "***");
}

#[test]
fn test_sanitizer_matrix_token() {
    let token = "syt_YWxpY2U_example_long_token_1234567890abcdef";
    let sanitized = Sanitizer::matrix_token(token);
    assert!(sanitized.contains("..."));
    assert!(sanitized.starts_with("syt_YW"));

    // 短 token
    let short_token = "short";
    assert_eq!(Sanitizer::matrix_token(short_token), "***");

    // 空 token
    assert_eq!(Sanitizer::matrix_token(""), "***");
}

#[test]
fn test_sanitizer_token() {
    assert_eq!(Sanitizer::token("any_token"), "***TOKEN***");
    assert_eq!(Sanitizer::token(""), "***TOKEN***");
}

#[test]
fn test_log_config_default() {
    let config = LogConfig::default();
    assert_eq!(config.format, LogFormat::Pretty);
    assert!(config.enable_color);
    assert!(config.show_target);
    assert!(!config.show_thread_id);
    assert!(!config.show_thread_names);
    assert!(config.show_file);
    assert!(config.show_time);
}

#[test]
fn test_log_config_clone() {
    let config = LogConfig::default();
    let cloned = config.clone();
    assert_eq!(config.format, cloned.format);
    assert_eq!(config.show_target, cloned.show_target);
}

#[test]
fn test_timer_new() {
    let timer = Timer::new("test_operation");
    // Timer is created successfully
}

#[test]
fn test_timer_with_context() {
    let ctx = RequestContext::with_id("req-123");
    let timer = Timer::new("op").with_context(&ctx);
    // Timer with context is created successfully
}

#[test]
fn test_log_format_debug() {
    let format = LogFormat::Json;
    let debug_str = format!("{:?}", format);
    assert!(debug_str.contains("Json"));
}

#[test]
fn test_request_context_default() {
    let ctx: RequestContext = Default::default();
    assert!(!ctx.request_id.is_empty());
}
