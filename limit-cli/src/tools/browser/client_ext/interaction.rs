//! Interaction operations

use crate::tools::browser::executor::BrowserError;

/// Interaction operations for browser client
pub trait InteractionExt {
    /// Click an element by selector
    fn click(
        &self,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Double-click an element
    fn dblclick(
        &self,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Fill a form field with text
    fn fill(
        &self,
        selector: &str,
        text: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Type text into an element (character by character)
    fn type_text(
        &self,
        selector: &str,
        text: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Press a keyboard key
    fn press(
        &self,
        key: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Hover over an element
    fn hover(
        &self,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Select an option in a dropdown
    fn select_option(
        &self,
        selector: &str,
        value: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Focus an element
    fn focus(
        &self,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Check a checkbox
    fn check(
        &self,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Uncheck a checkbox
    fn uncheck(
        &self,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Scroll an element into view
    fn scrollintoview(
        &self,
        selector: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Drag and drop from source to destination
    fn drag(
        &self,
        source: &str,
        target: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Upload files to a file input
    fn upload(
        &self,
        selector: &str,
        files: &[&str],
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Scroll the page
    fn scroll(
        &self,
        direction: &str,
        pixels: Option<u32>,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;
}

impl InteractionExt for super::BrowserClient {
    async fn click(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["click", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to click element: {}",
                output.stderr
            )))
        }
    }

    async fn dblclick(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["dblclick", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to double-click element: {}",
                output.stderr
            )))
        }
    }

    async fn fill(&self, selector: &str, text: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["fill", selector, text]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to fill element: {}",
                output.stderr
            )))
        }
    }

    async fn type_text(&self, selector: &str, text: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["type", selector, text]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to type text: {}",
                output.stderr
            )))
        }
    }

    async fn press(&self, key: &str) -> Result<(), BrowserError> {
        if key.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Key cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["press", key]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to press key: {}",
                output.stderr
            )))
        }
    }

    async fn hover(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["hover", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to hover: {}",
                output.stderr
            )))
        }
    }

    async fn select_option(&self, selector: &str, value: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["select", selector, value]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to select option: {}",
                output.stderr
            )))
        }
    }

    async fn focus(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["focus", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to focus element: {}",
                output.stderr
            )))
        }
    }

    async fn check(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["check", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to check element: {}",
                output.stderr
            )))
        }
    }

    async fn uncheck(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["uncheck", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to uncheck element: {}",
                output.stderr
            )))
        }
    }

    async fn scrollintoview(&self, selector: &str) -> Result<(), BrowserError> {
        if selector.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Selector cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["scrollintoview", selector]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to scroll element into view: {}",
                output.stderr
            )))
        }
    }

    async fn drag(&self, source: &str, target: &str) -> Result<(), BrowserError> {
        if source.is_empty() || target.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Source and target selectors cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["drag", source, target]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to drag and drop: {}",
                output.stderr
            )))
        }
    }

    async fn upload(&self, selector: &str, files: &[&str]) -> Result<(), BrowserError> {
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

        let output = self.executor().execute(&args).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to upload files: {}",
                output.stderr
            )))
        }
    }

    async fn scroll(&self, direction: &str, pixels: Option<u32>) -> Result<(), BrowserError> {
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
                self.executor()
                    .execute(&["scroll", direction, &px_str])
                    .await?
            }
            None => self.executor().execute(&["scroll", direction]).await?,
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
}
