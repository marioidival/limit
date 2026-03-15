//! Waiting operations

use crate::tools::browser::executor::BrowserError;

/// Waiting operations for browser client
pub trait WaitingExt {
    /// Wait for a condition (text, element, or timeout)
    fn wait_for(
        &self,
        condition: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Wait for text to appear on page
    fn wait_for_text(
        &self,
        text: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Wait for URL pattern match
    fn wait_for_url(
        &self,
        pattern: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Wait for page load state (networkidle, domcontentloaded, load)
    fn wait_for_load(
        &self,
        state: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Wait for download to complete, returns download path
    fn wait_for_download(
        &self,
        path: Option<&str>,
    ) -> impl std::future::Future<Output = Result<String, BrowserError>> + Send;

    /// Wait for JavaScript function to return truthy value
    fn wait_for_fn(
        &self,
        js: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Wait for element state (visible, hidden, attached, detached, enabled, disabled)
    fn wait_for_state(
        &self,
        selector: &str,
        state: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;
}

impl WaitingExt for super::BrowserClient {
    async fn wait_for(&self, condition: &str) -> Result<(), BrowserError> {
        if condition.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Condition cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["wait", condition]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Wait condition not met: {}",
                output.stderr
            )))
        }
    }

    async fn wait_for_text(&self, text: &str) -> Result<(), BrowserError> {
        if text.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Text cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["wait", "--text", text]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Text not found: {}",
                output.stderr
            )))
        }
    }

    async fn wait_for_url(&self, pattern: &str) -> Result<(), BrowserError> {
        if pattern.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "URL pattern cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["wait", "--url", pattern]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "URL pattern not matched: {}",
                output.stderr
            )))
        }
    }

    async fn wait_for_load(&self, state: &str) -> Result<(), BrowserError> {
        let valid_states = ["networkidle", "domcontentloaded", "load"];
        if !valid_states.contains(&state) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid load state '{}'. Valid states: {}",
                state,
                valid_states.join(", ")
            )));
        }

        let output = self.executor().execute(&["wait", "--load", state]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Load state not reached: {}",
                output.stderr
            )))
        }
    }

    async fn wait_for_download(&self, path: Option<&str>) -> Result<String, BrowserError> {
        let output = if let Some(p) = path {
            self.executor().execute(&["wait", "--download", p]).await?
        } else {
            self.executor().execute(&["wait", "--download"]).await?
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

    async fn wait_for_fn(&self, js: &str) -> Result<(), BrowserError> {
        if js.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "JavaScript function cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["wait", "--fn", js]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "JavaScript condition not met: {}",
                output.stderr
            )))
        }
    }

    async fn wait_for_state(&self, selector: &str, state: &str) -> Result<(), BrowserError> {
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
            .executor()
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
}
