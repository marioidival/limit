use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

/// Web fetch tool for retrieving and parsing web content
pub struct WebFetchTool {
    client: Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    const MAX_SIZE: usize = 5 * 1024 * 1024; // 5MB
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::ToolError("Missing 'url' argument".to_string()))?;

        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("markdown");

        // Validate URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(AgentError::ToolError(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        // Fetch the URL
        let response = self
            .client
            .get(url)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,text/markdown,text/plain,*/*;q=0.8",
            )
            .send()
            .await
            .map_err(|e| AgentError::ToolError(format!("Request failed: {}", e)))?;

        // Check status
        if !response.status().is_success() {
            return Err(AgentError::ToolError(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        // Check content length
        if let Some(content_length) = response.headers().get("content-length") {
            if let Ok(length_str) = content_length.to_str() {
                if let Ok(length) = length_str.parse::<usize>() {
                    if length > Self::MAX_SIZE {
                        return Err(AgentError::ToolError(format!(
                            "Response too large: {} bytes (max: {})",
                            length,
                            Self::MAX_SIZE
                        )));
                    }
                }
            }
        }

        // Get content type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/plain")
            .to_string();

        // Get body
        let body = response
            .text()
            .await
            .map_err(|e| AgentError::ToolError(format!("Failed to read response: {}", e)))?;

        // Check actual size
        if body.len() > Self::MAX_SIZE {
            return Err(AgentError::ToolError(format!(
                "Response too large: {} bytes (max: {})",
                body.len(),
                Self::MAX_SIZE
            )));
        }

        // Process based on format and content type
        let output = if content_type.contains("text/html") {
            match format {
                "markdown" => html_to_markdown(&body),
                "text" => html_to_text(&body),
                "html" => body,
                _ => html_to_markdown(&body),
            }
        } else {
            body
        };

        Ok(serde_json::json!({
            "url": url,
            "content_type": content_type,
            "format": format,
            "content": output
        }))
    }
}

/// Convert HTML to Markdown (simple implementation)
fn html_to_markdown(html: &str) -> String {
    let mut text = html.to_string();

    // Remove script, style, nav, footer, header tags
    let remove_patterns = [
        r"<script[^>]*>.*?</script>",
        r"<style[^>]*>.*?</style>",
        r"<nav[^>]*>.*?</nav>",
        r"<footer[^>]*>.*?</footer>",
        r"<header[^>]*>.*?</header>",
        r"<!--.*?-->",
    ];

    for pattern in &remove_patterns {
        if let Ok(re) = Regex::new(pattern) {
            text = re.replace_all(&text, "").to_string();
        }
    }

    // Convert headings
    for i in 1..=6 {
        if let Ok(re) = Regex::new(&format!(r"<h{0}[^>]*>(.*?)</h{0}>", i)) {
            text = re
                .replace_all(&text, |caps: &regex::Captures| {
                    format!("{} {}\n\n", "#".repeat(i), &caps[1])
                })
                .to_string();
        }
    }

    // Convert links
    if let Ok(re) = Regex::new(r#"<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#) {
        text = re
            .replace_all(&text, |caps: &regex::Captures| {
                format!("[{}]({})", &caps[2], &caps[1])
            })
            .to_string();
    }

    // Convert paragraphs
    if let Ok(re) = Regex::new(r"<p[^>]*>(.*?)</p>") {
        text = re
            .replace_all(&text, |caps: &regex::Captures| format!("{}\n\n", &caps[1]))
            .to_string();
    }

    // Convert line breaks
    if let Ok(re) = Regex::new(r"<br\s*/?>") {
        text = re.replace_all(&text, "\n").to_string();
    }

    // Convert code blocks
    if let Ok(re) = Regex::new(r"<pre[^>]*><code[^>]*>(.*?)</code></pre>") {
        text = re
            .replace_all(&text, |caps: &regex::Captures| {
                format!("```\n{}\n```\n\n", &caps[1])
            })
            .to_string();
    }

    // Convert inline code
    if let Ok(re) = Regex::new(r"<code[^>]*>(.*?)</code>") {
        text = re
            .replace_all(&text, |caps: &regex::Captures| format!("`{}`", &caps[1]))
            .to_string();
    }

    // Convert strong/bold
    if let Ok(re) = Regex::new(r"<strong[^>]*>(.*?)</strong>") {
        text = re
            .replace_all(&text, |caps: &regex::Captures| format!("**{}**", &caps[1]))
            .to_string();
    }
    if let Ok(re) = Regex::new(r"<b[^>]*>(.*?)</b>") {
        text = re
            .replace_all(&text, |caps: &regex::Captures| format!("**{}**", &caps[1]))
            .to_string();
    }

    // Convert em/italic
    if let Ok(re) = Regex::new(r"<em[^>]*>(.*?)</em>") {
        text = re
            .replace_all(&text, |caps: &regex::Captures| format!("*{}*", &caps[1]))
            .to_string();
    }
    if let Ok(re) = Regex::new(r"<i[^>]*>(.*?)</i>") {
        text = re
            .replace_all(&text, |caps: &regex::Captures| format!("*{}*", &caps[1]))
            .to_string();
    }

    // Convert lists
    if let Ok(re) = Regex::new(r"<li[^>]*>(.*?)</li>") {
        text = re
            .replace_all(&text, |caps: &regex::Captures| format!("- {}\n", &caps[1]))
            .to_string();
    }

    // Remove remaining HTML tags
    if let Ok(re) = Regex::new(r"<[^>]+>") {
        text = re.replace_all(&text, "").to_string();
    }

    // Decode HTML entities
    text = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    // Clean up whitespace
    clean_whitespace(&text)
}

/// Convert HTML to plain text
fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();

    // Remove script, style, nav, footer, header tags
    let remove_patterns = [
        r"<script[^>]*>.*?</script>",
        r"<style[^>]*>.*?</style>",
        r"<nav[^>]*>.*?</nav>",
        r"<footer[^>]*>.*?</footer>",
        r"<header[^>]*>.*?</header>",
        r"<!--.*?-->",
    ];

    for pattern in &remove_patterns {
        if let Ok(re) = Regex::new(pattern) {
            text = re.replace_all(&text, "").to_string();
        }
    }

    // Convert block elements to newlines
    let block_patterns = [r"</p>", r"</div>", r"</h[1-6]>", r"</li>", r"<br\s*/?>"];
    for pattern in &block_patterns {
        if let Ok(re) = Regex::new(pattern) {
            text = re.replace_all(&text, "\n").to_string();
        }
    }

    // Remove remaining HTML tags
    if let Ok(re) = Regex::new(r"<[^>]+>") {
        text = re.replace_all(&text, "").to_string();
    }

    // Decode HTML entities
    text = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    clean_whitespace(&text)
}

/// Clean up whitespace in text
fn clean_whitespace(text: &str) -> String {
    // Replace multiple spaces with single space
    let re = Regex::new(r" {2,}").unwrap();
    let mut text = re.replace_all(text, " ").to_string();

    // Replace more than 2 newlines with 2 newlines
    let re = Regex::new(r"\n{3,}").unwrap();
    text = re.replace_all(&text, "\n\n").to_string();

    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_fetch_tool_name() {
        let tool = WebFetchTool::new();
        assert_eq!(tool.name(), "web_fetch");
    }

    #[test]
    fn test_web_fetch_tool_default() {
        let tool = WebFetchTool::new();
        assert_eq!(tool.name(), "web_fetch");
    }

    #[tokio::test]
    async fn test_web_fetch_missing_url() {
        let tool = WebFetchTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'url'"));
    }

    #[tokio::test]
    async fn test_web_fetch_invalid_url() {
        let tool = WebFetchTool::new();
        let args = serde_json::json!({
            "url": "ftp://example.com"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("http:// or https://"));
    }

    #[test]
    fn test_html_to_markdown() {
        let html = r#"<h1>Title</h1><p>This is <strong>bold</strong> text.</p>"#;
        let markdown = html_to_markdown(html);
        assert!(markdown.contains("# Title"));
        assert!(markdown.contains("**bold**"));
    }

    #[test]
    fn test_html_to_text() {
        let html = r#"<p>Hello</p><p>World</p>"#;
        let text = html_to_text(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn test_clean_whitespace() {
        let text = "Hello   World\n\n\n\nTest";
        let cleaned = clean_whitespace(text);
        assert!(!cleaned.contains("   "));
        assert!(!cleaned.contains("\n\n\n"));
    }
}
