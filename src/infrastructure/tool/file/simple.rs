//! 简单文件读取工具
//!
//! 提供基本的文件读取功能

use anyhow::Result;
use serde_json::{json, Value};
use tokio::fs;

/// 简单文件读取工具
pub struct SimpleFileTool {
    /// 基础目录（可选）
    base_dir: Option<String>,
}

impl SimpleFileTool {
    pub fn new() -> Self {
        Self { base_dir: None }
    }

    pub fn with_base_dir(base_dir: impl Into<String>) -> Self {
        Self {
            base_dir: Some(base_dir.into()),
        }
    }

    /// 读取文件内容
    pub async fn read(&self, path: &str) -> Result<String> {
        let full_path = match &self.base_dir {
            Some(base) => {
                if path.starts_with('/') || path.starts_with('\\') {
                    path.to_string()
                } else {
                    format!("{}/{}", base, path)
                }
            }
            None => path.to_string(),
        };

        let content = fs::read_to_string(&full_path).await?;
        Ok(content)
    }

    /// 读取文件并限制最大长度
    pub async fn read_with_limit(&self, path: &str, max_chars: usize) -> Result<String> {
        let content = self.read(path).await?;

        if content.len() <= max_chars {
            Ok(content)
        } else {
            Ok(format!(
                "{}\n\n[Content truncated: showing first {} of {} characters]",
                &content[..max_chars],
                max_chars,
                content.len()
            ))
        }
    }

    /// 列出目录内容
    pub async fn list(&self, path: &str) -> Result<Vec<FileInfo>> {
        let full_path = match &self.base_dir {
            Some(base) => {
                if path.starts_with('/') || path.starts_with('\\') {
                    path.to_string()
                } else {
                    format!("{}/{}", base, path)
                }
            }
            None => path.to_string(),
        };

        let mut entries = Vec::new();
        let mut dir = fs::read_dir(&full_path).await?;

        while let Some(entry) = dir.next_entry().await? {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type().await?;
            let metadata = entry.metadata().await?;

            entries.push(FileInfo {
                name: file_name,
                is_dir: file_type.is_dir(),
                is_file: file_type.is_file(),
                size: metadata.len(),
            });
        }

        Ok(entries)
    }
}

impl Default for SimpleFileTool {
    fn default() -> Self {
        Self::new()
    }
}

/// 文件信息
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub size: u64,
}

impl FileInfo {
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "is_dir": self.is_dir,
            "is_file": self.is_file,
            "size": self.size,
        })
    }
}

/// 执行工具调用
pub async fn execute_simple_file(params: Value) -> Result<Value> {
    let action = params.get("action")
        .and_then(|a| a.as_str())
        .unwrap_or("read");

    match action {
        "read" => {
            let path = params.get("path")
                .and_then(|p| p.as_str())
                .ok_or_else(|| anyhow::anyhow!("path parameter is required"))?;

            let limit = params.get("limit").and_then(|l| l.as_u64()).map(|l| l as usize);

            let tool = SimpleFileTool::new();
            let result = match limit {
                Some(max_chars) => tool.read_with_limit(path, max_chars).await,
                None => tool.read(path).await,
            };

            match result {
                Ok(content) => Ok(json!({
                    "success": true,
                    "content": content
                })),
                Err(e) => Ok(json!({
                    "success": false,
                    "error": e.to_string()
                })),
            }
        }
        "list" => {
            let path = params.get("path")
                .and_then(|p| p.as_str())
                .unwrap_or(".");

            let tool = SimpleFileTool::new();
            match tool.list(path).await {
                Ok(entries) => Ok(json!({
                    "success": true,
                    "entries": entries.iter().map(|e| e.to_json()).collect::<Vec<_>>()
                })),
                Err(e) => Ok(json!({
                    "success": false,
                    "error": e.to_string()
                })),
            }
        }
        _ => Ok(json!({
            "success": false,
            "error": format!("Unknown action: {}. Supported actions: read, list", action)
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_read_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"Hello, World!").await.unwrap();

        let tool = SimpleFileTool::with_base_dir(dir.path().to_str().unwrap());
        let result = tool.read("test.txt").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[tokio::test]
    async fn test_read_with_limit() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("large.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"0123456789".repeat(100).as_slice()).await.unwrap();

        let tool = SimpleFileTool::with_base_dir(dir.path().to_str().unwrap());
        let result = tool.read_with_limit("large.txt", 50).await;

        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("truncated"));
    }

    #[tokio::test]
    async fn test_list_directory() {
        let dir = tempdir().unwrap();

        let file_path = dir.path().join("test.txt");
        let subdir = dir.path().join("subdir");

        File::create(&file_path).await.unwrap();
        fs::create_dir(&subdir).await.unwrap();

        let tool = SimpleFileTool::with_base_dir(dir.path().to_str().unwrap());
        let result = tool.list(".").await;

        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(entries.iter().any(|e| e.name == "test.txt"));
        assert!(entries.iter().any(|e| e.name == "subdir" && e.is_dir));
    }
}
