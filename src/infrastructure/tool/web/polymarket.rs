//! Polymarket 工具 - 获取预测市场数据
//!
//! 使用 Polymarket 官方 API
//! API 文档：https://polymarket.github.io/polymarket-docs/

use anyhow::Result;
use serde_json::{json, Value};

/// Polymarket 工具
pub struct PolymarketTool {
    client: reqwest::Client,
    base_url: String,
}

impl PolymarketTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; ImitatorT/1.0)")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            base_url: "https://polymarket.com".to_string(),
        }
    }

    /// 搜索市场
    pub async fn search_markets(&self, query: &str, limit: usize) -> Result<String> {
        // Polymarket 没有公开搜索 API，使用网页爬取
        let url = format!("{}/search?q={}", self.base_url, urlencoding::encode(query));

        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;

        self.parse_search_results(&html, limit)
    }

    /// 获取热门市场
    pub async fn get_trending_markets(&self, limit: usize) -> Result<String> {
        let url = format!("{}/api/markets/trending", self.base_url);

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let json: Value = response.json().await?;
            return self.format_markets(&json, limit);
        }

        // API 失败时尝试爬取网页
        self.get_trending_from_web(limit).await
    }

    /// 获取特定市场详情
    pub async fn get_market(&self, market_id: &str) -> Result<String> {
        let url = format!("{}/api/markets/{}", self.base_url, market_id);

        let response = self.client.get(&url).send().await?;
        let json: Value = response.json().await?;

        self.format_market_detail(&json)
    }

    /// 获取市场赔率
    pub async fn get_market_odds(&self, market_id: &str) -> Result<String> {
        let url = format!("{}/api/markets/{}/odds", self.base_url, market_id);

        let response = self.client.get(&url).send().await?;
        let json: Value = response.json().await?;

        self.format_odds(&json)
    }

    /// 解析搜索结果（网页）
    fn parse_search_results(&self, html: &str, limit: usize) -> Result<String> {
        let document = scraper::Html::parse_document(html);
        let mut results = Vec::new();

        // 查找市场卡片
        let market_selector = scraper::Selector::parse("[data-testid='market-card']").unwrap();
        let title_selector = scraper::Selector::parse("[data-testid='market-title']").unwrap();
        let volume_selector = scraper::Selector::parse("[data-testid='market-volume']").unwrap();

        for (idx, market) in document.select(&market_selector).take(limit).enumerate() {
            let title = market.select(&title_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let volume = market.select(&volume_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if !title.is_empty() {
                let mut result = format!("{}. **{}**", idx + 1, title);
                if !volume.is_empty() {
                    result.push_str(&format!(" (Volume: {})", volume));
                }
                results.push(result);
            }
        }

        // 备用解析方法
        if results.is_empty() {
            return self.parse_with_links(html, limit);
        }

        if results.is_empty() {
            return Ok("No markets found.".to_string());
        }

        Ok(results.join("\n\n"))
    }

    /// 备用链接解析
    fn parse_with_links(&self, html: &str, limit: usize) -> Result<String> {
        let document = scraper::Html::parse_document(html);
        let mut results = Vec::new();

        let link_selector = scraper::Selector::parse("a").unwrap();

        for link in document.select(&link_selector) {
            let href = link.value().attr("href").unwrap_or("");
            let text = link.text().collect::<String>().trim().to_string();

            // 过滤市场链接
            if href.contains("/market/") && text.len() > 10 {
                results.push(format!("- [{}]({}{})", text, self.base_url, href));
                if results.len() >= limit {
                    break;
                }
            }
        }

        if results.is_empty() {
            return Ok("No markets found.".to_string());
        }

        Ok(results.join("\n"))
    }

    /// 从网页获取热门市场
    async fn get_trending_from_web(&self, limit: usize) -> Result<String> {
        let url = format!("{}/trending", self.base_url);

        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;

        self.parse_search_results(&html, limit)
    }

    /// 格式化市场列表
    fn format_markets(&self, json: &Value, limit: usize) -> Result<String> {
        let mut results = Vec::new();

        if let Some(markets) = json.as_array() {
            for (idx, market) in markets.iter().take(limit).enumerate() {
                let title = market.get("question")
                    .or_else(|| market.get("title"))
                    .and_then(|v: &Value| v.as_str())
                    .unwrap_or("Unknown");

                let volume = market.get("volume")
                    .and_then(|v: &Value| v.as_u64())
                    .unwrap_or(0);

                let url = market.get("url")
                    .and_then(|v: &Value| v.as_str())
                    .unwrap_or("");

                results.push(format!(
                    "{}. **{}**\n   Volume: ${}\n   URL: {}",
                    idx + 1,
                    title,
                    volume,
                    url
                ));
            }
        }

        if results.is_empty() {
            return Ok("No trending markets found.".to_string());
        }

        Ok(results.join("\n\n"))
    }

    /// 格式化市场详情
    fn format_market_detail(&self, json: &Value) -> Result<String> {
        let mut info = Vec::new();

        let title = json.get("question")
            .or_else(|| json.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        info.push(format!("**{}**", title));

        if let Some(volume) = json.get("volume").and_then(|v| v.as_u64()) {
            info.push(format!("Volume: ${}", volume));
        }

        if let Some(outcomes) = json.get("outcomes").and_then(|v| v.as_array()) {
            info.push("\nOutcomes:".to_string());
            for outcome in outcomes {
                let name = outcome.get("name").and_then(|s| s.as_str()).unwrap_or("");
                let price = outcome.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                info.push(format!("  - {}: {:.1}%", name, price * 100.0));
            }
        }

        Ok(info.join("\n"))
    }

    /// 格式化赔率
    fn format_odds(&self, json: &Value) -> Result<String> {
        let mut results = Vec::new();

        if let Some(outcomes) = json.get("outcomes").and_then(|v| v.as_array()) {
            for outcome in outcomes {
                let name = outcome.get("name").and_then(|s| s.as_str()).unwrap_or("");
                let price = outcome.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let probability = price * 100.0;

                results.push(format!("{}: {:.1}% probability", name, probability));
            }
        }

        if results.is_empty() {
            return Ok("No odds data available.".to_string());
        }

        Ok(results.join("\n"))
    }
}

impl Default for PolymarketTool {
    fn default() -> Self {
        Self::new()
    }
}

/// 执行工具调用
pub async fn execute_polymarket(params: Value) -> Result<Value> {
    let tool = PolymarketTool::new();

    let action = params.get("action")
        .and_then(|a| a.as_str())
        .unwrap_or("search");

    match action {
        "search" => {
            let query = params.get("query")
                .and_then(|q| q.as_str())
                .ok_or_else(|| anyhow::anyhow!("query parameter is required"))?;

            let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

            match tool.search_markets(query, limit).await {
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
        "trending" => {
            let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

            match tool.get_trending_markets(limit).await {
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
        "market" => {
            let market_id = params.get("market_id")
                .and_then(|id| id.as_str())
                .ok_or_else(|| anyhow::anyhow!("market_id parameter is required"))?;

            match tool.get_market(market_id).await {
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
        "odds" => {
            let market_id = params.get("market_id")
                .and_then(|id| id.as_str())
                .ok_or_else(|| anyhow::anyhow!("market_id parameter is required"))?;

            match tool.get_market_odds(market_id).await {
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
            "error": format!("Unknown action: {}. Supported actions: search, trending, market, odds", action)
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let _tool = PolymarketTool::new();
    }
}
