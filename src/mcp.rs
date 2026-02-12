use anyhow::{Context, Result};
use tokio::process::Command;

pub async fn run_stdio_tool(command_line: &str, prompt: &str) -> Result<Option<String>> {
    if command_line.trim().is_empty() {
        return Ok(None);
    }

    let mut parts = command_line.split_whitespace();
    let cmd = match parts.next() {
        Some(c) => c,
        None => return Ok(None),
    };
    let args = parts.collect::<Vec<_>>();

    let output = Command::new(cmd)
        .args(args)
        .arg(prompt)
        .output()
        .await
        .context("failed to execute MCP stdio tool")?;

    if !output.status.success() {
        return Ok(Some(format!(
            "MCP tool exited with status {}",
            output.status
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        Ok(None)
    } else {
        Ok(Some(stdout))
    }
}
