//! Browser client - shared API for browser automation
//!
//! Provides a high-level API for browser operations used by both
//! the TUI command and the agent tool.

use super::config::BrowserConfig;
use super::executor::{BrowserError, BrowserExecutor};
use std::sync::Arc;

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

    /// Get reference to executor (for extension traits)
    pub fn executor(&self) -> &Arc<dyn BrowserExecutor> {
        &self.executor
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
    pub(crate) fn extract_field(content: &str, field_name: &str) -> Option<String> {
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
