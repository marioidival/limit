//! Browser tool for LLM agent
//!
//! Implements the Tool trait for browser automation in the agent system.

use super::action::BrowserAction;
use super::client::BrowserClient;
use super::client_ext::{InteractionExt, NavigationExt, QueryExt, StorageExt, TabsExt, WaitingExt};
use super::config::{BrowserConfig, BrowserEngine};
use super::executor::{BrowserExecutor, CliExecutor};
use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use serde_json::Value;
use std::str::FromStr;
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

        self.client.open(url).await?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Opened {}", url)
        }))
    }

    /// Handle the close action
    async fn handle_close(&self) -> Result<Value, AgentError> {
        self.client.close().await?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Browser closed"
        }))
    }

    /// Handle the snapshot action
    async fn handle_snapshot(&self) -> Result<Value, AgentError> {
        let result = self.client.snapshot().await?;

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

        self.client.click(selector).await?;

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

        self.client.fill(selector, text).await?;

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

        self.client.screenshot(path).await?;

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

        self.client.wait_for(condition).await?;

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

        self.client.wait_for_text(text).await?;

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

        self.client.wait_for_url(pattern).await?;

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

        self.client.wait_for_load(state).await?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Load state reached: {}", state)
        }))
    }

    /// Handle the wait_for_download action
    async fn handle_wait_for_download(&self, args: &Value) -> Result<Value, AgentError> {
        let path = args.get("path").and_then(|v| v.as_str());

        let download_path = self.client.wait_for_download(path).await?;

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

        self.client.wait_for_fn(js).await?;

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

        self.client.wait_for_state(selector, state).await?;

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

        let result = self.client.eval(script).await?;

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

        let content = self.client.get(what).await?;

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

        let value = self.client.get_attr(selector, attr).await?;

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

        let count = self.client.get_count(selector).await?;

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

        let bbox = self.client.get_box(selector).await?;

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

        let styles = self.client.get_styles(selector).await?;

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
        self.client.back().await?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Navigated back"
        }))
    }

    /// Handle the forward action
    async fn handle_forward(&self) -> Result<Value, AgentError> {
        self.client.forward().await?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Navigated forward"
        }))
    }

    /// Handle the reload action
    async fn handle_reload(&self) -> Result<Value, AgentError> {
        self.client.reload().await?;

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

        self.client.type_text(selector, text).await?;

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

        self.client.press(key).await?;

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

        self.client.hover(selector).await?;

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

        self.client.select_option(selector, value).await?;

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

        self.client.dblclick(selector).await?;

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

        self.client.focus(selector).await?;

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

        self.client.check(selector).await?;

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

        self.client.uncheck(selector).await?;

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

        self.client.scrollintoview(selector).await?;

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

        self.client.drag(source, target).await?;

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

        self.client.upload(selector, &file_paths).await?;

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

        self.client.pdf(path).await?;

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
            .await?;

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

        self.client.scroll(direction, pixels).await?;

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

        let result = self.client.is_(what, selector).await?;

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

        let download_path = self.client.download(selector, path).await?;

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
        let tabs = self.client.tab_list().await?;

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

        self.client.tab_new(url).await?;

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

        self.client.tab_close(index).await?;

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

        self.client.tab_select(index).await?;

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

        self.client.dialog_accept(text).await?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Dialog accepted"
        }))
    }

    /// Handle the dialog_dismiss action
    async fn handle_dialog_dismiss(&self) -> Result<Value, AgentError> {
        self.client.dialog_dismiss().await?;

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
        let cookies = self.client.cookies().await?;

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

        self.client.cookies_set(name, value).await?;

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

        let value = self.client.storage_get(storage_type, key).await?;

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

        self.client.storage_set(storage_type, key, value).await?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Set {} storage: {}={}", storage_type, key, value)
        }))
    }

    /// Handle the network_requests action
    async fn handle_network_requests(&self, args: &Value) -> Result<Value, AgentError> {
        let filter = args.get("filter").and_then(|v| v.as_str());

        let requests = self.client.network_requests(filter).await?;

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

        self.client.set_viewport(width, height, scale).await?;

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

        self.client.set_device(name).await?;

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

        self.client.set_geo(latitude, longitude).await?;

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
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::ToolError("Missing 'action' argument".to_string()))?;

        match BrowserAction::from_str(action)? {
            BrowserAction::Open => self.handle_open(&args).await,
            BrowserAction::Close => self.handle_close().await,
            BrowserAction::Snapshot => self.handle_snapshot().await,
            BrowserAction::Screenshot => self.handle_screenshot(&args).await,
            BrowserAction::Back => self.handle_back().await,
            BrowserAction::Forward => self.handle_forward().await,
            BrowserAction::Reload => self.handle_reload().await,
            BrowserAction::Click => self.handle_click(&args).await,
            BrowserAction::Fill => self.handle_fill(&args).await,
            BrowserAction::Type => self.handle_type(&args).await,
            BrowserAction::Press => self.handle_press(&args).await,
            BrowserAction::Hover => self.handle_hover(&args).await,
            BrowserAction::Select => self.handle_select(&args).await,
            BrowserAction::Dblclick => self.handle_dblclick(&args).await,
            BrowserAction::Focus => self.handle_focus(&args).await,
            BrowserAction::Check => self.handle_check(&args).await,
            BrowserAction::Uncheck => self.handle_uncheck(&args).await,
            BrowserAction::Scrollintoview => self.handle_scrollintoview(&args).await,
            BrowserAction::Drag => self.handle_drag(&args).await,
            BrowserAction::Upload => self.handle_upload(&args).await,
            BrowserAction::Pdf => self.handle_pdf(&args).await,
            BrowserAction::Get => self.handle_get(&args).await,
            BrowserAction::GetAttr => self.handle_get_attr(&args).await,
            BrowserAction::GetCount => self.handle_get_count(&args).await,
            BrowserAction::GetBox => self.handle_get_box(&args).await,
            BrowserAction::GetStyles => self.handle_get_styles(&args).await,
            BrowserAction::Wait => self.handle_wait(&args).await,
            BrowserAction::WaitForText => self.handle_wait_for_text(&args).await,
            BrowserAction::WaitForUrl => self.handle_wait_for_url(&args).await,
            BrowserAction::WaitForLoad => self.handle_wait_for_load(&args).await,
            BrowserAction::WaitForDownload => self.handle_wait_for_download(&args).await,
            BrowserAction::WaitForFn => self.handle_wait_for_fn(&args).await,
            BrowserAction::WaitForState => self.handle_wait_for_state(&args).await,
            BrowserAction::Find => self.handle_find(&args).await,
            BrowserAction::Scroll => self.handle_scroll(&args).await,
            BrowserAction::Is => self.handle_is(&args).await,
            BrowserAction::Download => self.handle_download(&args).await,
            BrowserAction::TabList => self.handle_tab_list().await,
            BrowserAction::TabNew => self.handle_tab_new(&args).await,
            BrowserAction::TabClose => self.handle_tab_close(&args).await,
            BrowserAction::TabSelect => self.handle_tab_select(&args).await,
            BrowserAction::DialogAccept => self.handle_dialog_accept(&args).await,
            BrowserAction::DialogDismiss => self.handle_dialog_dismiss().await,
            BrowserAction::Cookies => self.handle_cookies().await,
            BrowserAction::CookiesSet => self.handle_cookies_set(&args).await,
            BrowserAction::StorageGet => self.handle_storage_get(&args).await,
            BrowserAction::StorageSet => self.handle_storage_set(&args).await,
            BrowserAction::NetworkRequests => self.handle_network_requests(&args).await,
            BrowserAction::SetViewport => self.handle_set_viewport(&args).await,
            BrowserAction::SetDevice => self.handle_set_device(&args).await,
            BrowserAction::SetGeo => self.handle_set_geo(&args).await,
            BrowserAction::Eval => self.handle_eval(&args).await,
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
