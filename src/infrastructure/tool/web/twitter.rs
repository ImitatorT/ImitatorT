//! Twitter/X 工具 - 获取推文和用户信息
//!
//! 使用 Nitter 实例（隐私友好的 Twitter 前端）或直接爬取

use anyhow::Result;
use serde_json::{json, Value};

/// Nitter 实例列表（公共实例）
const NITTER_INSTANCES: &[&str] = &[
    "https://nitter.net",
    "https://nitter.privacy.com.de",
    "https://nitter.dark.fail",
];

/// Twitter 工具
pub struct TwitterTool {
    client: reqwest::Client,
    nitter_instance: String,
}

impl TwitterTool {
    pub fn new() -> Self {
        // 默认使用主 Nitter 实例
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            nitter_instance: NITTER_INSTANCES[0].to_string(),
        }
    }

    /// 获取用户推文
    pub async fn get_user_tweets(&self, username: &str, limit: usize) -> Result<String> {
        // 尝试 Nitter 实例
        for instance in NITTER_INSTANCES {
            match self.get_tweets_from_nitter(instance, username, limit).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("Nitter instance {} failed: {}", instance, e);
                    continue;
                }
            }
        }

        // 所有 Nitter 实例都失败，尝试直接爬取 Twitter Mobile
        self.get_tweets_from_mobile_twitter(username, limit).await
    }

    /// 从 Nitter 获取推文
    async fn get_tweets_from_nitter(&self, instance: &str, username: &str, limit: usize) -> Result<String> {
        let url = format!("{}/{}?format=html", instance.trim_end_matches('/'), username);

        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;

        self.parse_nitter_tweets(&html, limit)
    }

    /// 解析 Nitter 推文
    fn parse_nitter_tweets(&self, html: &str, limit: usize) -> Result<String> {
        let document = scraper::Html::parse_document(html);
        let mut tweets = Vec::new();

        // Nitter 使用 stream-item 类标识推文
        let tweet_selector = scraper::Selector::parse(".stream-item").unwrap();
        let content_selector = scraper::Selector::parse(".tweet-content").unwrap();
        let date_selector = scraper::Selector::parse("a.tweet-date").unwrap();
        let stats_selector = scraper::Selector::parse(".tweet-stats").unwrap();

        for (idx, tweet_elem) in document.select(&tweet_selector).take(limit).enumerate() {
            let content = tweet_elem.select(&content_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let date = tweet_elem.select(&date_selector)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();

            let stats = tweet_elem.select(&stats_selector)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();

            if !content.is_empty() {
                tweets.push(format!(
                    "{}. {}\n   - Date: {}\n   - Stats: {}",
                    idx + 1,
                    content,
                    date.trim(),
                    stats.trim()
                ));
            }
        }

        if tweets.is_empty() {
            return Ok("No tweets found or unable to parse.".to_string());
        }

        Ok(tweets.join("\n\n"))
    }

    /// 从 Twitter Mobile 获取推文（备用方案）
    async fn get_tweets_from_mobile_twitter(&self, username: &str, limit: usize) -> Result<String> {
        let url = format!("https://mobile.twitter.com/{}", username);

        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;

        // 解析 Mobile Twitter HTML
        self.parse_mobile_twitter(&html, limit)
    }

    /// 解析 Mobile Twitter HTML
    fn parse_mobile_twitter(&self, html: &str, _limit: usize) -> Result<String> {
        let document = scraper::Html::parse_document(html);
        let mut tweets = Vec::new();

        // Mobile Twitter 使用 article 标签标识推文
        let article_selector = scraper::Selector::parse("article").unwrap();

        for article in document.select(&article_selector).take(20) {
            // 提取推文文本
            let text = article.text().collect::<String>();
            if !text.trim().is_empty() {
                tweets.push(text.trim().to_string());
            }
        }

        if tweets.is_empty() {
            return Ok("No tweets found. Twitter may have blocked the request.".to_string());
        }

        Ok(tweets.join("\n\n---\n\n"))
    }

    /// 获取用户信息
    pub async fn get_user_info(&self, username: &str) -> Result<String> {
        let url = format!("{}/{}", self.nitter_instance, username);

        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;

        self.parse_nitter_profile(&html)
    }

    /// 解析 Nitter 用户资料
    fn parse_nitter_profile(&self, html: &str) -> Result<String> {
        let document = scraper::Html::parse_document(html);
        let mut info = Vec::new();

        // 用户名
        let name_selector = scraper::Selector::parse(".profile-card-fullname").unwrap();
        if let Some(elem) = document.select(&name_selector).next() {
            info.push(format!("Name: {}", elem.text().collect::<String>().trim()));
        }

        // 简介
        let bio_selector = scraper::Selector::parse(".profile-bio").unwrap();
        if let Some(elem) = document.select(&bio_selector).next() {
            info.push(format!("Bio: {}", elem.text().collect::<String>().trim()));
        }

        // 关注数据
        let joining_selector = scraper::Selector::parse(".profile-joindate").unwrap();
        if let Some(elem) = document.select(&joining_selector).next() {
            info.push(format!("Joined: {}", elem.text().collect::<String>().trim()));
        }

        if info.is_empty() {
            return Ok("User not found or profile unavailable.".to_string());
        }

        Ok(info.join("\n"))
    }
}

impl Default for TwitterTool {
    fn default() -> Self {
        Self::new()
    }
}

/// 执行工具调用
pub async fn execute_twitter(params: Value) -> Result<Value> {
    let tool = TwitterTool::new();

    let action = params.get("action")
        .and_then(|a| a.as_str())
        .unwrap_or("tweets");

    match action {
        "tweets" | "get_user_tweets" => {
            let username = params.get("username")
                .and_then(|u| u.as_str())
                .ok_or_else(|| anyhow::anyhow!("username parameter is required"))?;

            let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

            match tool.get_user_tweets(username, limit).await {
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
        "info" | "get_user_info" => {
            let username = params.get("username")
                .and_then(|u| u.as_str())
                .ok_or_else(|| anyhow::anyhow!("username parameter is required"))?;

            match tool.get_user_info(username).await {
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
            "error": format!("Unknown action: {}. Supported actions: tweets, info", action)
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let _tool = TwitterTool::new();
    }
}
