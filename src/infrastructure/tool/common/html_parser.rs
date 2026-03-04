//! HTML 解析工具
//!
//! 提供简化的 HTML 解析功能

use regex::Regex;

/// HTML 解析器
#[derive(Debug, Clone)]
pub struct HtmlParser {
    /// 是否移除脚本和样式
    pub strip_scripts: bool,
    /// 是否移除注释
    pub strip_comments: bool,
}

impl Default for HtmlParser {
    fn default() -> Self {
        Self {
            strip_scripts: true,
            strip_comments: true,
        }
    }
}

impl HtmlParser {
    /// 创建新的 HTML 解析器
    pub fn new() -> Self {
        Self::default()
    }

    /// 解析 HTML 并提取纯文本
    pub fn parse_to_text(&self, html: &str) -> String {
        let mut text = html.to_string();

        // 移除脚本标签
        if self.strip_scripts {
            text = remove_script_tags(&text);
        }

        // 移除样式标签
        text = remove_style_tags(&text);

        // 移除 HTML 注释
        if self.strip_comments {
            text = remove_html_comments(&text);
        }

        // 移除所有 HTML 标签
        text = strip_html_tags(&text);

        // 解码 HTML 实体
        text = decode_html_entities(&text);

        // 清理空白
        cleanup_whitespace(&text)
    }

    /// 提取标题
    pub fn extract_title(&self, html: &str) -> Option<String> {
        let title_regex = Regex::new(r"<title[^>]*>(.*?)</title>").unwrap();
        title_regex
            .captures(html)
            .and_then(|cap| cap.get(1))
            .map(|m| strip_html_tags(m.as_str()).to_string())
    }
}

fn remove_script_tags(html: &str) -> String {
    let script_regex = Regex::new(r"(?s)<script[^>]*>.*?</script>").unwrap();
    script_regex.replace_all(html, "").to_string()
}

fn remove_style_tags(html: &str) -> String {
    let style_regex = Regex::new(r"(?s)<style[^>]*>.*?</style>").unwrap();
    style_regex.replace_all(html, "").to_string()
}

fn remove_html_comments(html: &str) -> String {
    let comment_regex = Regex::new(r"(?s)<!--.*?-->").unwrap();
    comment_regex.replace_all(html, "").to_string()
}

fn strip_html_tags(html: &str) -> String {
    let tag_regex = Regex::new(r"<[^>]*>").unwrap();
    tag_regex.replace_all(html, "").to_string()
}

fn decode_html_entities(text: &str) -> String {
    let mut result = text.to_string();

    let entities = [
        ("&nbsp;", " "),
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&apos;", "'"),
        ("&hellip;", "..."),
        ("&mdash;", "—"),
        ("&ndash;", "–"),
    ];

    for (entity, replacement) in entities.iter() {
        result = result.replace(entity, replacement);
    }

    result
}

fn cleanup_whitespace(text: &str) -> String {
    let whitespace_regex = Regex::new(r"\s+").unwrap();
    whitespace_regex.replace_all(text, " ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_to_text() {
        let html = r#"<html><head><title>Test</title></head><body><p>Hello <b>World</b></p></body></html>"#;
        let parser = HtmlParser::new();
        let text = parser.parse_to_text(html);
        assert!(text.contains("Test"));
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }
}
