//! Clipboard operations for the TUI

use arboard::Clipboard;
use std::sync::Mutex;

/// Thread-safe clipboard wrapper
pub struct ClipboardManager {
    clipboard: Mutex<Clipboard>,
}

impl ClipboardManager {
    /// Create a new clipboard manager
    pub fn new() -> Result<Self, arboard::Error> {
        let clipboard = Clipboard::new()?;
        Ok(Self {
            clipboard: Mutex::new(clipboard),
        })
    }

    /// Copy text to clipboard
    pub fn set_text(&self, text: &str) -> Result<(), arboard::Error> {
        let mut clipboard = self
            .clipboard
            .lock()
            .map_err(|_| arboard::Error::ClipboardNotSupported)?;
        clipboard.set_text(text.to_string())?;
        Ok(())
    }

    /// Get text from clipboard
    pub fn get_text(&self) -> Result<String, arboard::Error> {
        let mut clipboard = self
            .clipboard
            .lock()
            .map_err(|_| arboard::Error::ClipboardNotSupported)?;
        clipboard.get_text()
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize clipboard")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_manager_new() {
        // This test may fail in CI without clipboard access
        if let Ok(manager) = ClipboardManager::new() {
            assert!(manager.set_text("test").is_ok());
            assert_eq!(manager.get_text().unwrap(), "test");
        }
    }
}
