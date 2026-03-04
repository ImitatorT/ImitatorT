//! Web 搜索工具 - 多引擎支持
//!
//! 支持的搜索引擎：
//! - Jina Search (优先，免费无限)
//! - Brave Search (备用，2500 次/月免费)
//! - Bing Search (备用，1000 次/月免费)
//! - DuckDuckGo HTML (最后备用，无需 API)

use anyhow::Result;
use serde_json::{json, Value};
use std::env;
use std::sync::Arc;

use crate::infrastructure::tool::common::{ApiKeyPool, HtmlParser, SimpleRateLimiter};

/// Jina Search 客户端
pub struct JinaSearchClient {
    client: reqwest::Client,
    base_url: String,
}

impl JinaSearchClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; ImitatorT/1.0)")
                .build()
                .unwrap_or_default(),
            base_url: "https://s.jina.ai".to_string(),
        }
    }

    /// 执行搜索
    pub async fn search(&self, query: &str) -> Result<String> {
        let encoded_query = urlencoding::encode(query);
        let url = format!("{}/{}", self.base_url, encoded_query);

        let response = self.client.get(&url).send().await?;
        let status = response.status();

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Jina Search API error: {} {}",
                status,
                response.text().await.unwrap_or_default()
            ));
        }

        let text = response.text().await?;
        Ok(text)
    }
}

impl Default for JinaSearchClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Brave Search 客户端
pub struct BraveSearchClient {
    client: reqwest::Client,
    api_key_pool: Option<ApiKeyPool>,
    base_url: String,
    rate_limiter: Arc<SimpleRateLimiter>,
}

impl BraveSearchClient {
    pub fn new() -> Self {
        let api_keys = env::var("BRAVE_API_KEYS").ok();
        let api_key_pool = api_keys.as_deref().map(ApiKeyPool::from_csv);

        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; ImitatorT/1.0)")
                .build()
                .unwrap_or_default(),
            api_key_pool,
            base_url: "https://api.search.brave.com/res/v1/web/search".to_string(),
            rate_limiter: Arc::new(SimpleRateLimiter::new(2)), // 2 请求/秒
        }
    }

    /// 执行搜索
    pub async fn search(&self, query: &str) -> Result<String> {
        // 检查是否有 API Key
        let api_key = match &self.api_key_pool {
            Some(pool) => pool.get_next_key().ok_or_else(|| {
                anyhow::anyhow!("No available Brave API keys or all keys are rate limited")
            })?,
            None => {
                return Err(anyhow::anyhow!(
                    "Brave API keys not configured. Set BRAVE_API_KEYS environment variable."
                ))
            }
        };

        // 限流
        self.rate_limiter.wait().await;

        let encoded_query = urlencoding::encode(query);
        let url = format!("{}?q={}&count=10", self.base_url, encoded_query);

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &api_key)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            // 如果是 401 或 403，可能是 API Key 问题，将其置于冷却期
            if status.as_u16() == 401 || status.as_u16() == 403 {
                if let Some(pool) = &self.api_key_pool {
                    pool.cooldown_key(&api_key, std::time::Duration::from_secs(3600));
                }
            }
            return Err(anyhow::anyhow!("Brave Search API error: {} {}", status, text));
        }

        // 解析 JSON 响应并格式化
        self.format_brave_response(&text)
    }

    /// 格式化 Brave Search 响应
    fn format_brave_response(&self, json_str: &str) -> Result<String> {
        let value: Value = serde_json::from_str(json_str)?;
        let mut results = Vec::new();

        // 提取 Web 结果
        if let Some(web) = value.get("web").and_then(|w| w.get("results")).and_then(|r| r.as_array()) {
            for (idx, item) in web.iter().take(10).enumerate() {
                let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("");
                let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("");
                let description = item.get("description").and_then(|d| d.as_str()).unwrap_or("");

                results.push(format!(
                    "{}. [{}]({})\n   {}",
                    idx + 1,
                    title,
                    url,
                    description
                ));
            }
        }

        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        Ok(results.join("\n\n"))
    }
}

impl Default for BraveSearchClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Bing Search 客户端
pub struct BingSearchClient {
    client: reqwest::Client,
    api_key_pool: Option<ApiKeyPool>,
    base_url: String,
    rate_limiter: Arc<SimpleRateLimiter>,
}

impl BingSearchClient {
    pub fn new() -> Self {
        let api_keys = env::var("BING_API_KEYS").ok();
        let api_key_pool = api_keys.as_deref().map(ApiKeyPool::from_csv);

        Self {
            client: reqwest::Client::builder().build().unwrap_or_default(),
            api_key_pool,
            base_url: "https://api.bing.microsoft.com/v7.0/search".to_string(),
            rate_limiter: Arc::new(SimpleRateLimiter::new(2)), // 2 请求/秒
        }
    }

    /// 执行搜索
    pub async fn search(&self, query: &str) -> Result<String> {
        let api_key = match &self.api_key_pool {
            Some(pool) => pool.get_next_key().ok_or_else(|| {
                anyhow::anyhow!("No available Bing API keys or all keys are rate limited")
            })?,
            None => {
                return Err(anyhow::anyhow!(
                    "Bing API keys not configured. Set BING_API_KEYS environment variable."
                ))
            }
        };

        // 限流
        self.rate_limiter.wait().await;

        let encoded_query = urlencoding::encode(query);
        let url = format!(
            "{}?q={}&count=10&mkt=zh-CN",
            self.base_url, encoded_query
        );

        let response = self
            .client
            .get(&url)
            .header("Ocp-Apim-Subscription-Key", &api_key)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            // 如果是 401，可能是 API Key 问题
            if status.as_u16() == 401 {
                if let Some(pool) = &self.api_key_pool {
                    pool.cooldown_key(&api_key, std::time::Duration::from_secs(3600));
                }
            }
            return Err(anyhow::anyhow!("Bing Search API error: {} {}", status, text));
        }

        // 解析 JSON 响应并格式化
        self.format_bing_response(&text)
    }

    /// 格式化 Bing Search 响应
    fn format_bing_response(&self, json_str: &str) -> Result<String> {
        let value: Value = serde_json::from_str(json_str)?;
        let mut results = Vec::new();

        // 提取 Web 结果
        if let Some(web_pages) = value
            .get("webPages")
            .and_then(|wp| wp.get("value"))
            .and_then(|v| v.as_array())
        {
            for (idx, item) in web_pages.iter().take(10).enumerate() {
                let title = item.get("name").and_then(|t| t.as_str()).unwrap_or("");
                let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("");
                let snippet = item.get("snippet").and_then(|s| s.as_str()).unwrap_or("");

                results.push(format!(
                    "{}. [{}]({})\n   {}",
                    idx + 1,
                    title,
                    url,
                    snippet
                ));
            }
        }

        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        Ok(results.join("\n\n"))
    }
}

impl Default for BingSearchClient {
    fn default() -> Self {
        Self::new()
    }
}

/// DuckDuckGo HTML 搜索客户端（无需 API）
pub struct DuckDuckGoClient {
    client: reqwest::Client,
    rate_limiter: Arc<SimpleRateLimiter>,
}

impl DuckDuckGoClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .build()
                .unwrap_or_default(),
            rate_limiter: Arc::new(SimpleRateLimiter::new(1)), // 1 请求/秒，更保守
        }
    }

    /// 执行搜索
    pub async fn search(&self, query: &str) -> Result<String> {
        // 限流
        self.rate_limiter.wait().await;

        let encoded_query = urlencoding::encode(query);
        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!("q={}", encoded_query))
            .send()
            .await?;

        let status = response.status();
        let html = response.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!("DuckDuckGo HTML error: {}", status));
        }

        // 解析 HTML 结果
        self.parse_ddg_html(&html)
    }

    /// 解析 DuckDuckGo HTML 结果
    fn parse_ddg_html(&self, html: &str) -> Result<String> {
        let document = scraper::Html::parse_document(html);
        let mut results = Vec::new();

        // DuckDuckGo HTML 结果使用 result__body 类
        let result_selector = scraper::Selector::parse(".result__body").unwrap();
        let title_selector = scraper::Selector::parse("a.result__a").unwrap();
        let snippet_selector = scraper::Selector::parse("a.result__snippet").unwrap();

        for (idx, result_elem) in document.select(&result_selector).take(10).enumerate() {
            if let Some(title_elem) = result_elem.select(&title_selector).next() {
                let title = title_elem.text().collect::<String>().trim().to_string();
                let url = title_elem
                    .value()
                    .attr("href")
                    .unwrap_or("")
                    .to_string();

                let snippet = result_elem
                    .select(&snippet_selector)
                    .next()
                    .map(|e| e.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();

                results.push(format!(
                    "{}. [{}]({})\n   {}",
                    idx + 1,
                    title,
                    url,
                    snippet
                ));
            }
        }

        if results.is_empty() {
            // 尝试备用选择器
            let alt_selector = scraper::Selector::parse(".results").unwrap();
            if let Some(results_elem) = document.select(&alt_selector).next() {
                let link_selector = scraper::Selector::parse("a.result__url").unwrap();
                for (idx, link_elem) in results_elem.select(&link_selector).take(10).enumerate() {
                    let url = link_elem.text().collect::<String>().trim().to_string();
                    results.push(format!("{}. {}", idx + 1, url));
                }
            }
        }

        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        Ok(results.join("\n\n"))
    }
}

impl Default for DuckDuckGoClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Web 搜索工具 - 多引擎支持
pub struct WebSearchTool {
    jina_client: JinaSearchClient,
    brave_client: Option<BraveSearchClient>,
    bing_client: Option<BingSearchClient>,
    ddg_client: DuckDuckGoClient,
    html_parser: HtmlParser,
}

impl WebSearchTool {
    pub fn new() -> Self {
        // 检查环境变量，决定是否初始化带 API Key 的客户端
        let has_brave_keys = env::var("BRAVE_API_KEYS").is_ok();
        let has_bing_keys = env::var("BING_API_KEYS").is_ok();

        Self {
            jina_client: JinaSearchClient::new(),
            brave_client: if has_brave_keys {
                Some(BraveSearchClient::new())
            } else {
                None
            },
            bing_client: if has_bing_keys {
                Some(BingSearchClient::new())
            } else {
                None
            },
            ddg_client: DuckDuckGoClient::new(),
            html_parser: HtmlParser::new(),
        }
    }

    /// 执行搜索（多引擎故障转移）
    pub async fn search(&self, query: &str) -> Result<String> {
        // 1. 尝试 Jina Search（免费无限）
        match self.jina_client.search(query).await {
            Ok(result) if !result.is_empty() && !result.contains("error") => {
                return Ok(self.format_jina_response(&result));
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("Jina Search failed: {}", e);
            }
        }

        // 2. 尝试 Brave Search
        if let Some(ref brave) = self.brave_client {
            match brave.search(query).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("Brave Search failed: {}", e);
                }
            }
        }

        // 3. 尝试 Bing Search
        if let Some(ref bing) = self.bing_client {
            match bing.search(query).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("Bing Search failed: {}", e);
                }
            }
        }

        // 4. 最后尝试 DuckDuckGo HTML
        match self.ddg_client.search(query).await {
            Ok(result) => Ok(result),
            Err(e) => Err(anyhow::anyhow!(
                "All search engines failed. Last error: {}",
                e
            )),
        }
    }

    /// 格式化 Jina Search 响应
    fn format_jina_response(&self, text: &str) -> String {
        // Jina Search 返回的是 Markdown 格式
        // 每篇文章格式类似：
        // Title: ...
        // URL: ...
        // Content: ...
        //
        // 解析并重新格式化

        let mut results = Vec::new();
        let mut current_title = String::new();
        let mut current_url = String::new();
        let mut current_content = String::new();

        for line in text.lines() {
            if line.starts_with("Title:") {
                if !current_title.is_empty() {
                    results.push(format!(
                        "[{}]({})\n{}",
                        current_title, current_url, current_content
                    ));
                }
                current_title = line.strip_prefix("Title:").unwrap_or("").trim().to_string();
            } else if line.starts_with("URL:") {
                current_url = line.strip_prefix("URL:").unwrap_or("").trim().to_string();
            } else if line.starts_with("Content:") {
                current_content = line.strip_prefix("Content:").unwrap_or("").trim().to_string();
            } else if !line.is_empty() && !current_content.is_empty() {
                current_content.push('\n');
                current_content.push_str(line);
            }
        }

        // 添加最后一个结果
        if !current_title.is_empty() {
            results.push(format!(
                "[{}]({})\n{}",
                current_title, current_url, current_content
            ));
        }

        if results.is_empty() {
            return "No results found.".to_string();
        }

        // 添加编号
        results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}", i + 1, r))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// 执行批量搜索
    pub async fn search_multiple(&self, queries: &[String]) -> Result<String> {
        if queries.is_empty() {
            return Ok("No queries provided.".to_string());
        }

        // 并发执行多个搜索
        let futures: Vec<_> = queries.iter().map(|q| self.search(q)).collect();
        let results = futures::future::join_all(futures).await;

        let mut output = Vec::new();
        for (query, result) in queries.iter().zip(results.iter()) {
            let result_str = match result {
                Ok(s) => s.as_str(),
                Err(e) => &format!("Error: {}", e),
            };
            output.push(format!("## Search: {}\n{}", query, result_str));
        }

        Ok(output.join("\n\n---\n\n"))
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

/// 执行工具调用（符合框架接口）
pub async fn execute_web_search(params: Value) -> Result<Value> {
    let tool = WebSearchTool::new();

    let queries = if let Some(query) = params.get("query") {
        if let Some(arr) = query.as_array() {
            arr.iter()
                .filter_map(|q| q.as_str().map(String::from))
                .collect::<Vec<_>>()
        } else if let Some(q) = query.as_str() {
            vec![q.to_string()]
        } else {
            return Ok(json!({
                "success": false,
                "error": "query must be a string or array of strings"
            }));
        }
    } else {
        return Ok(json!({
            "success": false,
            "error": "query parameter is required"
        }));
    };

    let result = if queries.len() == 1 {
        tool.search(&queries[0]).await
    } else {
        tool.search_multiple(&queries).await
    };

    match result {
        Ok(text) => Ok(json!({
            "success": true,
            "results": text
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
    async fn test_jina_search() {
        let client = JinaSearchClient::new();
        let result = client.search("Rust programming").await;
        // 注意：这个测试需要网络连接
        if result.is_ok() {
            assert!(!result.unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn test_duckduckgo_search() {
        let client = DuckDuckGoClient::new();
        let result = client.search("Rust programming").await;
        // 注意：这个测试需要网络连接
        if result.is_ok() {
            assert!(!result.unwrap().is_empty());
        }
    }

    #[test]
    fn test_web_search_tool_creation() {
        let _tool = WebSearchTool::new();
        // 测试工具创建成功
    }
}
