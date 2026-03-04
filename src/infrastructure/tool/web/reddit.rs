//! Reddit 工具 - 爬取 old.reddit.com 获取帖子
//!
//! 使用旧版界面因为结构更简单，反爬更宽松

use anyhow::Result;
use serde_json::{json, Value};

/// Reddit 工具
pub struct RedditTool {
    client: reqwest::Client,
    base_url: String,
}

impl RedditTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            base_url: "https://old.reddit.com".to_string(),
        }
    }

    /// 搜索帖子
    pub async fn search(&self, subreddit: Option<&str>, query: &str, limit: usize) -> Result<String> {
        let url = match subreddit {
            Some(sub) => format!(
                "{}/r/{}/search?q={}&limit={}&restrict_sr=1&sort=relevance",
                self.base_url,
                sub,
                urlencoding::encode(query),
                limit
            ),
            None => format!(
                "{}/search?q={}&limit={}&restrict_sr=0&sort=relevance",
                self.base_url,
                urlencoding::encode(query),
                limit
            ),
        };

        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;

        self.parse_search_results(&html)
    }

    /// 获取板块热门帖子
    pub async fn get_hot_posts(&self, subreddit: &str, limit: usize) -> Result<String> {
        let url = format!("{}/r/{}/hot.json?limit={}", self.base_url, subreddit, limit);

        let response = self.client.get(&url).send().await?;
        let json: Value = response.json().await?;

        self.format_hot_posts(&json)
    }

    /// 获取板块最新帖子
    pub async fn get_new_posts(&self, subreddit: &str, limit: usize) -> Result<String> {
        let url = format!("{}/r/{}/new.json?limit={}", self.base_url, subreddit, limit);

        let response = self.client.get(&url).send().await?;
        let json: Value = response.json().await?;

        self.format_posts(&json, "new")
    }

    /// 解析搜索结果 HTML
    fn parse_search_results(&self, html: &str) -> Result<String> {
        let document = scraper::Html::parse_document(html);
        let mut results = Vec::new();

        // old.reddit.com 使用 thing 类标识帖子
        let post_selector = scraper::Selector::parse(".thing").unwrap();
        let title_selector = scraper::Selector::parse("a.title").unwrap();
        let score_selector = scraper::Selector::parse(".score").unwrap();
        let domain_selector = scraper::Selector::parse(".domain").unwrap();

        for post in document.select(&post_selector).take(20) {
            if let Some(title_elem) = post.select(&title_selector).next() {
                let title = title_elem.text().collect::<String>().trim().to_string();
                let url = title_elem.value().attr("href").unwrap_or("").to_string();

                let score = post.select(&score_selector)
                    .next()
                    .map(|e| e.text().collect::<String>())
                    .unwrap_or_default();

                let domain = post.select(&domain_selector)
                    .next()
                    .map(|e| e.text().collect::<String>())
                    .unwrap_or_default();

                let full_url = if url.starts_with("http") {
                    url
                } else {
                    format!("{}{}", self.base_url, url)
                };

                results.push(format!(
                    "**{}**\n- Score: {}\n- Domain: {}\n- URL: {}",
                    title, score, domain, full_url
                ));
            }
        }

        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        Ok(results.join("\n\n"))
    }

    /// 格式化热门帖子（JSON 响应）
    fn format_hot_posts(&self, json: &Value) -> Result<String> {
        self.format_posts(json, "hot")
    }

    /// 格式化帖子
    fn format_posts(&self, json: &Value, sort_type: &str) -> Result<String> {
        let mut results = Vec::new();

        if let Some(posts) = json.get("data")
            .and_then(|d| d.get("children"))
            .and_then(|c| c.as_array())
        {
            for (idx, post) in posts.iter().take(20).enumerate() {
                let data = post.get("data").unwrap_or(&Value::Null);

                let title = data.get("title").and_then(|t| t.as_str()).unwrap_or("");
                let author = data.get("author").and_then(|a| a.as_str()).unwrap_or("[deleted]");
                let score = data.get("score").and_then(|s| s.as_u64()).unwrap_or(0);
                let num_comments = data.get("num_comments").and_then(|c| c.as_u64()).unwrap_or(0);
                let url = data.get("url").and_then(|u| u.as_str()).unwrap_or("");
                let permalink = data.get("permalink").and_then(|p| p.as_str()).unwrap_or("");
                let subreddit = data.get("subreddit").and_then(|s| s.as_str()).unwrap_or("");

                results.push(format!(
                    "{}. **{}**\n   - Author: u/{}\n   - Score: {} | Comments: {}\n   - Subreddit: r/{}\n   - URL: {}{}",
                    idx + 1,
                    title,
                    author,
                    score,
                    num_comments,
                    subreddit,
                    self.base_url,
                    permalink
                ));
            }
        }

        if results.is_empty() {
            return Ok(format!("No {} posts found.", sort_type));
        }

        Ok(results.join("\n\n"))
    }
}

impl Default for RedditTool {
    fn default() -> Self {
        Self::new()
    }
}

/// 执行工具调用
pub async fn execute_reddit(params: Value) -> Result<Value> {
    let tool = RedditTool::new();

    let action = params.get("action")
        .and_then(|a| a.as_str())
        .unwrap_or("search");

    match action {
        "search" => {
            let query = params.get("query")
                .and_then(|q| q.as_str())
                .ok_or_else(|| anyhow::anyhow!("query parameter is required"))?;

            let subreddit = params.get("subreddit").and_then(|s| s.as_str());
            let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

            match tool.search(subreddit, query, limit).await {
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
        "hot" => {
            let subreddit = params.get("subreddit")
                .and_then(|s| s.as_str())
                .ok_or_else(|| anyhow::anyhow!("subreddit parameter is required"))?;

            let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

            match tool.get_hot_posts(subreddit, limit).await {
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
        "new" => {
            let subreddit = params.get("subreddit")
                .and_then(|s| s.as_str())
                .ok_or_else(|| anyhow::anyhow!("subreddit parameter is required"))?;

            let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

            match tool.get_new_posts(subreddit, limit).await {
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
            "error": format!("Unknown action: {}. Supported actions: search, hot, new", action)
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hot_posts() {
        let tool = RedditTool::new();
        let result = tool.get_hot_posts("technology", 5).await;
        if result.is_ok() {
            let content = result.unwrap();
            assert!(!content.is_empty());
        }
    }

    #[test]
    fn test_tool_creation() {
        let _tool = RedditTool::new();
    }
}
