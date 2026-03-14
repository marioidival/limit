//! Browser tool for LLM agent
//!
//! Implements the Tool trait for browser automation in the agent system.

use super::client::BrowserClient;
use super::config::{BrowserConfig, BrowserEngine};
use super::executor::{BrowserExecutor, CliExecutor};
use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use serde_json::Value;
use std::sync::Arc;

/// Browser automation tool for the LLM agent
pub struct BrowserTool {
    client: BrowserClient,
}

impl BrowserTool {
    /// Create a new browser tool with default configuration
    pub fn new() -> Self {
        Self::with_config(BrowserConfig::default())
    }

    /// Create a browser tool with custom configuration
    pub fn with_config(config: BrowserConfig) -> Self {
        let executor = Arc::new(CliExecutor::new(config));
        Self {
            client: BrowserClient::new(executor),
        }
    }

    /// Create a browser tool with a custom executor (for testing)
    pub fn with_executor(executor: Arc<dyn BrowserExecutor>) -> Self {
        Self {
            client: BrowserClient::new(executor),
        }
    }

    /// Parse engine from string
    #[allow(dead_code)]
    fn parse_engine(s: &str) -> Option<BrowserEngine> {
        match s.to_lowercase().as_str() {
            "chrome" => Some(BrowserEngine::Chrome),
            "lightpanda" => Some(BrowserEngine::Lightpanda),
            _ => None,
        }
    }

    /// Handle the open action
    async fn handle_open(&self, args: &Value) -> Result<Value, AgentError> {
        let url = args.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'url' argument for open action".to_string())
        })?;

        self.client
            .open(url)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Opened {}", url)
        }))
    }

    /// Handle the close action
    async fn handle_close(&self) -> Result<Value, AgentError> {
        self.client
            .close()
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Browser closed"
        }))
    }

    /// Handle the snapshot action
    async fn handle_snapshot(&self) -> Result<Value, AgentError> {
        let result = self
            .client
            .snapshot()
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "content": result.content,
            "title": result.title,
            "url": result.url
        }))
    }

    /// Handle the click action
    async fn handle_click(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for click action".to_string())
            })?;

        self.client
            .click(selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Clicked element: {}", selector)
        }))
    }

    /// Handle the fill action
    async fn handle_fill(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for fill action".to_string())
            })?;

        let text = args.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'text' argument for fill action".to_string())
        })?;

        self.client
            .fill(selector, text)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Filled element {} with text", selector)
        }))
    }

    /// Handle the screenshot action
    async fn handle_screenshot(&self, args: &Value) -> Result<Value, AgentError> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'path' argument for screenshot action".to_string())
        })?;

        self.client
            .screenshot(path)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Screenshot saved to {}", path)
        }))
    }

    /// Handle the wait action
    async fn handle_wait(&self, args: &Value) -> Result<Value, AgentError> {
        let condition = args
            .get("wait_for")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'wait_for' argument for wait action".to_string())
            })?;

        self.client
            .wait_for(condition)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Wait condition met: {}", condition)
        }))
    }

    /// Handle the eval action
    async fn handle_eval(&self, args: &Value) -> Result<Value, AgentError> {
        let script = args.get("script").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'script' argument for eval action".to_string())
        })?;

        let result = self
            .client
            .eval(script)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "result": result
        }))
    }

    /// Handle the get action
    async fn handle_get(&self, args: &Value) -> Result<Value, AgentError> {
        let what = args
            .get("get_what")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'get_what' argument for get action".to_string())
            })?;

        let content = self
            .client
            .get(what)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "content": content
        }))
    }
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for BrowserTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserTool").finish()
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        // Extract action from arguments
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::ToolError("Missing 'action' argument".to_string()))?;

        match action {
            "open" => self.handle_open(&args).await,
            "close" => self.handle_close().await,
            "snapshot" => self.handle_snapshot().await,
            "click" => self.handle_click(&args).await,
            "fill" => self.handle_fill(&args).await,
            "screenshot" => self.handle_screenshot(&args).await,
            "wait" => self.handle_wait(&args).await,
            "eval" => self.handle_eval(&args).await,
            "get" => self.handle_get(&args).await,
            _ => Err(AgentError::ToolError(format!(
                "Unknown browser action: {}. Valid actions: open, close, snapshot, click, fill, screenshot, wait, eval, get",
                action
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_tool_name() {
        let tool = BrowserTool::new();
        assert_eq!(tool.name(), "browser");
    }

    #[test]
    fn test_browser_tool_default() {
        let tool = BrowserTool::default();
        assert_eq!(tool.name(), "browser");
    }

    #[test]
    fn test_parse_engine() {
        assert_eq!(
            BrowserTool::parse_engine("chrome"),
            Some(BrowserEngine::Chrome)
        );
        assert_eq!(
            BrowserTool::parse_engine("CHROME"),
            Some(BrowserEngine::Chrome)
        );
        assert_eq!(
            BrowserTool::parse_engine("lightpanda"),
            Some(BrowserEngine::Lightpanda)
        );
        assert_eq!(
            BrowserTool::parse_engine("LightPanda"),
            Some(BrowserEngine::Lightpanda)
        );
        assert_eq!(BrowserTool::parse_engine("invalid"), None);
    }

    #[tokio::test]
    async fn test_browser_tool_missing_action() {
        let tool = BrowserTool::new();
        let args = serde_json::json!({
            "url": "https://example.com"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'action'"));
    }

    #[tokio::test]
    async fn test_browser_tool_unknown_action() {
        let tool = BrowserTool::new();
        let args = serde_json::json!({
            "action": "unknown"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unknown browser action"));
    }

    #[tokio::test]
    async fn test_browser_tool_open_missing_url() {
        let tool = BrowserTool::new();
        let args = serde_json::json!({
            "action": "open"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'url'"));
    }

    #[tokio::test]
    async fn test_browser_tool_click_missing_selector() {
        let tool = BrowserTool::new();
        let args = serde_json::json!({
            "action": "click"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'selector'"));
    }

    #[tokio::test]
    async fn test_browser_tool_screenshot_missing_path() {
        let tool = BrowserTool::new();
        let args = serde_json::json!({
            "action": "screenshot"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'path'"));
    }
}
