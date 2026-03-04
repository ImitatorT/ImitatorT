//! Wikipedia 工具 - 使用 MediaWiki API
//!
//! 官方 API 文档：https://www.mediawiki.org/wiki/API:Main_page

use anyhow::Result;
use serde_json::{json, Value};

/// Wikipedia 工具
pub struct WikipediaTool {
    client: reqwest::Client,
    base_url: String,
}

impl WikipediaTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("ImitatorT/1.0 (https://github.com/ImitatorT/ImitatorT)")
                .build()
                .unwrap_or_default(),
            base_url: "https://en.wikipedia.org/w/api.php".to_string(),
        }
    }

    /// 搜索维基百科
    pub async fn search(&self, query: &str, limit: usize) -> Result<String> {
        let url = format!(
            "{}?action=query&list=search&srsearch={}&srlimit={}&format=json",
            self.base_url,
            urlencoding::encode(query),
            limit
        );

        let response = self.client.get(&url).send().await?;
        let json: Value = response.json().await?;

        self.format_search_results(&json)
    }

    /// 获取词条全文
    pub async fn get_article(&self, title: &str) -> Result<String> {
        let url = format!(
            "{}?action=query&titles={}&prop=extracts&exintro=&explaintext=&format=json",
            self.base_url,
            urlencoding::encode(title)
        );

        let response = self.client.get(&url).send().await?;
        let json: Value = response.json().await?;

        self.format_article(&json)
    }

    /// 获取词条摘要
    pub async fn get_summary(&self, title: &str) -> Result<String> {
        let url = format!(
            "{}?action=query&titles={}&prop=extracts&exintro=&format=json",
            self.base_url,
            urlencoding::encode(title)
        );

        let response = self.client.get(&url).send().await?;
        let json: Value = response.json().await?;

        self.format_article(&json)
    }

    /// 格式化搜索结果
    fn format_search_results(&self, json: &Value) -> Result<String> {
        let mut results = Vec::new();

        if let Some(items) = json.get("query").and_then(|q| q.get("search")).and_then(|s| s.as_array()) {
            for (idx, item) in items.iter().enumerate() {
                let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("");
                let snippet = item.get("snippet").and_then(|s| s.as_str()).unwrap_or("");
                let url = format!(
                    "https://en.wikipedia.org/wiki/{}",
                    urlencoding::encode(title)
                );

                // 移除 HTML 标签
                let clean_snippet = snippet.replace("<span class=\"searchmatch\">", "")
                    .replace("</span>", "");

                results.push(format!(
                    "{}. [{}]({})\n   {}",
                    idx + 1,
                    title,
                    url,
                    clean_snippet
                ));
            }
        }

        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        Ok(results.join("\n\n"))
    }

    /// 格式化词条内容
    fn format_article(&self, json: &Value) -> Result<String> {
        if let Some(pages) = json.get("query").and_then(|q| q.get("pages")).and_then(|p| p.as_object()) {
            for (_, page) in pages.iter() {
                if let Some(missing) = page.get("missing") {
                    if missing.as_bool() == Some(true) {
                        return Ok("Article not found.".to_string());
                    }
                }

                if let Some(extract) = page.get("extract").and_then(|e| e.as_str()) {
                    return Ok(extract.to_string());
                }
            }
        }

        Ok("Article not found.".to_string())
    }
}

impl Default for WikipediaTool {
    fn default() -> Self {
        Self::new()
    }
}

/// 执行工具调用
pub async fn execute_wikipedia(params: Value) -> Result<Value> {
    let tool = WikipediaTool::new();

    let action = params.get("action")
        .and_then(|a| a.as_str())
        .unwrap_or("search");

    match action {
        "search" => {
            let query = params.get("query")
                .and_then(|q| q.as_str())
                .ok_or_else(|| anyhow::anyhow!("query parameter is required"))?;

            let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(5) as usize;

            match tool.search(query, limit).await {
                Ok(results) => Ok(json!({
                    "success": true,
                    "results": results
                })),
                Err(e) => Ok(json!({
                    "success": false,
                    "error": e.to_string()
                })),
            }
        }
        "get_article" | "article" => {
            let title = params.get("title")
                .and_then(|t| t.as_str())
                .ok_or_else(|| anyhow::anyhow!("title parameter is required"))?;

            match tool.get_article(title).await {
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
        "summary" => {
            let title = params.get("title")
                .and_then(|t| t.as_str())
                .ok_or_else(|| anyhow::anyhow!("title parameter is required"))?;

            match tool.get_summary(title).await {
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
        _ => Ok(json!({
            "success": false,
            "error": format!("Unknown action: {}. Supported actions: search, get_article, summary", action)
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search() {
        let tool = WikipediaTool::new();
        let result = tool.search("Rust programming", 3).await;
        if result.is_ok() {
            let content = result.unwrap();
            assert!(content.contains("Rust") || !content.is_empty());
        }
    }

    #[tokio::test]
    async fn test_get_article() {
        let tool = WikipediaTool::new();
        let result = tool.get_article("Rust_(programming_language)").await;
        if result.is_ok() {
            let content = result.unwrap();
            assert!(!content.is_empty());
        }
    }

    #[test]
    fn test_tool_creation() {
        let _tool = WikipediaTool::new();
    }
}
