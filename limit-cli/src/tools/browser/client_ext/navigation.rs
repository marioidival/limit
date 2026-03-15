//! Navigation operations

use crate::tools::browser::executor::BrowserError;

/// Navigation operations for browser client
pub trait NavigationExt {
    /// Navigate back in browser history
    fn back(&self) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;
    /// Navigate forward in browser history
    fn forward(&self) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;
    /// Reload the current page
    fn reload(&self) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;
}

impl NavigationExt for super::super::BrowserClient {
    async fn back(&self) -> Result<(), BrowserError> {
        let output = self.executor().execute(&["back"]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to navigate back: {}",
                output.stderr
            )))
        }
    }

    async fn forward(&self) -> Result<(), BrowserError> {
        let output = self.executor().execute(&["forward"]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to navigate forward: {}",
                output.stderr
            )))
        }
    }

    async fn reload(&self) -> Result<(), BrowserError> {
        let output = self.executor().execute(&["reload"]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to reload page: {}",
                output.stderr
            )))
        }
    }
}
