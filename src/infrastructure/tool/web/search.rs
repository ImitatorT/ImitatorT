//! Web 搜索工具 - SearXNG + Brave Search
//!
//! 支持的搜索引擎：
//! - SearXNG（首选，开源元搜索引擎）
//! - Brave Search（备用，需配置 API Key）

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::sync::Arc;

use crate::infrastructure::tool::common::{ApiKeyPool, SimpleRateLimiter};

/// SearXNG 搜索结果项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearXngResult {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub engine: String,
}

/// SearXNG 搜索响应
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearXngResponse {
    pub query: String,
    pub results: Vec<SearXngResult>,
    #[serde(default)]
    pub number_of_results: usize,
}

/// SearXNG 客户端
pub struct SearXngClient {
    client: reqwest::Client,
    instance_url: String,
    rate_limiter: Arc<SimpleRateLimiter>,
}

impl SearXngClient {
    pub fn new() -> Self {
        let instance_url = env::var("SEARXNG_INSTANCE_URL")
            .unwrap_or_else(|_| "https://searx.be".to_string());

        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; ImitatorT/1.0)")
                .build()
                .unwrap_or_default(),
            instance_url,
            rate_limiter: Arc::new(SimpleRateLimiter::new(2)), // 2 请求/秒
        }
    }

    /// 执行搜索
    pub async fn search(&self, query: &str) -> Result<String> {
        // 限流
        self.rate_limiter.wait().await;

        let encoded_query = urlencoding::encode(query);
        let url = format!(
            "{}/search?q={}&format=json",
            self.instance_url, encoded_query
        );

        let response = self.client.get(&url).send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "SearXNG API error: {} {}",
                status,
                text
            ));
        }

        // 解析 JSON 响应并格式化
        self.format_searxng_response(&text)
    }

    /// 格式化 SearXNG 响应
    fn format_searxng_response(&self, json_str: &str) -> Result<String> {
        let response: SearXngResponse = serde_json::from_str(json_str)?;
        let mut results = Vec::new();

        for (idx, item) in response.results.iter().take(10).enumerate() {
            let title = &item.title;
            let url = &item.url;
            let content = &item.content;

            if content.is_empty() {
                results.push(format!(
                    "{}. [{}]({})",
                    idx + 1,
                    title,
                    url
                ));
            } else {
                results.push(format!(
                    "{}. [{}]({})\n   {}",
                    idx + 1,
                    title,
                    url,
                    content
                ));
            }
        }

        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        Ok(results.join("\n\n"))
    }
}

impl Default for SearXngClient {
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

/// Web 搜索工具 - SearXNG + Brave Search
pub struct WebSearchTool {
    searxng_client: SearXngClient,
    brave_client: Option<BraveSearchClient>,
}

impl WebSearchTool {
    pub fn new() -> Self {
        // 检查是否配置了 Brave API Key
        let has_brave_keys = env::var("BRAVE_API_KEYS").is_ok();

        Self {
            searxng_client: SearXngClient::new(),
            brave_client: if has_brave_keys {
                Some(BraveSearchClient::new())
            } else {
                None
            },
        }
    }

    /// 执行搜索（SearXNG 优先，Brave Search 备用）
    pub async fn search(&self, query: &str) -> Result<String> {
        // 1. 首先尝试 SearXNG
        match self.searxng_client.search(query).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::warn!("SearXNG search failed: {}", e);
            }
        }

        // 2. 如果 SearXNG 失败且有 Brave 客户端，尝试 Brave Search
        if let Some(ref brave) = self.brave_client {
            match brave.search(query).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("Brave Search failed: {}", e);
                }
            }
        }

        // 都失败
        Err(anyhow::anyhow!(
            "All search engines failed. SearXNG failed, and Brave Search is {}",
            if self.brave_client.is_some() {
                "also failed"
            } else {
                "not configured (set BRAVE_API_KEYS to enable)"
            }
        ))
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
    async fn test_searxng_search() {
        let client = SearXngClient::new();
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
