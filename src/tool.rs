use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Tool definition for OpenAI function calling
#[derive(Serialize, Clone)]
pub struct Tool {
    pub r#type: String,
    pub function: Function,
}

#[derive(Serialize, Clone)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool call from LLM response
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Available tools registry
pub struct ToolRegistry;

impl ToolRegistry {
    /// Get all available tools definitions
    pub fn get_tools() -> Vec<Tool> {
        vec![
            Tool {
                r#type: "function".to_string(),
                function: Function {
                    name: "execute_command".to_string(),
                    description: "执行系统命令并返回输出".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "要执行的命令"
                            }
                        },
                        "required": ["command"]
                    }),
                },
            },
            Tool {
                r#type: "function".to_string(),
                function: Function {
                    name: "fetch_url".to_string(),
                    description: "获取指定 URL 的内容".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "要获取的 URL"
                            }
                        },
                        "required": ["url"]
                    }),
                },
            },
        ]
    }

    /// Execute a tool call and return the result
    pub async fn execute(call: &ToolCall) -> Result<String> {
        match call.function.name.as_str() {
            "execute_command" => {
                let args: serde_json::Value =
                    serde_json::from_str(&call.function.arguments).context("解析参数失败")?;
                let command = args["command"].as_str().unwrap_or("");
                execute_shell_command(command).await
            }
            "fetch_url" => {
                let args: serde_json::Value =
                    serde_json::from_str(&call.function.arguments).context("解析参数失败")?;
                let url = args["url"].as_str().unwrap_or("");
                fetch_url_content(url).await
            }
            _ => Ok(format!("未知工具: {}", call.function.name)),
        }
    }
}

/// Execute shell command and return output
async fn execute_shell_command(command: &str) -> Result<String> {
    if command.trim().is_empty() {
        return Ok("命令不能为空".to_string());
    }

    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await
        .context("执行命令失败")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(format!("命令输出:\n{}", stdout))
    } else {
        Ok(format!(
            "命令执行失败 (exit code: {}):\nstdout: {}\nstderr: {}",
            output.status,
            stdout,
            stderr
        ))
    }
}

/// Fetch URL content
async fn fetch_url_content(url: &str) -> Result<String> {
    if url.trim().is_empty() {
        return Ok("URL 不能为空".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("创建 HTTP 客户端失败")?;

    let response = client
        .get(url)
        .send()
        .await
        .context("请求失败")?;

    let status = response.status();
    let text = response
        .text()
        .await
        .context("读取响应内容失败")?;

    if status.is_success() {
        Ok(format!("URL 内容 ({} 字符):\n{}", text.len(), text.chars().take(2000).collect::<String>()))
    } else {
        Ok(format!("请求失败，状态码: {}\n响应: {}", status, text.chars().take(500).collect::<String>()))
    }
}
