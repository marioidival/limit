//! Tab operations

use crate::tools::browser::executor::BrowserError;
use crate::tools::browser::types::TabInfo;

/// Tab operations for browser client
pub trait TabsExt {
    /// List all browser tabs
    fn tab_list(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<TabInfo>, BrowserError>> + Send;

    /// Open a new tab, optionally with URL
    fn tab_new(
        &self,
        url: Option<&str>,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Close a tab by index (or current if None)
    fn tab_close(
        &self,
        index: Option<usize>,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Select/switch to a tab by index
    fn tab_select(
        &self,
        index: usize,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Accept a dialog (alert, confirm, prompt) with optional text
    fn dialog_accept(
        &self,
        text: Option<&str>,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Dismiss a dialog (alert, confirm, prompt)
    fn dialog_dismiss(&self) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;
}

impl TabsExt for super::BrowserClient {
    async fn tab_list(&self) -> Result<Vec<TabInfo>, BrowserError> {
        let output = self.executor().execute(&["tab", "list"]).await?;

        if output.success {
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

    async fn tab_new(&self, url: Option<&str>) -> Result<(), BrowserError> {
        let output = if let Some(u) = url {
            self.executor().execute(&["tab", "new", u]).await?
        } else {
            self.executor().execute(&["tab", "new"]).await?
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

    async fn tab_close(&self, index: Option<usize>) -> Result<(), BrowserError> {
        let output = if let Some(i) = index {
            let idx = i.to_string();
            self.executor().execute(&["tab", "close", &idx]).await?
        } else {
            self.executor().execute(&["tab", "close"]).await?
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

    async fn tab_select(&self, index: usize) -> Result<(), BrowserError> {
        let idx = index.to_string();
        let output = self.executor().execute(&["tab", "select", &idx]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to select tab: {}",
                output.stderr
            )))
        }
    }

    async fn dialog_accept(&self, text: Option<&str>) -> Result<(), BrowserError> {
        let output = if let Some(t) = text {
            self.executor().execute(&["dialog", "accept", t]).await?
        } else {
            self.executor().execute(&["dialog", "accept"]).await?
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

    async fn dialog_dismiss(&self) -> Result<(), BrowserError> {
        let output = self.executor().execute(&["dialog", "dismiss"]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to dismiss dialog: {}",
                output.stderr
            )))
        }
    }
}
