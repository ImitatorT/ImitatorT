//! 日志模块 - 提供结构化日志、请求追踪和性能指标
//!
//! 特性：
//! - 支持人类可读和 JSON 两种格式
//! - 请求追踪 ID，贯穿单次执行流程
//! - 敏感信息自动脱敏
//! - 性能指标记录
//! - 可配置的日志级别和目标

use std::fmt;
use std::time::Instant;
use tracing::{field, Event, Level, Subscriber};
use tracing_subscriber::{
    fmt::{format::Writer, FmtContext, FormatEvent, FormatFields},
    layer::SubscriberExt,
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter,
};
use uuid::Uuid;

/// 日志格式类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogFormat {
    /// 人类可读格式（带颜色）
    Pretty,
    /// 紧凑单行格式
    Compact,
    /// JSON 结构化格式（适合日志收集系统）
    Json,
}

impl std::str::FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pretty" => Ok(LogFormat::Pretty),
            "compact" => Ok(LogFormat::Compact),
            "json" => Ok(LogFormat::Json),
            _ => Err(format!("unknown log format: {}", s)),
        }
    }
}

/// 日志配置
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// 日志格式
    pub format: LogFormat,
    /// 是否启用颜色（仅 Pretty 格式有效）
    pub enable_color: bool,
    /// 是否显示目标模块
    pub show_target: bool,
    /// 是否显示线程 ID
    pub show_thread_id: bool,
    /// 是否显示线程名称
    pub show_thread_names: bool,
    /// 是否显示文件名和行号
    pub show_file: bool,
    /// 是否显示时间
    pub show_time: bool,
    /// 时间格式（RFC3339 或自定义）
    pub time_format: TimeFormat,
    /// 请求 ID 头名称
    #[allow(dead_code)]
    pub request_id_header: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Pretty,
            enable_color: true,
            show_target: true,
            show_thread_id: false,
            show_thread_names: false,
            show_file: true,
            show_time: true,
            time_format: TimeFormat::Rfc3339,
            request_id_header: "request_id".to_string(),
        }
    }
}

/// 时间格式选项
#[derive(Debug, Clone)]
pub enum TimeFormat {
    /// RFC3339 格式
    Rfc3339,
    /// 自定义格式（chrono 格式字符串）
    #[allow(dead_code)]
    Custom(String),
}

/// 初始化日志系统
///
/// # 环境变量
/// - `RUST_LOG`: 日志级别过滤（如 `info`, `debug`, `warn,matrix=trace`）
/// - `LOG_FORMAT`: 日志格式（`pretty`, `compact`, `json`）
///
/// # 示例
/// ```
/// use imitatort_stateless_company::logger::{init, LogConfig, LogFormat};
///
/// let config = LogConfig {
///     format: LogFormat::Json,
///     show_target: false,
///     ..Default::default()
/// };
/// init(config);
/// ```
pub fn init(config: LogConfig) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = tracing_subscriber::registry().with(env_filter);

    match config.format {
        LogFormat::Pretty => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .event_format(PrettyFormatter::new(config.clone()))
                .fmt_fields(PrettyFields::new(config.show_target));
            subscriber.with(fmt_layer).init();
        }
        LogFormat::Compact => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .compact()
                .with_target(config.show_target)
                .with_thread_ids(config.show_thread_id)
                .with_thread_names(config.show_thread_names)
                .with_file(config.show_file)
                .with_line_number(config.show_file)
                .with_ansi(config.enable_color);
            subscriber.with(fmt_layer).init();
        }
        LogFormat::Json => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_target(config.show_target)
                .with_thread_ids(config.show_thread_id)
                .with_thread_names(config.show_thread_names)
                .with_file(config.show_file)
                .with_line_number(config.show_file)
                .with_current_span(true)
                .with_span_list(true);
            subscriber.with(fmt_layer).init();
        }
    }
}

/// 请求追踪上下文
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// 请求唯一 ID
    pub request_id: String,
    /// 请求开始时间
    pub start_time: Instant,
    /// 额外上下文数据
    pub metadata: std::collections::HashMap<String, String>,
}

impl RequestContext {
    /// 创建新的请求上下文
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// 获取已流逝的时间
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 使用指定 ID 创建请求上下文
    #[allow(dead_code)]
    pub fn with_id(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            start_time: Instant::now(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建带有请求 ID 的 span
#[macro_export]
macro_rules! request_span {
    ($ctx:expr) => {
        tracing::info_span!(
            "request",
            request_id = %$ctx.request_id,
            duration_ms = tracing::field::Empty,
        )
    };
}

/// 性能计时器 - 自动记录执行时间
pub struct Timer {
    name: String,
    start: Instant,
    request_id: Option<String>,
}

impl Timer {
    /// 创建新的计时器
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
            request_id: None,
        }
    }

    /// 关联到请求上下文
    pub fn with_context(mut self, ctx: &RequestContext) -> Self {
        self.request_id = Some(ctx.request_id.clone());
        self
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        match &self.request_id {
            Some(req_id) => {
                tracing::info!(
                    target: "metrics",
                    request_id = %req_id,
                    operation = %self.name,
                    elapsed_ms = %format!("{:.2}", elapsed_ms),
                    "operation completed"
                );
            }
            None => {
                tracing::info!(
                    target: "metrics",
                    operation = %self.name,
                    elapsed_ms = %format!("{:.2}", elapsed_ms),
                    "operation completed"
                );
            }
        }
    }
}

/// 敏感信息脱敏工具
pub struct Sanitizer;

impl Sanitizer {
    /// 脱敏 API 密钥 - 只保留前 8 位和后 4 位
    pub fn api_key(key: &str) -> String {
        if key.len() <= 16 {
            return "***".to_string();
        }
        format!("{}...{}", &key[..8], &key[key.len() - 4..])
    }

    /// 脱敏 Token - 完全隐藏
    #[allow(dead_code)]
    pub fn token(_token: &str) -> String {
        "***TOKEN***".to_string()
    }

    /// 脱敏 Matrix 访问令牌
    #[allow(dead_code)]
    pub fn matrix_token(token: &str) -> String {
        if token.len() <= 12 {
            return "***".to_string();
        }
        format!("{}...{}", &token[..6], &token[token.len() - 4..])
    }
}

/// 自定义美观格式器
pub struct PrettyFormatter {
    config: LogConfig,
}

impl PrettyFormatter {
    pub fn new(config: LogConfig) -> Self {
        Self { config }
    }
}

impl<S, N> FormatEvent<S, N> for PrettyFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        // 时间戳
        if self.config.show_time {
            match &self.config.time_format {
                TimeFormat::Rfc3339 => {
                    write!(writer, "{} ", chrono::Local::now().to_rfc3339())?;
                }
                TimeFormat::Custom(format) => {
                    write!(writer, "{} ", chrono::Local::now().format(format))?;
                }
            }
        }

        // 日志级别（带颜色）
        let level = event.metadata().level();
        if self.config.enable_color {
            match *level {
                Level::ERROR => write!(writer, "\x1b[31m[ERROR]\x1b[0m ")?, // 红色
                Level::WARN => write!(writer, "\x1b[33m[WARN]\x1b[0m ")?,   // 黄色
                Level::INFO => write!(writer, "\x1b[32m[INFO]\x1b[0m ")?,   // 绿色
                Level::DEBUG => write!(writer, "\x1b[34m[DEBUG]\x1b[0m ")?, // 蓝色
                Level::TRACE => write!(writer, "\x1b[35m[TRACE]\x1b[0m ")?, // 紫色
            }
        } else {
            write!(writer, "[{}] ", level)?;
        }

        // 目标模块
        if self.config.show_target {
            write!(writer, "{} ", event.metadata().target())?;
        }

        // 文件和行号
        if self.config.show_file {
            if let Some(file) = event.metadata().file() {
                write!(writer, "({}", file)?;
                if let Some(line) = event.metadata().line() {
                    write!(writer, ":{}", line)?;
                }
                write!(writer, ") ")?;
            }
        }

        // 请求 ID（如果在 span 中）
        if let Some(span) = ctx.lookup_current() {
            if let Some(request_id) = span.extensions().get::<String>() {
                if self.config.enable_color {
                    write!(writer, "\x1b[90m[{}]\x1b[0m ", request_id)?;
                } else {
                    write!(writer, "[{}] ", request_id)?;
                }
            }
        }

        // 消息内容
        ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

/// 自定义字段格式化
pub struct PrettyFields;

impl PrettyFields {
    pub fn new(_show_target: bool) -> Self {
        Self
    }
}

impl FormatFields<'_> for PrettyFields {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        writer: Writer<'_>,
        fields: R,
    ) -> std::fmt::Result {
        let mut visitor = FieldVisitor {
            writer,
            result: Ok(()),
        };
        fields.record(&mut visitor);
        visitor.result
    }
}

struct FieldVisitor<'a> {
    writer: Writer<'a>,
    result: std::fmt::Result,
}

impl field::Visit for FieldVisitor<'_> {
    fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.result = write!(self.writer, "{:?}", value);
        } else {
            self.result = write!(self.writer, " {}={:?}", field.name(), value);
        }
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        if field.name() == "message" {
            self.result = write!(self.writer, "{}", value);
        } else {
            self.result = write!(self.writer, " {}={}", field.name(), value);
        }
    }

    fn record_i64(&mut self, field: &field::Field, value: i64) {
        self.result = write!(self.writer, " {}={}", field.name(), value);
    }

    fn record_u64(&mut self, field: &field::Field, value: u64) {
        self.result = write!(self.writer, " {}={}", field.name(), value);
    }

    fn record_bool(&mut self, field: &field::Field, value: bool) {
        self.result = write!(self.writer, " {}={}", field.name(), value);
    }

    fn record_f64(&mut self, field: &field::Field, value: f64) {
        self.result = write!(self.writer, " {}={:.2}", field.name(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_format_parse() {
        assert_eq!("pretty".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
        assert_eq!("compact".parse::<LogFormat>().unwrap(), LogFormat::Compact);
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert!("invalid".parse::<LogFormat>().is_err());
    }

    #[test]
    fn test_request_context() {
        let ctx = RequestContext::new();
        assert!(!ctx.request_id.is_empty());
        assert!(ctx.elapsed().as_nanos() > 0);

        let ctx_with_id = RequestContext::with_id("test-123");
        assert_eq!(ctx_with_id.request_id, "test-123");

        let ctx_with_meta = RequestContext::new()
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");
        assert_eq!(
            ctx_with_meta.metadata.get("key1"),
            Some(&"value1".to_string())
        );
    }

    #[test]
    fn test_sanitizer_api_key() {
        let key = "sk-abcdefghijklmnopqrstuvwxyz123456";
        let sanitized = Sanitizer::api_key(key);
        assert!(sanitized.starts_with("sk-abcde"));
        assert!(sanitized.ends_with("3456"));
        assert!(sanitized.contains("..."));

        let short_key = "short";
        assert_eq!(Sanitizer::api_key(short_key), "***");
    }

    #[test]
    fn test_sanitizer_matrix_token() {
        let token = "syt_YWxpY2U_example_long_token_1234567890abcdef";
        let sanitized = Sanitizer::matrix_token(token);
        assert!(sanitized.contains("..."));
        assert!(sanitized.starts_with("syt_YW"));

        let short_token = "short";
        assert_eq!(Sanitizer::matrix_token(short_token), "***");
    }

    #[test]
    fn test_sanitizer_token() {
        assert_eq!(Sanitizer::token("any_token"), "***TOKEN***");
    }

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert_eq!(config.format, LogFormat::Pretty);
        assert!(config.enable_color);
        assert!(config.show_target);
        assert!(!config.show_thread_id);
        assert!(config.show_file);
        assert!(config.show_time);
    }

    #[test]
    fn test_timer_new() {
        let timer = Timer::new("test_operation");
        assert_eq!(timer.name, "test_operation");
        assert!(timer.request_id.is_none());

        let ctx = RequestContext::with_id("req-123");
        let timer_with_ctx = Timer::new("op").with_context(&ctx);
        assert_eq!(timer_with_ctx.request_id, Some("req-123".to_string()));
    }
}
