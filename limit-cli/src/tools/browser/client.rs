//! Browser client - shared API for browser automation
//!
//! Provides a high-level API for browser operations used by both
//! the TUI command and the agent tool.

use super::config::BrowserConfig;
use super::executor::{BrowserError, BrowserExecutor};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Result from snapshot operation
#[derive(Debug, Clone)]
pub struct SnapshotResult {
    /// The accessibility tree as markdown
    pub content: String,
    /// Page title if available
    pub title: Option<String>,
    /// Current URL
    pub url: Option<String>,
}

/// Information about a browser tab
#[derive(Debug, Clone)]
pub struct TabInfo {
    /// Tab index
    pub index: usize,
    /// Tab title
    pub title: String,
}

/// Element bounding box
#[derive(Debug, Clone)]
pub struct BoundingBox {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Width
    pub width: f64,
    /// Height
    pub height: f64,
}

/// Cookie information
#[derive(Debug, Clone)]
pub struct Cookie {
    /// Cookie name
    pub name: String,
    /// Cookie value
    pub value: String,
}

/// Network request information
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP method
    pub method: String,
    /// Request URL
    pub url: String,
}

/// Browser client for high-level operations
pub struct BrowserClient {
    executor: Arc<dyn BrowserExecutor>,
}

impl BrowserClient {
    /// Create a new browser client with the given executor
    pub fn new(executor: Arc<dyn BrowserExecutor>) -> Self {
        Self { executor }
    }

    /// Create a client with default configuration
    pub fn with_default_config() -> Self {
        let config = BrowserConfig::default();
        let executor = Arc::new(super::executor::CliExecutor::new(config));
        Self::new(executor)
    }

    /// Open a URL in the browser
    pub async fn open(&self, url: &str) -> Result<(), BrowserError> {
        self.validate_url(url)?;

        let output = self.executor.execute(&["open", url]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to open URL: {}",
                output.stderr
            )))
        }
    }

    /// Close the browser
    pub async fn close(&self) -> Result<(), BrowserError> {
        let output = self.executor.execute(&["close"]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to close browser: {}",
                output.stderr
            )))
        }
    }

    /// Take an accessibility snapshot of the current page
    pub async fn snapshot(&self) -> Result<SnapshotResult, BrowserError> {
        let output = self.executor.execute(&["snapshot"]).await?;

        if output.success {
            // Parse the snapshot output
            let content = output.stdout.trim().to_string();

            // Try to extract title and URL from the output
            let title = Self::extract_field(&content, "Title:");
            let url = Self::extract_field(&content, "URL:");

            Ok(SnapshotResult {
                content,
                title,
                url,
            })
        } else {
            Err(BrowserError::Other(format!(
                "Failed to take snapshot: {}",
                output.stderr
            )))
        }
    }

    /// Click an element by selector or ref
    pub async fn click(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["click", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to click element: {}",
                output.stderr
            )))
        }
    }

    /// Fill a form field with text
    pub async fn fill(&self, selector: &str, text: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["fill", selector, text]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to fill element: {}",
                output.stderr
            )))
        }
    }

    /// Take a screenshot and save to path
    pub async fn screenshot(&self, path: &str) -> Result<(), BrowserError> {
        if path.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Path cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["screenshot", path]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to take screenshot: {}",
                output.stderr
            )))
        }
    }

    /// Wait for a condition (text, element, or timeout)
    pub async fn wait_for(&self, condition: &str) -> Result<(), BrowserError> {
        if condition.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Condition cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["wait", condition]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Wait condition not met: {}",
                output.stderr
            )))
        }
    }

    /// Wait for text to appear on page
    pub async fn wait_for_text(&self, text: &str) -> Result<(), BrowserError> {
        if text.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Text cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["wait", "--text", text]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Text not found: {}",
                output.stderr
            )))
        }
    }

    /// Wait for URL pattern match
    pub async fn wait_for_url(&self, pattern: &str) -> Result<(), BrowserError> {
        if pattern.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "URL pattern cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["wait", "--url", pattern]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "URL pattern not matched: {}",
                output.stderr
            )))
        }
    }

    /// Wait for page load state (networkidle, domcontentloaded, load)
    pub async fn wait_for_load(&self, state: &str) -> Result<(), BrowserError> {
        let valid_states = ["networkidle", "domcontentloaded", "load"];
        if !valid_states.contains(&state) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid load state '{}'. Valid states: {}",
                state,
                valid_states.join(", ")
            )));
        }

        let output = self.executor.execute(&["wait", "--load", state]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Load state not reached: {}",
                output.stderr
            )))
        }
    }

    /// Wait for download to complete, returns download path
    pub async fn wait_for_download(&self, path: Option<&str>) -> Result<String, BrowserError> {
        let output = if let Some(p) = path {
            self.executor.execute(&["wait", "--download", p]).await?
        } else {
            self.executor.execute(&["wait", "--download"]).await?
        };

        if output.success {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(BrowserError::Other(format!(
                "Download wait failed: {}",
                output.stderr
            )))
        }
    }

    /// Wait for JavaScript function to return truthy value
    pub async fn wait_for_fn(&self, js: &str) -> Result<(), BrowserError> {
        if js.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "JavaScript function cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["wait", "--fn", js]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "JavaScript condition not met: {}",
                output.stderr
            )))
        }
    }

    /// Wait for element state (visible, hidden, attached, detached, enabled, disabled)
    pub async fn wait_for_state(&self, selector: &str, state: &str) -> Result<(), BrowserError> {
        let valid_states = [
            "visible", "hidden", "attached", "detached", "enabled", "disabled",
        ];
        if !valid_states.contains(&state) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid state '{}'. Valid states: {}",
                state,
                valid_states.join(", ")
            )));
        }

        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self
            .executor
            .execute(&["wait", "--state", state, selector])
            .await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Element state not reached: {}",
                output.stderr
            )))
        }
    }

    /// Evaluate JavaScript in the browser
    pub async fn eval(&self, script: &str) -> Result<JsonValue, BrowserError> {
        if script.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Script cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["eval", script]).await?;

        if output.success {
            // Try to parse the output as JSON
            let trimmed = output.stdout.trim();
            if trimmed.is_empty() {
                Ok(JsonValue::Null)
            } else {
                serde_json::from_str(trimmed)
                    .map_err(|e| BrowserError::ParseError(format!("Invalid JSON: {}", e)))
            }
        } else {
            Err(BrowserError::Other(format!(
                "Failed to evaluate script: {}",
                output.stderr
            )))
        }
    }

    /// Get page content (text, html, value, url, or title)
    pub async fn get(&self, what: &str) -> Result<String, BrowserError> {
        let valid_types = ["text", "html", "value", "url", "title"];
        if !valid_types.contains(&what) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid get type '{}'. Valid types: {}",
                what,
                valid_types.join(", ")
            )));
        }

        let output = self.executor.execute(&["get", what]).await?;

        if output.success {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get {}: {}",
                what, output.stderr
            )))
        }
    }

    /// Get element attribute value
    pub async fn get_attr(&self, selector: &str, attr: &str) -> Result<String, BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        if attr.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Attribute name cannot be empty".to_string(),
            ));
        }

        let output = self
            .executor
            .execute(&["get", "attr", selector, attr])
            .await?;

        if output.success {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get attribute: {}",
                output.stderr
            )))
        }
    }

    /// Get count of elements matching selector
    pub async fn get_count(&self, selector: &str) -> Result<usize, BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["get", "count", selector]).await?;

        if output.success {
            let count = output
                .stdout
                .trim()
                .parse::<usize>()
                .map_err(|_| BrowserError::ParseError("Invalid count value".to_string()))?;
            Ok(count)
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get count: {}",
                output.stderr
            )))
        }
    }

    /// Get element bounding box
    pub async fn get_box(&self, selector: &str) -> Result<BoundingBox, BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["get", "box", selector]).await?;

        if output.success {
            // Parse bounding box from output (format: x,y,width,height)
            let parts: Vec<&str> = output.stdout.trim().split(',').collect();
            if parts.len() == 4 {
                Ok(BoundingBox {
                    x: parts[0]
                        .parse()
                        .map_err(|_| BrowserError::ParseError("Invalid x value".to_string()))?,
                    y: parts[1]
                        .parse()
                        .map_err(|_| BrowserError::ParseError("Invalid y value".to_string()))?,
                    width: parts[2]
                        .parse()
                        .map_err(|_| BrowserError::ParseError("Invalid width".to_string()))?,
                    height: parts[3]
                        .parse()
                        .map_err(|_| BrowserError::ParseError("Invalid height".to_string()))?,
                })
            } else {
                Err(BrowserError::ParseError(
                    "Invalid bounding box format".to_string(),
                ))
            }
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get bounding box: {}",
                output.stderr
            )))
        }
    }

    /// Get element computed styles
    pub async fn get_styles(
        &self,
        selector: &str,
    ) -> Result<std::collections::HashMap<String, String>, BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["get", "styles", selector]).await?;

        if output.success {
            // Parse styles from output (format: key:value per line)
            let mut styles = std::collections::HashMap::new();
            for line in output.stdout.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    styles.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
            Ok(styles)
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get styles: {}",
                output.stderr
            )))
        }
    }

    // ========================================
    // Navigation methods
    // ========================================

    /// Navigate back in browser history
    pub async fn back(&self) -> Result<(), BrowserError> {
        let output = self.executor.execute(&["back"]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to navigate back: {}",
                output.stderr
            )))
        }
    }

    /// Navigate forward in browser history
    pub async fn forward(&self) -> Result<(), BrowserError> {
        let output = self.executor.execute(&["forward"]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to navigate forward: {}",
                output.stderr
            )))
        }
    }

    /// Reload the current page
    pub async fn reload(&self) -> Result<(), BrowserError> {
        let output = self.executor.execute(&["reload"]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to reload page: {}",
                output.stderr
            )))
        }
    }

    // ========================================
    // Input methods
    // ========================================

    /// Type text into an element (character by character)
    pub async fn type_text(&self, selector: &str, text: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["type", selector, text]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to type text: {}",
                output.stderr
            )))
        }
    }

    /// Press a keyboard key
    pub async fn press(&self, key: &str) -> Result<(), BrowserError> {
        if key.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Key cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["press", key]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to press key: {}",
                output.stderr
            )))
        }
    }

    /// Hover over an element
    pub async fn hover(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["hover", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to hover: {}",
                output.stderr
            )))
        }
    }

    /// Select an option in a dropdown
    pub async fn select_option(&self, selector: &str, value: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["select", selector, value]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to select option: {}",
                output.stderr
            )))
        }
    }

    /// Double-click an element
    pub async fn dblclick(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["dblclick", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to double-click element: {}",
                output.stderr
            )))
        }
    }

    /// Focus an element
    pub async fn focus(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["focus", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to focus element: {}",
                output.stderr
            )))
        }
    }

    /// Check a checkbox
    pub async fn check(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["check", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to check element: {}",
                output.stderr
            )))
        }
    }

    /// Uncheck a checkbox
    pub async fn uncheck(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["uncheck", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to uncheck element: {}",
                output.stderr
            )))
        }
    }

    /// Scroll an element into view
    pub async fn scrollintoview(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["scrollintoview", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to scroll element into view: {}",
                output.stderr
            )))
        }
    }

    /// Drag and drop from source to destination
    pub async fn drag(&self, source: &str, target: &str) -> Result<(), BrowserError> {
        if source.is_empty() || target.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Source and target selectors cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["drag", source, target]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to drag and drop: {}",
                output.stderr
            )))
        }
    }

    /// Upload files to a file input
    pub async fn upload(&self, selector: &str, files: &[&str]) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        if files.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "At least one file must be specified".to_string(),
            ));
        }

        let mut args = vec!["upload", selector];
        let files_owned: Vec<String> = files.iter().map(|f| f.to_string()).collect();
        let file_refs: Vec<&str> = files_owned.iter().map(|f| f.as_str()).collect();
        args.extend(file_refs);

        let output = self.executor.execute(&args).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to upload files: {}",
                output.stderr
            )))
        }
    }

    /// Save page as PDF
    pub async fn pdf(&self, path: &str) -> Result<(), BrowserError> {
        if path.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Path cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["pdf", path]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to save PDF: {}",
                output.stderr
            )))
        }
    }

    // ========================================
    // State methods
    // ========================================

    /// Find elements using various locator strategies
    ///
    /// Locator types: role, text, label, placeholder, alt, title, testid, css, xpath
    /// Actions: click, fill, text, count, first, last, nth
    pub async fn find(
        &self,
        locator_type: &str,
        value: &str,
        action: &str,
        action_value: Option<&str>,
    ) -> Result<String, BrowserError> {
        let valid_locators = [
            "role",
            "text",
            "label",
            "placeholder",
            "alt",
            "title",
            "testid",
            "css",
            "xpath",
        ];
        if !valid_locators.contains(&locator_type) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid locator type '{}'. Valid types: {}",
                locator_type,
                valid_locators.join(", ")
            )));
        }

        if value.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Locator value cannot be empty".to_string(),
            ));
        }

        let valid_actions = [
            "click", "fill", "text", "count", "first", "last", "nth", "hover", "focus", "check",
            "uncheck",
        ];
        if !valid_actions.contains(&action) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid action '{}'. Valid actions: {}",
                action,
                valid_actions.join(", ")
            )));
        }

        // Build command: find --<locator> <value> <action> [action_value]
        let locator_flag = format!("--{}", locator_type);
        let mut args = vec!["find", &locator_flag, value, action];

        let output = if let Some(av) = action_value {
            args.push(av);
            self.executor.execute(&args).await?
        } else {
            self.executor.execute(&args).await?
        };

        if output.success {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(BrowserError::Other(format!(
                "Find action failed: {}",
                output.stderr
            )))
        }
    }

    /// Scroll the page (up, down, left, right) with optional pixel amount
    pub async fn scroll(&self, direction: &str, pixels: Option<u32>) -> Result<(), BrowserError> {
        let valid_directions = ["up", "down", "left", "right"];
        if !valid_directions.contains(&direction) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid scroll direction '{}'. Valid directions: {}",
                direction,
                valid_directions.join(", ")
            )));
        }

        let output = match pixels {
            Some(px) => {
                let px_str = px.to_string();
                self.executor
                    .execute(&["scroll", direction, &px_str])
                    .await?
            }
            None => self.executor.execute(&["scroll", direction]).await?,
        };

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to scroll: {}",
                output.stderr
            )))
        }
    }

    /// Check element state (visible, hidden, enabled, disabled, editable)
    pub async fn is_(&self, what: &str, selector: &str) -> Result<bool, BrowserError> {
        let valid_states = ["visible", "hidden", "enabled", "disabled", "editable"];
        if !valid_states.contains(&what) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid state check '{}'. Valid states: {}",
                what,
                valid_states.join(", ")
            )));
        }

        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["is", what, selector]).await?;

        if output.success {
            // Parse boolean from output
            let result = output.stdout.trim().to_lowercase();
            Ok(result == "true" || result == "yes" || result == "1")
        } else {
            Err(BrowserError::Other(format!(
                "Failed to check state: {}",
                output.stderr
            )))
        }
    }

    // ========================================
    // Download methods
    // ========================================

    /// Download file from link/button to path
    pub async fn download(&self, selector: &str, path: &str) -> Result<String, BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["download", selector, path]).await?;

        if output.success {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to download: {}",
                output.stderr
            )))
        }
    }

    // ========================================
    // Tab methods
    // ========================================

    /// List all browser tabs
    pub async fn tab_list(&self) -> Result<Vec<TabInfo>, BrowserError> {
        let output = self.executor.execute(&["tab", "list"]).await?;

        if output.success {
            // Parse tab list from output
            let tabs = output
                .stdout
                .lines()
                .filter(|line| !line.is_empty())
                .enumerate()
                .map(|(i, line)| TabInfo {
                    index: i,
                    title: line.to_string(),
                })
                .collect();
            Ok(tabs)
        } else {
            Err(BrowserError::Other(format!(
                "Failed to list tabs: {}",
                output.stderr
            )))
        }
    }

    /// Open a new tab, optionally with URL
    pub async fn tab_new(&self, url: Option<&str>) -> Result<(), BrowserError> {
        let output = if let Some(u) = url {
            self.executor.execute(&["tab", "new", u]).await?
        } else {
            self.executor.execute(&["tab", "new"]).await?
        };

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to open new tab: {}",
                output.stderr
            )))
        }
    }

    /// Close a tab by index (or current if None)
    pub async fn tab_close(&self, index: Option<usize>) -> Result<(), BrowserError> {
        let output = if let Some(i) = index {
            let idx = i.to_string();
            self.executor.execute(&["tab", "close", &idx]).await?
        } else {
            self.executor.execute(&["tab", "close"]).await?
        };

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to close tab: {}",
                output.stderr
            )))
        }
    }

    /// Select/switch to a tab by index
    pub async fn tab_select(&self, index: usize) -> Result<(), BrowserError> {
        let idx = index.to_string();
        let output = self.executor.execute(&["tab", "select", &idx]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to select tab: {}",
                output.stderr
            )))
        }
    }

    // ========================================
    // Dialog methods
    // ========================================

    /// Accept a dialog (alert, confirm, prompt) with optional text
    pub async fn dialog_accept(&self, text: Option<&str>) -> Result<(), BrowserError> {
        let output = if let Some(t) = text {
            self.executor.execute(&["dialog", "accept", t]).await?
        } else {
            self.executor.execute(&["dialog", "accept"]).await?
        };

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to accept dialog: {}",
                output.stderr
            )))
        }
    }

    /// Dismiss a dialog (alert, confirm, prompt)
    pub async fn dialog_dismiss(&self) -> Result<(), BrowserError> {
        let output = self.executor.execute(&["dialog", "dismiss"]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to dismiss dialog: {}",
                output.stderr
            )))
        }
    }

    // ========================================
    // Storage & Network methods
    // ========================================

    /// Get all cookies
    pub async fn cookies(&self) -> Result<Vec<Cookie>, BrowserError> {
        let output = self.executor.execute(&["cookies"]).await?;

        if output.success {
            // Parse cookies from output (format: name=value per line)
            let cookies = output
                .stdout
                .lines()
                .filter(|line| !line.is_empty())
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        Some(Cookie {
                            name: parts[0].to_string(),
                            value: parts[1].to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            Ok(cookies)
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get cookies: {}",
                output.stderr
            )))
        }
    }

    /// Set a cookie
    pub async fn cookies_set(&self, name: &str, value: &str) -> Result<(), BrowserError> {
        if name.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Cookie name cannot be empty".to_string(),
            ));
        }

        let output = self
            .executor
            .execute(&["cookies", "set", name, value])
            .await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to set cookie: {}",
                output.stderr
            )))
        }
    }

    /// Get storage value (local or session)
    pub async fn storage_get(
        &self,
        storage_type: &str,
        key: Option<&str>,
    ) -> Result<JsonValue, BrowserError> {
        let valid_types = ["local", "session"];
        if !valid_types.contains(&storage_type) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid storage type '{}'. Valid types: {}",
                storage_type,
                valid_types.join(", ")
            )));
        }

        let output = if let Some(k) = key {
            self.executor
                .execute(&["storage", storage_type, "get", k])
                .await?
        } else {
            self.executor
                .execute(&["storage", storage_type, "get"])
                .await?
        };

        if output.success {
            let trimmed = output.stdout.trim();
            if trimmed.is_empty() {
                Ok(JsonValue::Null)
            } else {
                Ok(serde_json::from_str(trimmed)
                    .unwrap_or_else(|_| JsonValue::String(trimmed.to_string())))
            }
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get storage: {}",
                output.stderr
            )))
        }
    }

    /// Set storage value (local or session)
    pub async fn storage_set(
        &self,
        storage_type: &str,
        key: &str,
        value: &str,
    ) -> Result<(), BrowserError> {
        let valid_types = ["local", "session"];
        if !valid_types.contains(&storage_type) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid storage type '{}'. Valid types: {}",
                storage_type,
                valid_types.join(", ")
            )));
        }

        if key.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Key cannot be empty".to_string(),
            ));
        }

        let output = self
            .executor
            .execute(&["storage", storage_type, "set", key, value])
            .await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to set storage: {}",
                output.stderr
            )))
        }
    }

    /// Get network requests
    pub async fn network_requests(
        &self,
        filter: Option<&str>,
    ) -> Result<Vec<Request>, BrowserError> {
        let output = if let Some(f) = filter {
            self.executor.execute(&["network", "requests", f]).await?
        } else {
            self.executor.execute(&["network", "requests"]).await?
        };

        if output.success {
            // Parse requests from output (format: METHOD URL per line)
            let requests = output
                .stdout
                .lines()
                .filter(|line| !line.is_empty())
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        Some(Request {
                            method: parts[0].to_string(),
                            url: parts[1].to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            Ok(requests)
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get network requests: {}",
                output.stderr
            )))
        }
    }

    // ========================================
    // Settings methods
    // ========================================

    /// Set viewport size
    pub async fn set_viewport(
        &self,
        width: u32,
        height: u32,
        scale: Option<f32>,
    ) -> Result<(), BrowserError> {
        let w = width.to_string();
        let h = height.to_string();

        let output = if let Some(s) = scale {
            let s_str = s.to_string();
            self.executor
                .execute(&["set", "viewport", &w, &h, &s_str])
                .await?
        } else {
            self.executor.execute(&["set", "viewport", &w, &h]).await?
        };

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to set viewport: {}",
                output.stderr
            )))
        }
    }

    /// Set device emulation (e.g., "iPhone 12", "Pixel 5")
    pub async fn set_device(&self, name: &str) -> Result<(), BrowserError> {
        if name.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Device name cannot be empty".to_string(),
            ));
        }

        let output = self.executor.execute(&["set", "device", name]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to set device: {}",
                output.stderr
            )))
        }
    }

    /// Set geolocation
    pub async fn set_geo(&self, latitude: f64, longitude: f64) -> Result<(), BrowserError> {
        let lat = latitude.to_string();
        let lng = longitude.to_string();

        let output = self.executor.execute(&["set", "geo", &lat, &lng]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to set geolocation: {}",
                output.stderr
            )))
        }
    }

    /// Check if browser daemon is running
    pub fn is_daemon_running(&self) -> bool {
        self.executor.is_daemon_running()
    }

    /// Validate a URL
    fn validate_url(&self, url: &str) -> Result<(), BrowserError> {
        if url.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "URL cannot be empty".to_string(),
            ));
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(BrowserError::InvalidArguments(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        Ok(())
    }

    /// Extract a field value from snapshot content
    fn extract_field(content: &str, field_name: &str) -> Option<String> {
        for line in content.lines() {
            if let Some(stripped) = line.strip_prefix(field_name) {
                return Some(stripped.trim().to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_result_creation() {
        let result = SnapshotResult {
            content: "test content".to_string(),
            title: Some("Test Page".to_string()),
            url: Some("https://example.com".to_string()),
        };

        assert_eq!(result.content, "test content");
        assert_eq!(result.title, Some("Test Page".to_string()));
    }

    #[test]
    fn test_extract_field() {
        let content = "Title: Example\nURL: https://example.com\nOther content";
        assert_eq!(
            BrowserClient::extract_field(content, "Title:"),
            Some("Example".to_string())
        );
        assert_eq!(
            BrowserClient::extract_field(content, "URL:"),
            Some("https://example.com".to_string())
        );
        assert_eq!(BrowserClient::extract_field(content, "Missing:"), None);
    }
}
