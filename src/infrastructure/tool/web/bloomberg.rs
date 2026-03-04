//! Bloomberg 工具 - 获取彭博社财经新闻
//!
//! 爬取 Bloomberg 搜索页面获取新闻

use anyhow::Result;
use serde_json::{json, Value};

/// Bloomberg 工具
pub struct BloombergTool {
    client: reqwest::Client,
    base_url: String,
}

impl BloombergTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            base_url: "https://www.bloomberg.com".to_string(),
        }
    }

    /// 搜索新闻
    pub async fn search_news(&self, query: &str, limit: usize) -> Result<String> {
        let url = format!(
            "{}/search?query={}",
            self.base_url,
            urlencoding::encode(query)
        );

        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;

        self.parse_search_results(&html, limit)
    }

    /// 获取最新新闻
    pub async fn get_latest_news(&self, limit: usize) -> Result<String> {
        let url = format!("{}/latest", self.base_url);

        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;

        self.parse_latest_news(&html, limit)
    }

    /// 解析搜索结果
    fn parse_search_results(&self, html: &str, limit: usize) -> Result<String> {
        let document = scraper::Html::parse_document(html);
        let mut results = Vec::new();

        // Bloomberg 搜索结果选择器
        // 注意：Bloomberg 的 HTML 结构可能变化，需要定期检查
        let article_selector = scraper::Selector::parse("article").unwrap();
        let headline_selector = scraper::Selector::parse(".headline").unwrap();
        let summary_selector = scraper::Selector::parse(".summary").unwrap();
        let timestamp_selector = scraper::Selector::parse("time").unwrap();

        for (idx, article) in document.select(&article_selector).take(limit).enumerate() {
            let headline = article.select(&headline_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let summary = article.select(&summary_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let timestamp = article.select(&timestamp_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // 获取文章链接
            let url = article.value().attr("data-url")
                .or_else(|| article.value().attr("href"))
                .unwrap_or("");

            let full_url = if url.starts_with("http") {
                url.to_string()
            } else if url.is_empty() {
                String::new()
            } else {
                format!("{}{}", self.base_url, url)
            };

            if !headline.is_empty() {
                let mut result = format!("{}. **{}**", idx + 1, headline);
                if !full_url.is_empty() {
                    result.push_str(&format!("\n   URL: {}", full_url));
                }
                if !timestamp.is_empty() {
                    result.push_str(&format!("\n   Time: {}", timestamp));
                }
                if !summary.is_empty() {
                    result.push_str(&format!("\n   Summary: {}", summary));
                }
                results.push(result);
            }
        }

        // 如果标准选择器没有结果，尝试备用方法
        if results.is_empty() {
            return self.parse_with_text_search(html, limit);
        }

        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        Ok(results.join("\n\n"))
    }

    /// 备用文本搜索解析
    fn parse_with_text_search(&self, html: &str, limit: usize) -> Result<String> {
        let document = scraper::Html::parse_document(html);
        let mut results = Vec::new();

        // 查找所有链接
        let link_selector = scraper::Selector::parse("a").unwrap();

        for (idx, link) in document.select(&link_selector).take(50).enumerate() {
            let text = link.text().collect::<String>().trim().to_string();
            let href = link.value().attr("href").unwrap_or("").to_string();

            // 过滤出新闻链接
            if text.len() > 20 && text.len() < 200 && href.contains("/news/") {
                results.push(format!("{}. {}{}", idx + 1, text,
                    if href.starts_with("http") {
                        format!(" ({})", href)
                    } else {
                        format!(" ({}/{})", self.base_url, href)
                    }
                ));

                if results.len() >= limit {
                    break;
                }
            }
        }

        if results.is_empty() {
            return Ok("No results found. Bloomberg may have blocked the request.".to_string());
        }

        Ok(results.join("\n"))
    }

    /// 解析最新新闻
    fn parse_latest_news(&self, html: &str, limit: usize) -> Result<String> {
        self.parse_search_results(html, limit)
    }
}

impl Default for BloombergTool {
    fn default() -> Self {
        Self::new()
    }
}

/// 执行工具调用
pub async fn execute_bloomberg(params: Value) -> Result<Value> {
    let tool = BloombergTool::new();

    let action = params.get("action")
        .and_then(|a| a.as_str())
        .unwrap_or("search");

    match action {
        "search" => {
            let query = params.get("query")
                .and_then(|q| q.as_str())
                .ok_or_else(|| anyhow::anyhow!("query parameter is required"))?;

            let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

            match tool.search_news(query, limit).await {
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
        "latest" => {
            let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

            match tool.get_latest_news(limit).await {
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
        _ => Ok(json!({
            "success": false,
            "error": format!("Unknown action: {}. Supported actions: search, latest", action)
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let _tool = BloombergTool::new();
    }
}
