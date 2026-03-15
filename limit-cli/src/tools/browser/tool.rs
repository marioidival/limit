//! Browser tool for LLM agent
//!
//! Implements the Tool trait for browser automation in the agent system.

use super::client::BrowserClient;
use super::client_ext::{InteractionExt, NavigationExt, QueryExt, StorageExt, TabsExt, WaitingExt};
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

    /// Handle the wait_for_text action
    async fn handle_wait_for_text(&self, args: &Value) -> Result<Value, AgentError> {
        let text = args.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'text' argument for wait_for_text action".to_string())
        })?;

        self.client
            .wait_for_text(text)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Text found: {}", text)
        }))
    }

    /// Handle the wait_for_url action
    async fn handle_wait_for_url(&self, args: &Value) -> Result<Value, AgentError> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError(
                    "Missing 'pattern' argument for wait_for_url action".to_string(),
                )
            })?;

        self.client
            .wait_for_url(pattern)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("URL pattern matched: {}", pattern)
        }))
    }

    /// Handle the wait_for_load action
    async fn handle_wait_for_load(&self, args: &Value) -> Result<Value, AgentError> {
        let state = args.get("state").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'state' argument for wait_for_load action".to_string())
        })?;

        self.client
            .wait_for_load(state)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Load state reached: {}", state)
        }))
    }

    /// Handle the wait_for_download action
    async fn handle_wait_for_download(&self, args: &Value) -> Result<Value, AgentError> {
        let path = args.get("path").and_then(|v| v.as_str());

        let download_path = self
            .client
            .wait_for_download(path)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "path": download_path,
            "message": "Download completed"
        }))
    }

    /// Handle the wait_for_fn action
    async fn handle_wait_for_fn(&self, args: &Value) -> Result<Value, AgentError> {
        let js = args.get("js").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'js' argument for wait_for_fn action".to_string())
        })?;

        self.client
            .wait_for_fn(js)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": "JavaScript condition met"
        }))
    }

    /// Handle the wait_for_state action
    async fn handle_wait_for_state(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError(
                    "Missing 'selector' argument for wait_for_state action".to_string(),
                )
            })?;

        let state = args.get("state").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'state' argument for wait_for_state action".to_string())
        })?;

        self.client
            .wait_for_state(selector, state)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Element {} reached state: {}", selector, state)
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

    /// Handle the get_attr action
    async fn handle_get_attr(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for get_attr action".to_string())
            })?;

        let attr = args.get("attr").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'attr' argument for get_attr action".to_string())
        })?;

        let value = self
            .client
            .get_attr(selector, attr)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "value": value,
            "message": format!("Attribute '{}' = '{}'", attr, value)
        }))
    }

    /// Handle the get_count action
    async fn handle_get_count(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError(
                    "Missing 'selector' argument for get_count action".to_string(),
                )
            })?;

        let count = self
            .client
            .get_count(selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "count": count,
            "message": format!("Found {} elements matching {}", count, selector)
        }))
    }

    /// Handle the get_box action
    async fn handle_get_box(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for get_box action".to_string())
            })?;

        let bbox = self
            .client
            .get_box(selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "x": bbox.x,
            "y": bbox.y,
            "width": bbox.width,
            "height": bbox.height,
            "message": format!("Element bounding box: ({}, {}) {}x{}", bbox.x, bbox.y, bbox.width, bbox.height)
        }))
    }

    /// Handle the get_styles action
    async fn handle_get_styles(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError(
                    "Missing 'selector' argument for get_styles action".to_string(),
                )
            })?;

        let styles = self
            .client
            .get_styles(selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "styles": styles,
            "message": format!("Got {} computed styles for {}", styles.len(), selector)
        }))
    }

    // ========================================
    // Navigation handlers
    // ========================================

    /// Handle the back action
    async fn handle_back(&self) -> Result<Value, AgentError> {
        self.client
            .back()
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Navigated back"
        }))
    }

    /// Handle the forward action
    async fn handle_forward(&self) -> Result<Value, AgentError> {
        self.client
            .forward()
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Navigated forward"
        }))
    }

    /// Handle the reload action
    async fn handle_reload(&self) -> Result<Value, AgentError> {
        self.client
            .reload()
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Page reloaded"
        }))
    }

    // ========================================
    // Input handlers
    // ========================================

    /// Handle the type action
    async fn handle_type(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for type action".to_string())
            })?;

        let text = args.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'text' argument for type action".to_string())
        })?;

        self.client
            .type_text(selector, text)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Typed text into element: {}", selector)
        }))
    }

    /// Handle the press action
    async fn handle_press(&self, args: &Value) -> Result<Value, AgentError> {
        let key = args.get("key").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'key' argument for press action".to_string())
        })?;

        self.client
            .press(key)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Pressed key: {}", key)
        }))
    }

    /// Handle the hover action
    async fn handle_hover(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for hover action".to_string())
            })?;

        self.client
            .hover(selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Hovered over element: {}", selector)
        }))
    }

    /// Handle the select action
    async fn handle_select(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for select action".to_string())
            })?;

        let value = args.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'value' argument for select action".to_string())
        })?;

        self.client
            .select_option(selector, value)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Selected option '{}' in element: {}", value, selector)
        }))
    }

    /// Handle the dblclick action
    async fn handle_dblclick(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for dblclick action".to_string())
            })?;

        self.client
            .dblclick(selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Double-clicked element: {}", selector)
        }))
    }

    /// Handle the focus action
    async fn handle_focus(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for focus action".to_string())
            })?;

        self.client
            .focus(selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Focused element: {}", selector)
        }))
    }

    /// Handle the check action
    async fn handle_check(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for check action".to_string())
            })?;

        self.client
            .check(selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Checked element: {}", selector)
        }))
    }

    /// Handle the uncheck action
    async fn handle_uncheck(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for uncheck action".to_string())
            })?;

        self.client
            .uncheck(selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Unchecked element: {}", selector)
        }))
    }

    /// Handle the scrollintoview action
    async fn handle_scrollintoview(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError(
                    "Missing 'selector' argument for scrollintoview action".to_string(),
                )
            })?;

        self.client
            .scrollintoview(selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Scrolled element into view: {}", selector)
        }))
    }

    /// Handle the drag action
    async fn handle_drag(&self, args: &Value) -> Result<Value, AgentError> {
        let source = args.get("source").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'source' argument for drag action".to_string())
        })?;

        let target = args.get("target").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'target' argument for drag action".to_string())
        })?;

        self.client
            .drag(source, target)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Dragged from {} to {}", source, target)
        }))
    }

    /// Handle the upload action
    async fn handle_upload(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for upload action".to_string())
            })?;

        let files = args
            .get("files")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'files' argument for upload action".to_string())
            })?;

        let file_paths: Vec<&str> = files.iter().filter_map(|f| f.as_str()).collect();

        if file_paths.is_empty() {
            return Err(AgentError::ToolError(
                "At least one file path required".to_string(),
            ));
        }

        self.client
            .upload(selector, &file_paths)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Uploaded {} file(s) to {}", file_paths.len(), selector)
        }))
    }

    /// Handle the pdf action
    async fn handle_pdf(&self, args: &Value) -> Result<Value, AgentError> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'path' argument for pdf action".to_string())
        })?;

        self.client
            .pdf(path)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Saved page as PDF: {}", path)
        }))
    }

    // ========================================
    // State handlers
    // ========================================

    /// Handle the find action
    async fn handle_find(&self, args: &Value) -> Result<Value, AgentError> {
        let locator = args
            .get("locator")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'locator' argument for find action".to_string())
            })?;

        let value = args.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'value' argument for find action".to_string())
        })?;

        let action = args.get("action").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'action' argument for find action".to_string())
        })?;

        let action_value = args.get("action_value").and_then(|v| v.as_str());

        let result = self
            .client
            .find(locator, value, action, action_value)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "result": result,
            "message": format!("Find {}='{}' with action '{}'", locator, value, action)
        }))
    }

    /// Handle the scroll action
    async fn handle_scroll(&self, args: &Value) -> Result<Value, AgentError> {
        let direction = args
            .get("direction")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'direction' argument for scroll action".to_string())
            })?;

        let pixels = args
            .get("pixels")
            .and_then(|v| v.as_u64())
            .map(|p| p as u32);

        self.client
            .scroll(direction, pixels)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Scrolled {}", direction)
        }))
    }

    /// Handle the is action
    async fn handle_is(&self, args: &Value) -> Result<Value, AgentError> {
        let what = args.get("what").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'what' argument for is action".to_string())
        })?;

        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for is action".to_string())
            })?;

        let result = self
            .client
            .is_(what, selector)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "result": result,
            "message": format!("Element {} is {}: {}", selector, what, result)
        }))
    }

    // ========================================
    // Download handlers
    // ========================================

    /// Handle the download action
    async fn handle_download(&self, args: &Value) -> Result<Value, AgentError> {
        let selector = args
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'selector' argument for download action".to_string())
            })?;

        let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'path' argument for download action".to_string())
        })?;

        let download_path = self
            .client
            .download(selector, path)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "path": download_path,
            "message": format!("Downloaded to: {}", download_path)
        }))
    }

    // ========================================
    // Tab handlers
    // ========================================

    /// Handle the tab_list action
    async fn handle_tab_list(&self) -> Result<Value, AgentError> {
        let tabs = self
            .client
            .tab_list()
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        let tabs_json: Vec<Value> = tabs
            .iter()
            .map(|t| {
                serde_json::json!({
                    "index": t.index,
                    "title": t.title
                })
            })
            .collect();

        Ok(serde_json::json!({
            "success": true,
            "tabs": tabs_json,
            "count": tabs.len()
        }))
    }

    /// Handle the tab_new action
    async fn handle_tab_new(&self, args: &Value) -> Result<Value, AgentError> {
        let url = args.get("url").and_then(|v| v.as_str());

        self.client
            .tab_new(url)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Opened new tab"
        }))
    }

    /// Handle the tab_close action
    async fn handle_tab_close(&self, args: &Value) -> Result<Value, AgentError> {
        let index = args
            .get("index")
            .and_then(|v| v.as_u64())
            .map(|i| i as usize);

        self.client
            .tab_close(index)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Tab closed"
        }))
    }

    /// Handle the tab_select action
    async fn handle_tab_select(&self, args: &Value) -> Result<Value, AgentError> {
        let index = args.get("index").and_then(|v| v.as_u64()).ok_or_else(|| {
            AgentError::ToolError("Missing 'index' argument for tab_select action".to_string())
        })? as usize;

        self.client
            .tab_select(index)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Switched to tab {}", index)
        }))
    }

    // ========================================
    // Dialog handlers
    // ========================================

    /// Handle the dialog_accept action
    async fn handle_dialog_accept(&self, args: &Value) -> Result<Value, AgentError> {
        let text = args.get("text").and_then(|v| v.as_str());

        self.client
            .dialog_accept(text)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Dialog accepted"
        }))
    }

    /// Handle the dialog_dismiss action
    async fn handle_dialog_dismiss(&self) -> Result<Value, AgentError> {
        self.client
            .dialog_dismiss()
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Dialog dismissed"
        }))
    }

    // ========================================
    // Storage & Network handlers
    // ========================================

    /// Handle the cookies action
    async fn handle_cookies(&self) -> Result<Value, AgentError> {
        let cookies = self
            .client
            .cookies()
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        let cookies_json: Vec<Value> = cookies
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "value": c.value
                })
            })
            .collect();

        Ok(serde_json::json!({
            "success": true,
            "cookies": cookies_json,
            "count": cookies.len()
        }))
    }

    /// Handle the cookies_set action
    async fn handle_cookies_set(&self, args: &Value) -> Result<Value, AgentError> {
        let name = args.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'name' argument for cookies_set action".to_string())
        })?;

        let value = args.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'value' argument for cookies_set action".to_string())
        })?;

        self.client
            .cookies_set(name, value)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Set cookie: {}={}", name, value)
        }))
    }

    /// Handle the storage_get action
    async fn handle_storage_get(&self, args: &Value) -> Result<Value, AgentError> {
        let storage_type = args
            .get("storage_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError(
                    "Missing 'storage_type' argument for storage_get action".to_string(),
                )
            })?;

        let key = args.get("key").and_then(|v| v.as_str());

        let value = self
            .client
            .storage_get(storage_type, key)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "value": value
        }))
    }

    /// Handle the storage_set action
    async fn handle_storage_set(&self, args: &Value) -> Result<Value, AgentError> {
        let storage_type = args
            .get("storage_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::ToolError(
                    "Missing 'storage_type' argument for storage_set action".to_string(),
                )
            })?;

        let key = args.get("key").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'key' argument for storage_set action".to_string())
        })?;

        let value = args.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'value' argument for storage_set action".to_string())
        })?;

        self.client
            .storage_set(storage_type, key, value)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Set {} storage: {}={}", storage_type, key, value)
        }))
    }

    /// Handle the network_requests action
    async fn handle_network_requests(&self, args: &Value) -> Result<Value, AgentError> {
        let filter = args.get("filter").and_then(|v| v.as_str());

        let requests = self
            .client
            .network_requests(filter)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        let requests_json: Vec<Value> = requests
            .iter()
            .map(|r| {
                serde_json::json!({
                    "method": r.method,
                    "url": r.url
                })
            })
            .collect();

        Ok(serde_json::json!({
            "success": true,
            "requests": requests_json,
            "count": requests.len()
        }))
    }

    // ========================================
    // Settings handlers
    // ========================================

    /// Handle the set_viewport action
    async fn handle_set_viewport(&self, args: &Value) -> Result<Value, AgentError> {
        let width = args.get("width").and_then(|v| v.as_u64()).ok_or_else(|| {
            AgentError::ToolError("Missing 'width' argument for set_viewport action".to_string())
        })? as u32;

        let height = args.get("height").and_then(|v| v.as_u64()).ok_or_else(|| {
            AgentError::ToolError("Missing 'height' argument for set_viewport action".to_string())
        })? as u32;

        let scale = args.get("scale").and_then(|v| v.as_f64()).map(|s| s as f32);

        self.client
            .set_viewport(width, height, scale)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Set viewport to {}x{}", width, height)
        }))
    }

    /// Handle the set_device action
    async fn handle_set_device(&self, args: &Value) -> Result<Value, AgentError> {
        let name = args.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError("Missing 'name' argument for set_device action".to_string())
        })?;

        self.client
            .set_device(name)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Set device to: {}", name)
        }))
    }

    /// Handle the set_geo action
    async fn handle_set_geo(&self, args: &Value) -> Result<Value, AgentError> {
        let latitude = args
            .get("latitude")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'latitude' argument for set_geo action".to_string())
            })?;

        let longitude = args
            .get("longitude")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                AgentError::ToolError("Missing 'longitude' argument for set_geo action".to_string())
            })?;

        self.client
            .set_geo(latitude, longitude)
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Set geolocation to ({}, {})", latitude, longitude)
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
            "wait_for_text" => self.handle_wait_for_text(&args).await,
            "wait_for_url" => self.handle_wait_for_url(&args).await,
            "wait_for_load" => self.handle_wait_for_load(&args).await,
            "wait_for_download" => self.handle_wait_for_download(&args).await,
            "wait_for_fn" => self.handle_wait_for_fn(&args).await,
            "wait_for_state" => self.handle_wait_for_state(&args).await,
            "eval" => self.handle_eval(&args).await,
            "get" => self.handle_get(&args).await,
            "get_attr" => self.handle_get_attr(&args).await,
            "get_count" => self.handle_get_count(&args).await,
            "get_box" => self.handle_get_box(&args).await,
            "get_styles" => self.handle_get_styles(&args).await,
            // Navigation
            "back" => self.handle_back().await,
            "forward" => self.handle_forward().await,
            "reload" => self.handle_reload().await,
            // Input
            "type" => self.handle_type(&args).await,
            "press" => self.handle_press(&args).await,
            "hover" => self.handle_hover(&args).await,
            "select" => self.handle_select(&args).await,
            "dblclick" => self.handle_dblclick(&args).await,
            "focus" => self.handle_focus(&args).await,
            "check" => self.handle_check(&args).await,
            "uncheck" => self.handle_uncheck(&args).await,
            "scrollintoview" => self.handle_scrollintoview(&args).await,
            "drag" => self.handle_drag(&args).await,
            "upload" => self.handle_upload(&args).await,
            "pdf" => self.handle_pdf(&args).await,
            // State
            "find" => self.handle_find(&args).await,
            "scroll" => self.handle_scroll(&args).await,
            "is" => self.handle_is(&args).await,
            // Downloads
            "download" => self.handle_download(&args).await,
            // Tabs
            "tab_list" => self.handle_tab_list().await,
            "tab_new" => self.handle_tab_new(&args).await,
            "tab_close" => self.handle_tab_close(&args).await,
            "tab_select" => self.handle_tab_select(&args).await,
            // Dialogs
            "dialog_accept" => self.handle_dialog_accept(&args).await,
            "dialog_dismiss" => self.handle_dialog_dismiss().await,
            // Storage & Network
            "cookies" => self.handle_cookies().await,
            "cookies_set" => self.handle_cookies_set(&args).await,
            "storage_get" => self.handle_storage_get(&args).await,
            "storage_set" => self.handle_storage_set(&args).await,
            "network_requests" => self.handle_network_requests(&args).await,
            // Settings
            "set_viewport" => self.handle_set_viewport(&args).await,
            "set_device" => self.handle_set_device(&args).await,
            "set_geo" => self.handle_set_geo(&args).await,
            _ => Err(AgentError::ToolError(format!(
                "Unknown browser action: {}. Valid actions: open, close, snapshot, click, fill, screenshot, wait, wait_for_text, wait_for_url, wait_for_load, wait_for_download, wait_for_fn, wait_for_state, eval, get, get_attr, get_count, get_box, get_styles, back, forward, reload, type, press, hover, select, dblclick, focus, check, uncheck, scrollintoview, drag, upload, pdf, find, scroll, is, download, tab_list, tab_new, tab_close, tab_select, dialog_accept, dialog_dismiss, cookies, cookies_set, storage_get, storage_set, network_requests, set_viewport, set_device, set_geo",
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
