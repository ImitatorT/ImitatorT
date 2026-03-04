//! Python 代码执行沙箱
//!
//! 使用 subprocess 执行 Python 代码，带有超时和资源限制

use anyhow::Result;
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Python 沙箱配置
#[derive(Debug, Clone)]
pub struct PythonSandboxConfig {
    /// 执行超时（秒）
    pub timeout_secs: u64,
    /// 最大输出长度（字节）
    pub max_output_size: usize,
    /// Python 解释器路径
    pub python_path: String,
}

impl Default for PythonSandboxConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_output_size: 1024 * 1024,
            python_path: "python3".to_string(),
        }
    }
}

/// Python 沙箱
pub struct PythonSandbox {
    config: PythonSandboxConfig,
}

impl PythonSandbox {
    pub fn new() -> Self {
        Self {
            config: PythonSandboxConfig::default(),
        }
    }

    pub fn with_config(config: PythonSandboxConfig) -> Self {
        Self { config }
    }

    /// 执行 Python 代码
    pub fn execute(&self, code: &str) -> Result<PythonExecutionResult> {
        let mut child = Command::new(&self.config.python_path)
            .arg("-c")
            .arg(code)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let timeout = Duration::from_secs(self.config.timeout_secs);
        let start_time = std::time::Instant::now();

        let (stdout, stderr) = loop {
            if start_time.elapsed() > timeout {
                let _ = child.kill();
                return Ok(PythonExecutionResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("Execution timed out after {} seconds", self.config.timeout_secs),
                    exit_code: -1,
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
                    return Err(anyhow::anyhow!("Failed to wait for process: {}", e));
                }
            }
        };

        Ok(PythonExecutionResult {
            success: child.wait()?.success(),
            stdout,
            stderr,
            exit_code: child.wait()?.code().unwrap_or(-1),
        })
    }

    /// 执行 Python 代码（安全版本，捕获所有错误）
    pub fn execute_safe(&self, code: &str) -> PythonExecutionResult {
        match self.execute(code) {
            Ok(result) => result,
            Err(e) => PythonExecutionResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Sandbox error: {}", e),
                exit_code: -1,
            },
        }
    }
}

impl Default for PythonSandbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Python 执行结果
#[derive(Debug, Clone)]
pub struct PythonExecutionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl PythonExecutionResult {
    pub fn to_json(&self) -> Value {
        json!({
            "success": self.success,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "exit_code": self.exit_code,
        })
    }
}

/// 执行工具调用
pub async fn execute_python(params: Value) -> Result<Value> {
    let code = params.get("code")
        .and_then(|c| c.as_str())
        .ok_or_else(|| anyhow::anyhow!("code parameter is required"))?;

    let timeout = params.get("timeout").and_then(|t| t.as_u64()).unwrap_or(30);

    let config = PythonSandboxConfig {
        timeout_secs: timeout,
        ..Default::default()
    };

    let sandbox = PythonSandbox::with_config(config);
    let result = sandbox.execute_safe(code);

    Ok(result.to_json())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_print() {
        let sandbox = PythonSandbox::new();
        let result = sandbox.execute_safe("print('Hello, World!')");
        assert!(result.success);
        assert!(result.stdout.contains("Hello, World!"));
    }

    #[test]
    fn test_math() {
        let sandbox = PythonSandbox::new();
        let result = sandbox.execute_safe("print(2 + 2)");
        assert!(result.success);
        assert!(result.stdout.contains("4"));
    }

    #[test]
    fn test_syntax_error() {
        let sandbox = PythonSandbox::new();
        let result = sandbox.execute_safe("print(invalid syntax here");
        assert!(!result.success);
        assert!(!result.stderr.is_empty());
    }
}
