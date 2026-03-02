//! Cargo 工具
//!
//! 提供安装 crates.io 包的功能

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::process::Command;
use tracing::{error, info};

use crate::domain::tool::ToolCallContext;
use crate::infrastructure::tool::ToolResult;

/// Cargo 工具执行器
pub struct CargoToolExecutor;

impl CargoToolExecutor {
    /// 创建新的 Cargo 工具执行器
    pub fn new() -> Self {
        Self
    }

    /// 获取支持的 Cargo 工具 ID 列表
    pub fn supported_tool_ids() -> Vec<&'static str> {
        vec![
            "cargo.install",
            "cargo.install_snapshot",
            "cargo.uninstall",
            "cargo.check_version",
        ]
    }

    /// 执行工具调用
    pub async fn execute(
        &self,
        tool_id: &str,
        params: Value,
        _context: &ToolCallContext,
    ) -> Result<ToolResult> {
        match tool_id {
            "cargo.install" => self.install(params).await,
            "cargo.install_snapshot" => self.install_snapshot(params).await,
            "cargo.uninstall" => self.uninstall(params).await,
            "cargo.check_version" => self.check_version(params).await,
            _ => Ok(ToolResult::error(format!("Unknown tool: {}", tool_id))),
        }
    }

    /// 安装 crate
    async fn install(&self, params: Value) -> Result<ToolResult> {
        let crate_name = params["crate_name"]
            .as_str()
            .context("crate_name is required")?;
        let version = params.get("version").and_then(|v| v.as_str());

        info!("Installing crate: {}", crate_name);

        let mut cmd = Command::new("cargo");
        cmd.arg("install").arg(crate_name);

        if let Some(v) = version {
            cmd.arg("--version").arg(v);
            info!("Installing version: {}", v);
        }

        let output = cmd.output().context("Failed to execute cargo command")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(ToolResult::success(json!({
                "installed": true,
                "crate": crate_name,
                "version": version,
                "output": stdout.to_string()
            })))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to install crate: {}", stderr);
            Ok(ToolResult::error(format!("Failed to install crate: {}", stderr)))
        }
    }

    /// 安装 snapshot 版本
    async fn install_snapshot(&self, params: Value) -> Result<ToolResult> {
        let crate_name = params["crate_name"]
            .as_str()
            .context("crate_name is required")?;
        let version = params["version"]
            .as_str()
            .context("version is required for snapshot install")?;

        info!(
            "Installing snapshot version: {}@{}",
            crate_name, version
        );

        let output = Command::new("cargo")
            .arg("install")
            .arg(crate_name)
            .arg("--version")
            .arg(version)
            .arg("--force")
            .output()
            .context("Failed to execute cargo command")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(ToolResult::success(json!({
                "installed": true,
                "crate": crate_name,
                "version": version,
                "output": stdout.to_string()
            })))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to install snapshot: {}", stderr);
            Ok(ToolResult::error(format!(
                "Failed to install snapshot: {}",
                stderr
            )))
        }
    }

    /// 卸载 crate
    async fn uninstall(&self, params: Value) -> Result<ToolResult> {
        let crate_name = params["crate_name"]
            .as_str()
            .context("crate_name is required")?;

        info!("Uninstalling crate: {}", crate_name);

        let output = Command::new("cargo")
            .arg("uninstall")
            .arg(crate_name)
            .output()
            .context("Failed to execute cargo command")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(ToolResult::success(json!({
                "uninstalled": true,
                "crate": crate_name,
                "output": stdout.to_string()
            })))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(ToolResult::error(format!("Failed to uninstall: {}", stderr)))
        }
    }

    /// 检查版本
    async fn check_version(&self, params: Value) -> Result<ToolResult> {
        let crate_name = params["crate_name"]
            .as_str()
            .context("crate_name is required")?;

        info!("Checking latest version of: {}", crate_name);

        // 使用 cargo search 获取最新版本
        let output = Command::new("cargo")
            .arg("search")
            .arg(crate_name)
            .arg("--limit")
            .arg("1")
            .output()
            .context("Failed to execute cargo search")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // 解析版本号（格式：crate_name = "version"）
            let version = extract_version(&stdout, crate_name);

            Ok(ToolResult::success(json!({
                "crate": crate_name,
                "latest_version": version,
                "search_output": stdout.to_string()
            })))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(ToolResult::error(format!("Failed to search: {}", stderr)))
        }
    }
}

/// 从 cargo search 输出中提取版本号
fn extract_version(search_output: &str, crate_name: &str) -> Option<String> {
    for line in search_output.lines() {
        if line.starts_with(crate_name) {
            // 格式：crate_name = "version" # description
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    return Some(line[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }
    None
}

impl Default for CargoToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// 实现 ToolExecutor trait
use crate::infrastructure::tool::ToolExecutor as ToolExecutorTrait;
use async_trait::async_trait;

#[async_trait]
impl ToolExecutorTrait for CargoToolExecutor {
    async fn execute(
        &self,
        tool_id: &str,
        params: Value,
        context: &ToolCallContext,
    ) -> Result<Value> {
        let result = Self::execute(self, tool_id, params, context).await?;

        if result.success {
            Ok(result.data)
        } else {
            Err(anyhow::anyhow!(
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ))
        }
    }

    fn can_execute(&self, tool_id: &str) -> bool {
        Self::supported_tool_ids().contains(&tool_id)
    }

    fn supported_tools(&self) -> Vec<String> {
        Self::supported_tool_ids().iter().map(|s| s.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_tool_ids() {
        let tools = CargoToolExecutor::supported_tool_ids();
        assert!(tools.contains(&"cargo.install"));
        assert!(tools.contains(&"cargo.install_snapshot"));
        assert!(tools.contains(&"cargo.uninstall"));
        assert!(tools.contains(&"cargo.check_version"));
    }

    #[test]
    fn test_extract_version() {
        let output = r#"imitatort = "0.0.1"    # A multi-agent framework
search = "0.1.0"      # Search tool"#;

        assert_eq!(extract_version(output, "imitatort"), Some("0.0.1".to_string()));
        assert_eq!(extract_version(output, "search"), Some("0.1.0".to_string()));
        assert_eq!(extract_version(output, "unknown"), None);
    }
}
