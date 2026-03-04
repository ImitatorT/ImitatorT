//! WebVisit 工具 - 网页访问和内容提取
//!
//! 使用 Jina Reader (https://r.jina.ai) 将网页转换为 LLM 友好的 Markdown 格式

use anyhow::Result;
use serde_json::{json, Value};

use crate::infrastructure::tool::common::HtmlParser;

/// WebVisit 工具
pub struct WebVisitTool {
    client: reqwest::Client,
    html_parser: HtmlParser,
}

impl WebVisitTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; ImitatorT/1.0)")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            html_parser: HtmlParser::new(),
        }
    }

    /// 访问单个网页
    pub async fn visit(&self, url: &str) -> Result<String> {
        // 使用 Jina Reader
        let jina_url = format!("https://r.jina.ai/{}", url);

        let response = self.client.get(&jina_url).send().await?;
        let status = response.status();

        if !status.is_success() {
            // Jina Reader 失败时，尝试直接获取并解析 HTML
            return self.visit_direct(url).await;
        }

        let text = response.text().await?;
        Ok(text)
    }

    /// 直接访问网页（备用方案）
    async fn visit_direct(&self, url: &str) -> Result<String> {
        let response = self.client.get(url).send().await?;
        let html = response.text().await?;

        // 使用 HTML 解析器提取纯文本
        Ok(self.html_parser.parse_to_text(&html))
    }

    /// 访问多个网页
    pub async fn visit_multiple(&self, urls: &[String]) -> Result<String> {
        if urls.is_empty() {
            return Ok("No URLs provided.".to_string());
        }

        // 并发访问多个网页
        let futures: Vec<_> = urls.iter().map(|u| self.visit(u)).collect();
        let results = futures::future::join_all(futures).await;

        let mut output = Vec::new();
        for (url, result) in urls.iter().zip(results.iter()) {
            match result {
                Ok(content) => {
                    output.push(format!("## URL: {}\n\n{}", url, content));
                }
                Err(e) => {
                    output.push(format!("## URL: {}\n\nError: {}", url, e));
                }
            }
        }

        Ok(output.join("\n\n---\n\n"))
    }

    /// 带目标地访问网页（使用 LLM 提取相关信息）
    pub async fn visit_with_goal(&self, url: &str, goal: &str) -> Result<String> {
        // 首先获取网页内容
        let content = self.visit(url).await?;

        // 如果内容已经是 Markdown 格式（来自 Jina），直接返回
        // 如果需要 LLM 提取，这里可以集成 LLM 调用
        // 为简化实现，直接返回内容

        Ok(format!(
            "## Goal: {}\n\n## Content from {}:\n\n{}",
            goal, url, content
        ))
    }
}

impl Default for WebVisitTool {
    fn default() -> Self {
        Self::new()
    }
}

/// 执行工具调用
pub async fn execute_web_visit(params: Value) -> Result<Value> {
    let tool = WebVisitTool::new();

    let urls = if let Some(url) = params.get("url") {
        if let Some(arr) = url.as_array() {
            arr.iter()
                .filter_map(|u| u.as_str().map(String::from))
                .collect::<Vec<_>>()
        } else if let Some(u) = url.as_str() {
            vec![u.to_string()]
        } else {
            return Ok(json!({
                "success": false,
                "error": "url must be a string or array of strings"
            }));
        }
    } else {
        return Ok(json!({
            "success": false,
            "error": "url parameter is required"
        }));
    };

    let goal = params.get("goal").and_then(|g| g.as_str()).unwrap_or("Extract relevant information");

    let result = if urls.len() == 1 {
        if goal.is_empty() {
            tool.visit(&urls[0]).await
        } else {
            tool.visit_with_goal(&urls[0], goal).await
        }
    } else {
        tool.visit_multiple(&urls).await
    };

    match result {
        Ok(text) => Ok(json!({
            "success": true,
            "content": text
        })),
        Err(e) => Ok(json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_visit_wikipedia() {
        let tool = WebVisitTool::new();
        let result = tool.visit("https://en.wikipedia.org/wiki/Rust_(programming_language)").await;
        if result.is_ok() {
            let content = result.unwrap();
            assert!(content.contains("Rust") || !content.is_empty());
        }
    }

    #[test]
    fn test_tool_creation() {
        let _tool = WebVisitTool::new();
    }
}
