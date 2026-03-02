//! GitHub Actions 工具
//!
//! 提供触发 workflow、检查构建状态等功能

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::process::Command;
use tracing::{error, info};

use crate::domain::tool::ToolCallContext;
use crate::infrastructure::tool::ToolResult;

/// GitHub Actions 工具执行器
pub struct GitHubToolExecutor;

impl GitHubToolExecutor {
    /// 创建新的 GitHub 工具执行器
    pub fn new() -> Self {
        Self
    }

    /// 获取支持的 GitHub 工具 ID 列表
    pub fn supported_tool_ids() -> Vec<&'static str> {
        vec![
            "github.trigger_workflow",
            "github.check_workflow_status",
            "github.get_latest_run",
            "github.download_artifact",
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
            "github.trigger_workflow" => self.trigger_workflow(params).await,
            "github.check_workflow_status" => self.check_workflow_status(params).await,
            "github.get_latest_run" => self.get_latest_run(params).await,
            "github.download_artifact" => self.download_artifact(params).await,
            _ => Ok(ToolResult::error(format!("Unknown tool: {}", tool_id))),
        }
    }

    /// 触发 GitHub Actions workflow
    async fn trigger_workflow(&self, params: Value) -> Result<ToolResult> {
        let workflow_id = params["workflow_id"]
            .as_str()
            .context("workflow_id is required")?;
        let branch = params["branch"].as_str().unwrap_or("main");
        let inputs = params.get("inputs").cloned().unwrap_or(Value::Null);

        info!("Triggering workflow: {} on branch: {}", workflow_id, branch);

        // 使用 gh CLI 触发 workflow
        let mut cmd = Command::new("gh");
        cmd.arg("workflow-run").arg("create").arg(workflow_id);
        cmd.arg("--ref").arg(branch);

        if let Some(inputs_obj) = inputs.as_object() {
            let inputs_json = serde_json::to_string(inputs_obj)?;
            cmd.arg("--inputs").arg(inputs_json);
        }

        let output = cmd.output().context("Failed to execute gh command")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(ToolResult::success(json!({
                "triggered": true,
                "workflow_id": workflow_id,
                "branch": branch,
                "output": stdout.to_string()
            })))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to trigger workflow: {}", stderr);
            Ok(ToolResult::error(format!(
                "Failed to trigger workflow: {}",
                stderr
            )))
        }
    }

    /// 检查 workflow 运行状态
    async fn check_workflow_status(&self, params: Value) -> Result<ToolResult> {
        let run_id = params["run_id"]
            .as_u64()
            .context("run_id is required")?;

        info!("Checking workflow run status: {}", run_id);

        // 使用 gh CLI 检查状态
        let output = Command::new("gh")
            .arg("run")
            .arg("view")
            .arg(run_id.to_string())
            .arg("--json")
            .arg("status,conclusion,displayTitle,workflowName,createdAt")
            .output()
            .context("Failed to execute gh command")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let status: Value = serde_json::from_str(&stdout)?;

            Ok(ToolResult::success(json!({
                "run_id": run_id,
                "status": status.get("status").and_then(|v| v.as_str()).unwrap_or("unknown"),
                "conclusion": status.get("conclusion").and_then(|v| v.as_str()),
                "workflow": status.get("workflowName").and_then(|v| v.as_str()),
                "created_at": status.get("createdAt").and_then(|v| v.as_str()),
            })))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(ToolResult::error(format!("Failed to check status: {}", stderr)))
        }
    }

    /// 获取最新的 workflow run
    async fn get_latest_run(&self, params: Value) -> Result<ToolResult> {
        let workflow_id = params["workflow_id"].as_str();
        let branch = params.get("branch").and_then(|v| v.as_str());
        let status = params.get("status").and_then(|v| v.as_str());

        info!("Getting latest workflow run");

        // 构建 gh CLI 命令
        let mut cmd = Command::new("gh");
        cmd.arg("run")
            .arg("list")
            .arg("--limit")
            .arg("1")
            .arg("--json")
            .arg(
                "databaseId,status,conclusion,displayTitle,workflowName,createdAt,headBranch",
            );

        if let Some(wid) = workflow_id {
            cmd.arg("--workflow").arg(wid);
        }

        if let Some(branch_name) = branch {
            cmd.arg("--branch").arg(branch_name);
        }

        if let Some(s) = status {
            cmd.arg("--status").arg(s);
        }

        let output = cmd.output().context("Failed to execute gh command")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let runs: Vec<Value> = serde_json::from_str(&stdout).unwrap_or_default();

            Ok(ToolResult::success(json!({
                "count": runs.len(),
                "latest_run": runs.first().cloned().unwrap_or(Value::Null)
            })))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(ToolResult::error(format!("Failed to list runs: {}", stderr)))
        }
    }

    /// 下载 workflow artifact
    async fn download_artifact(&self, params: Value) -> Result<ToolResult> {
        let run_id = params["run_id"].as_u64().context("run_id is required")?;
        let artifact_name = params["artifact_name"]
            .as_str()
            .context("artifact_name is required")?;
        let output_dir = params["output_dir"].as_str().unwrap_or(".");

        info!(
            "Downloading artifact {} from run {}",
            artifact_name, run_id
        );

        // 使用 gh CLI 下载 artifact
        let output = Command::new("gh")
            .arg("run")
            .arg("download")
            .arg(run_id.to_string())
            .arg("--name")
            .arg(artifact_name)
            .arg("--dir")
            .arg(output_dir)
            .output()
            .context("Failed to execute gh command")?;

        if output.status.success() {
            Ok(ToolResult::success(json!({
                "downloaded": true,
                "run_id": run_id,
                "artifact_name": artifact_name,
                "output_dir": output_dir
            })))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(ToolResult::error(format!(
                "Failed to download artifact: {}",
                stderr
            )))
        }
    }
}

impl Default for GitHubToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// 实现 ToolExecutor trait
use crate::infrastructure::tool::ToolExecutor as ToolExecutorTrait;
use async_trait::async_trait;

#[async_trait]
impl ToolExecutorTrait for GitHubToolExecutor {
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
        let tools = GitHubToolExecutor::supported_tool_ids();
        assert!(tools.contains(&"github.trigger_workflow"));
        assert!(tools.contains(&"github.check_workflow_status"));
        assert!(tools.contains(&"github.get_latest_run"));
        assert!(tools.contains(&"github.download_artifact"));
    }
}
