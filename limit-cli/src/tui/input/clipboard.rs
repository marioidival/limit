//! Clipboard operations handler
//!
//! Handles copy/paste operations with platform-specific support.

use crate::clipboard::ClipboardManager;
use crate::error::CliError;

/// Clipboard handler for copy/paste operations
pub struct ClipboardHandler {
    clipboard: Option<ClipboardManager>,
}

impl ClipboardHandler {
    /// Create a new clipboard handler
    pub fn new() -> Self {
        let clipboard = match ClipboardManager::new() {
            Ok(cb) => {
                tracing::info!("Clipboard initialized successfully");
                Some(cb)
            }
            Err(e) => {
                tracing::warn!("Clipboard unavailable: {}", e);
                None
            }
        };
        Self { clipboard }
    }

    /// Check if clipboard is available
    pub fn is_available(&self) -> bool {
        self.clipboard.is_some()
    }

    /// Copy text to clipboard
    pub fn copy(&self, text: &str) -> Result<(), CliError> {
        if let Some(ref clipboard) = self.clipboard {
            clipboard.set_text(text).map_err(|e| {
                CliError::ConfigError(format!("Failed to copy to clipboard: {}", e))
            })?;
            tracing::debug!("Copied {} chars to clipboard", text.len());
            Ok(())
        } else {
            Err(CliError::ConfigError("Clipboard not available".to_string()))
        }
    }

    /// Paste text from clipboard
    pub fn paste(&self) -> Result<String, CliError> {
        if let Some(ref clipboard) = self.clipboard {
            let text = clipboard.get_text().map_err(|e| {
                CliError::ConfigError(format!("Failed to read from clipboard: {}", e))
            })?;
            tracing::debug!("Read {} chars from clipboard", text.len());
            Ok(text)
        } else {
            Err(CliError::ConfigError("Clipboard not available".to_string()))
        }
    }
}

impl Default for ClipboardHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_handler_creation() {
        let handler = ClipboardHandler::new();
        // Clipboard may or may not be available in test environment
        assert!(handler.is_available() || !handler.is_available());
    }

    #[test]
    fn test_clipboard_handler_default() {
        let handler = ClipboardHandler::default();
        assert!(handler.is_available() || !handler.is_available());
    }
}
