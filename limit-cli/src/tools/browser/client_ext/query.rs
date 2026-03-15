//! Query and snapshot operations

use crate::tools::browser::executor::BrowserError;
use crate::tools::browser::types::{BoundingBox, SnapshotResult};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Query and snapshot operations for browser client
pub trait QueryExt {
    /// Take an accessibility snapshot of the current page
    fn snapshot(
        &self,
    ) -> impl std::future::Future<Output = Result<SnapshotResult, BrowserError>> + Send;

    /// Take a screenshot and save to path
    fn screenshot(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Save page as PDF
    fn pdf(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Evaluate JavaScript in the browser
    fn eval(
        &self,
        script: &str,
    ) -> impl std::future::Future<Output = Result<JsonValue, BrowserError>> + Send;

    /// Get page content (text, html, value, url, or title)
    fn get(
        &self,
        what: &str,
    ) -> impl std::future::Future<Output = Result<String, BrowserError>> + Send;

    /// Get element attribute value
    fn get_attr(
        &self,
        selector: &str,
        attr: &str,
    ) -> impl std::future::Future<Output = Result<String, BrowserError>> + Send;

    /// Get count of elements matching selector
    fn get_count(
        &self,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<usize, BrowserError>> + Send;

    /// Get element bounding box
    fn get_box(
        &self,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<BoundingBox, BrowserError>> + Send;

    /// Get element computed styles
    fn get_styles(
        &self,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<HashMap<String, String>, BrowserError>> + Send;

    /// Find elements using various locator strategies
    fn find(
        &self,
        locator_type: &str,
        value: &str,
        action: &str,
        action_value: Option<&str>,
    ) -> impl std::future::Future<Output = Result<String, BrowserError>> + Send;

    /// Check element state (visible, hidden, enabled, disabled, editable)
    fn is_(
        &self,
        what: &str,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<bool, BrowserError>> + Send;

    /// Download file from link/button to path
    fn download(
        &self,
        selector: &str,
        path: &str,
    ) -> impl std::future::Future<Output = Result<String, BrowserError>> + Send;
}

impl QueryExt for super::super::BrowserClient {
    async fn snapshot(&self) -> Result<SnapshotResult, BrowserError> {
        let output = self.executor().execute(&["snapshot"]).await?;

        if output.success {
            let content = output.stdout.trim().to_string();
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

    async fn screenshot(&self, path: &str) -> Result<(), BrowserError> {
        if path.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Path cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["screenshot", path]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to take screenshot: {}",
                output.stderr
            )))
        }
    }

    async fn pdf(&self, path: &str) -> Result<(), BrowserError> {
        if path.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Path cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["pdf", path]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to save PDF: {}",
                output.stderr
            )))
        }
    }

    async fn eval(&self, script: &str) -> Result<JsonValue, BrowserError> {
        if script.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Script cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["eval", script]).await?;

        if output.success {
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

    async fn get(&self, what: &str) -> Result<String, BrowserError> {
        let valid_types = ["text", "html", "value", "url", "title"];
        if !valid_types.contains(&what) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid get type '{}'. Valid types: {}",
                what,
                valid_types.join(", ")
            )));
        }

        let output = self.executor().execute(&["get", what]).await?;

        if output.success {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get {}: {}",
                what, output.stderr
            )))
        }
    }

    async fn get_attr(&self, selector: &str, attr: &str) -> Result<String, BrowserError> {
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
            .executor()
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

    async fn get_count(&self, selector: &str) -> Result<usize, BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["get", "count", selector]).await?;

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

    async fn get_box(&self, selector: &str) -> Result<BoundingBox, BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["get", "box", selector]).await?;

        if output.success {
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
                        .map_err(|_| BrowserError::ParseError("Invalid width value".to_string()))?,
                    height: parts[3]
                        .parse()
                        .map_err(|_| BrowserError::ParseError("Invalid height value".to_string()))?,
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

    async fn get_styles(&self, selector: &str) -> Result<HashMap<String, String>, BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["get", "styles", selector]).await?;

        if output.success {
            let mut styles = HashMap::new();
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

    async fn find(
        &self,
        locator_type: &str,
        value: &str,
        action: &str,
        action_value: Option<&str>,
    ) -> Result<String, BrowserError> {
        let valid_locators = [
            "role", "text", "label", "placeholder", "alt", "title", "testid", "css", "xpath",
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

        let locator_flag = format!("--{}", locator_type);
        let mut args = vec!["find", &locator_flag, value, action];

        let output = if let Some(av) = action_value {
            args.push(av);
            self.executor().execute(&args).await?
        } else {
            self.executor().execute(&args).await?
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

    async fn is_(&self, what: &str, selector: &str) -> Result<bool, BrowserError> {
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

        let output = self.executor().execute(&["is", what, selector]).await?;

        if output.success {
            let result = output.stdout.trim().to_lowercase();
            Ok(result == "true" || result == "yes" || result == "1")
        } else {
            Err(BrowserError::Other(format!(
                "Failed to check state: {}",
                output.stderr
            )))
        }
    }

    async fn download(&self, selector: &str, path: &str) -> Result<String, BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["download", selector, path]).await?;

        if output.success {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to download: {}",
                output.stderr
            )))
        }
    }
}
