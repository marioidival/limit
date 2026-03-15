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

    // ========================================
    // State methods
    // ========================================

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
