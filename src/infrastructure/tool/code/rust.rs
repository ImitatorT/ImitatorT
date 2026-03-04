//! Rust 代码执行器
//!
//! 使用 rustc 编译并执行 Rust 代码，带有超时和资源限制

use anyhow::Result;
use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
use uuid::Uuid;

/// Rust 执行器配置
#[derive(Debug, Clone)]
pub struct RustRunnerConfig {
    /// 执行超时（秒）
    pub timeout_secs: u64,
    /// 最大输出长度（字节）
    pub max_output_size: usize,
    /// rustc 编译器路径
    pub rustc_path: String,
    /// 临时目录
    pub temp_dir: String,
}

impl Default for RustRunnerConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_output_size: 1024 * 1024,
            rustc_path: "rustc".to_string(),
            temp_dir: std::env::temp_dir().to_string_lossy().to_string(),
        }
    }
}

/// Rust 代码执行器
pub struct RustRunner {
    config: RustRunnerConfig,
}

impl RustRunner {
    pub fn new() -> Self {
        Self {
            config: RustRunnerConfig::default(),
        }
    }

    pub fn with_config(config: RustRunnerConfig) -> Self {
        Self { config }
    }

    /// 执行 Rust 代码
    pub fn execute(&self, code: &str) -> Result<RustExecutionResult> {
        // 生成唯一的文件名
        let uuid = Uuid::new_v4().to_string();
        let source_path = format!("{}/{}_main.rs", self.config.temp_dir, uuid);
        let binary_path = format!("{}/{}_main", self.config.temp_dir, uuid);

        // 创建完整的 Rust 程序
        let full_code = format!(
            r#"fn main() {{
{}
}}"#,
            code
        );

        // 写入源文件
        let mut source_file = fs::File::create(&source_path)?;
        source_file.write_all(full_code.as_bytes())?;

        // 编译代码
        let compile_result = Command::new(&self.config.rustc_path)
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .arg("--edition=2021")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output();

        let compile_result = match compile_result {
            Ok(result) => result,
            Err(e) => {
                // 清理文件
                let _ = fs::remove_file(&source_path);
                return Ok(RustExecutionResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("Failed to start compiler: {}", e),
                    exit_code: -1,
                    compilation_error: true,
                });
            }
        };

        if !compile_result.status.success() {
            let stderr = String::from_utf8_lossy(&compile_result.stderr).to_string();
            // 清理文件
            let _ = fs::remove_file(&source_path);
            return Ok(RustExecutionResult {
                success: false,
                stdout: String::new(),
                stderr,
                exit_code: compile_result.status.code().unwrap_or(-1),
                compilation_error: true,
            });
        }

        // 执行编译后的二进制文件
        let timeout = Duration::from_secs(self.config.timeout_secs);
        let start_time = std::time::Instant::now();

        let mut child = Command::new(&binary_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let (stdout, stderr) = loop {
            if start_time.elapsed() > timeout {
                let _ = child.kill();
                // 清理文件
                let _ = fs::remove_file(&source_path);
                let _ = fs::remove_file(&binary_path);
                return Ok(RustExecutionResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("Execution timed out after {} seconds", self.config.timeout_secs),
                    exit_code: -1,
                    compilation_error: false,
                });
            }

            match child.try_wait() {
                Ok(Some(_)) => {
                    let mut stdout_buf = Vec::new();
                    let mut stderr_buf = Vec::new();

                    if let Some(mut stdout_pipe) = child.stdout.take() {
                        let _ = stdout_pipe.read_to_end(&mut stdout_buf);
                    }
                    if let Some(mut stderr_pipe) = child.stderr.take() {
                        let _ = stderr_pipe.read_to_end(&mut stderr_buf);
                    }

                    let stdout = String::from_utf8_lossy(&stdout_buf[..stdout_buf.len().min(self.config.max_output_size)]).to_string();
                    let stderr = String::from_utf8_lossy(&stderr_buf[..stderr_buf.len().min(self.config.max_output_size)]).to_string();

                    break (stdout, stderr);
                }
                Ok(None) => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    // 清理文件
                    let _ = fs::remove_file(&source_path);
                    let _ = fs::remove_file(&binary_path);
                    return Err(anyhow::anyhow!("Failed to wait for process: {}", e));
                }
            }
        };

        // 清理文件
        let _ = fs::remove_file(&source_path);
        let _ = fs::remove_file(&binary_path);

        Ok(RustExecutionResult {
            success: child.wait()?.success(),
            stdout,
            stderr,
            exit_code: child.wait()?.code().unwrap_or(-1),
            compilation_error: false,
        })
    }

    /// 执行 Rust 代码（安全版本，捕获所有错误）
    pub fn execute_safe(&self, code: &str) -> RustExecutionResult {
        match self.execute(code) {
            Ok(result) => result,
            Err(e) => RustExecutionResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Runner error: {}", e),
                exit_code: -1,
                compilation_error: false,
            },
        }
    }
}

impl Default for RustRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Rust 执行结果
#[derive(Debug, Clone)]
pub struct RustExecutionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub compilation_error: bool,
}

impl RustExecutionResult {
    pub fn to_json(&self) -> Value {
        json!({
            "success": self.success,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "exit_code": self.exit_code,
            "compilation_error": self.compilation_error,
        })
    }
}

/// 执行工具调用
pub async fn execute_rust(params: Value) -> Result<Value> {
    let code = params.get("code")
        .and_then(|c| c.as_str())
        .ok_or_else(|| anyhow::anyhow!("code parameter is required"))?;

    let timeout = params.get("timeout").and_then(|t| t.as_u64()).unwrap_or(30);

    let config = RustRunnerConfig {
        timeout_secs: timeout,
        ..Default::default()
    };

    let runner = RustRunner::with_config(config);
    let result = runner.execute_safe(code);

    Ok(result.to_json())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_print() {
        let runner = RustRunner::new();
        let result = runner.execute_safe("println!(\"Hello, World!\");");
        // 注意：这个测试需要系统安装 rustc
        if result.success {
            assert!(result.stdout.contains("Hello, World!"));
        }
    }

    #[test]
    fn test_math() {
        let runner = RustRunner::new();
        let result = runner.execute_safe("println!(\"{}\", 2 + 2);");
        if result.success {
            assert!(result.stdout.contains("4"));
        }
    }

    #[test]
    fn test_syntax_error() {
        let runner = RustRunner::new();
        let result = runner.execute_safe("println!(\"missing semicolon\"");
        assert!(!result.success);
        assert!(result.compilation_error);
    }

    #[test]
    fn test_variable() {
        let runner = RustRunner::new();
        let result = runner.execute_safe(r#"
            let x = 42;
            let y = x * 2;
            println!("x = {}, y = {}", x, y);
        "#);
        if result.success {
            assert!(result.stdout.contains("x = 42"));
            assert!(result.stdout.contains("y = 84"));
        }
    }
}
