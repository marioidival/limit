//! Input history management for TUI

use std::fs;
use std::path::PathBuf;

/// Maximum number of entries to keep in history
const MAX_HISTORY_SIZE: usize = 500;

/// Input history with navigation support
#[derive(Debug, Clone)]
pub struct InputHistory {
    /// History entries (index 0 = most recent)
    entries: Vec<String>,
    /// Maximum number of entries to keep
    max_size: usize,
    /// Current navigation position (None = new input, Some(0) = most recent)
    current_index: Option<usize>,
    /// Saved draft before navigation started
    saved_draft: Option<String>,
}

impl InputHistory {
    /// Create a new empty history
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(100),
            max_size: MAX_HISTORY_SIZE,
            current_index: None,
            saved_draft: None,
        }
    }

    /// Create a new history with custom max size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            entries: Vec::with_capacity(100),
            max_size,
            current_index: None,
            saved_draft: None,
        }
    }

    /// Add a new entry to history
    ///
    /// - Ignores empty strings
    /// - Ignores duplicates of the most recent entry
    /// - Truncates to max_size if needed
    pub fn add(&mut self, text: &str) {
        let text = text.trim();

        // Skip empty entries
        if text.is_empty() {
            return;
        }

        // Skip if same as most recent
        if self.entries.first().map(|s| s.as_str()) == Some(text) {
            return;
        }

        // Add to front
        self.entries.insert(0, text.to_string());

        // Truncate if needed
        if self.entries.len() > self.max_size {
            self.entries.truncate(self.max_size);
        }

        // Reset navigation
        self.current_index = None;
        self.saved_draft = None;
    }

    /// Navigate to previous (older) entry
    ///
    /// Returns the entry at the new position, or None if at the oldest
    /// Saves current draft before navigation starts
    pub fn navigate_up(&mut self, current_draft: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        // Save draft before first navigation
        if self.current_index.is_none() {
            self.saved_draft = if current_draft.is_empty() {
                None
            } else {
                Some(current_draft.to_string())
            };
        }

        // Move to older entry
        match self.current_index {
            None => {
                self.current_index = Some(0);
                self.entries.first().map(|s| s.as_str())
            }
            Some(idx) if idx + 1 < self.entries.len() => {
                self.current_index = Some(idx + 1);
                self.entries.get(idx + 1).map(|s| s.as_str())
            }
            Some(_) => {
                // Already at oldest, return current
                self.current()
            }
        }
    }

    /// Navigate to next (newer) entry
    ///
    /// Returns the entry at the new position, or restores draft if at newest
    pub fn navigate_down(&mut self) -> Option<&str> {
        match self.current_index {
            None => {
                // Already at newest/new input, nothing to do
                None
            }
            Some(0) => {
                // At newest, restore draft or clear
                self.current_index = None;
                None // Signal to restore draft
            }
            Some(idx) => {
                self.current_index = Some(idx - 1);
                self.current()
            }
        }
    }

    /// Get current entry (if navigating)
    pub fn current(&self) -> Option<&str> {
        self.current_index
            .and_then(|idx| self.entries.get(idx).map(|s| s.as_str()))
    }

    /// Check if currently navigating history
    pub fn is_navigating(&self) -> bool {
        self.current_index.is_some()
    }

    /// Get saved draft (text before navigation started)
    pub fn saved_draft(&self) -> Option<&str> {
        self.saved_draft.as_deref()
    }

    /// Reset navigation state
    pub fn reset_navigation(&mut self) {
        self.current_index = None;
        self.saved_draft = None;
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries (for debugging)
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Load history from file
    pub fn load(path: &PathBuf) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let data =
            fs::read_to_string(path).map_err(|e| format!("Failed to read history file: {}", e))?;

        let entries: Vec<String> = serde_json::from_str(&data)
            .map_err(|e| format!("Failed to deserialize history: {}", e))?;

        Ok(Self {
            entries,
            max_size: MAX_HISTORY_SIZE,
            current_index: None,
            saved_draft: None,
        })
    }

    /// Save history to file
    pub fn save(&self, path: &PathBuf) -> Result<(), String> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create history directory: {}", e))?;
        }

        let serialized = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| format!("Failed to serialize history: {}", e))?;

        fs::write(path, serialized).map_err(|e| format!("Failed to write history file: {}", e))?;

        Ok(())
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_index = None;
        self.saved_draft = None;
    }
}

impl Default for InputHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_history_is_empty() {
        let history = InputHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        assert!(!history.is_navigating());
    }

    #[test]
    fn test_add_entry() {
        let mut history = InputHistory::new();
        history.add("hello");

        assert_eq!(history.len(), 1);
        assert_eq!(history.entries()[0], "hello");
    }

    #[test]
    fn test_add_multiple_entries() {
        let mut history = InputHistory::new();
        history.add("first");
        history.add("second");
        history.add("third");

        assert_eq!(history.len(), 3);
        // Most recent first
        assert_eq!(history.entries()[0], "third");
        assert_eq!(history.entries()[1], "second");
        assert_eq!(history.entries()[2], "first");
    }

    #[test]
    fn test_add_empty_ignored() {
        let mut history = InputHistory::new();
        history.add("");
        history.add("   ");

        assert!(history.is_empty());
    }

    #[test]
    fn test_add_duplicate_most_recent_ignored() {
        let mut history = InputHistory::new();
        history.add("hello");
        history.add("hello");

        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_add_duplicate_older_allowed() {
        let mut history = InputHistory::new();
        history.add("hello");
        history.add("world");
        history.add("hello"); // Re-add hello - it moves to front

        assert_eq!(history.len(), 3);
        assert_eq!(history.entries()[0], "hello"); // most recent (re-added)
        assert_eq!(history.entries()[1], "world"); // middle
        assert_eq!(history.entries()[2], "hello"); // oldest (original)
    }

    #[test]
    fn test_max_size_truncation() {
        let mut history = InputHistory::with_max_size(3);
        history.add("first");
        history.add("second");
        history.add("third");
        history.add("fourth");

        assert_eq!(history.len(), 3);
        assert_eq!(history.entries()[0], "fourth");
        assert_eq!(history.entries()[2], "second");
    }

    #[test]
    fn test_navigate_up_empty_history() {
        let mut history = InputHistory::new();
        let result = history.navigate_up("draft");

        assert!(result.is_none());
        assert!(!history.is_navigating());
    }

    #[test]
    fn test_navigate_up_saves_draft() {
        let mut history = InputHistory::new();
        history.add("entry");

        history.navigate_up("my draft");

        assert!(history.is_navigating());
        assert_eq!(history.saved_draft(), Some("my draft"));
    }

    #[test]
    fn test_navigate_up_empty_draft_not_saved() {
        let mut history = InputHistory::new();
        history.add("entry");

        history.navigate_up("");

        assert!(history.is_navigating());
        assert!(history.saved_draft().is_none());
    }

    #[test]
    fn test_navigate_up_multiple() {
        let mut history = InputHistory::new();
        history.add("oldest");
        history.add("middle");
        history.add("newest");

        let first = history.navigate_up("");
        assert_eq!(first, Some("newest"));
        assert_eq!(history.current(), Some("newest"));

        let second = history.navigate_up("");
        assert_eq!(second, Some("middle"));

        let third = history.navigate_up("");
        assert_eq!(third, Some("oldest"));

        // At oldest, another up stays at oldest
        let fourth = history.navigate_up("");
        assert_eq!(fourth, Some("oldest"));
    }

    #[test]
    fn test_navigate_down_from_oldest() {
        let mut history = InputHistory::new();
        history.add("oldest");
        history.add("newest");

        // Navigate to oldest
        history.navigate_up("");
        history.navigate_up("");
        assert_eq!(history.current(), Some("oldest"));

        // Navigate down to newest
        let result = history.navigate_down();
        assert_eq!(result, Some("newest"));

        // Navigate down again - should signal restore draft
        let result = history.navigate_down();
        assert!(result.is_none());
        assert!(!history.is_navigating());
    }

    #[test]
    fn test_navigate_down_without_navigation() {
        let mut history = InputHistory::new();
        history.add("entry");

        // Navigate down without up - should do nothing
        let result = history.navigate_down();
        assert!(result.is_none());
        assert!(!history.is_navigating());
    }

    #[test]
    fn test_reset_navigation() {
        let mut history = InputHistory::new();
        history.add("entry");

        history.navigate_up("draft");
        assert!(history.is_navigating());

        history.reset_navigation();
        assert!(!history.is_navigating());
        assert!(history.saved_draft().is_none());
    }

    #[test]
    fn test_add_resets_navigation() {
        let mut history = InputHistory::new();
        history.add("first");

        history.navigate_up("draft");
        assert!(history.is_navigating());

        history.add("second");
        assert!(!history.is_navigating());
        assert!(history.saved_draft().is_none());
    }

    #[test]
    fn test_roundtrip_save_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("history.json");

        let mut history = InputHistory::new();
        history.add("first");
        history.add("second");
        history.add("third");

        history.save(&path).unwrap();

        let loaded = InputHistory::load(&path).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded.entries()[0], "third");
        assert_eq!(loaded.entries()[1], "second");
        assert_eq!(loaded.entries()[2], "first");
    }

    #[test]
    fn test_load_missing_file_returns_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");

        let history = InputHistory::load(&path).unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_save_creates_parent_directory() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("history.json");

        let mut history = InputHistory::new();
        history.add("test");

        history.save(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_clear() {
        let mut history = InputHistory::new();
        history.add("entry");
        history.navigate_up("draft");

        history.clear();

        assert!(history.is_empty());
        assert!(!history.is_navigating());
        assert!(history.saved_draft().is_none());
    }

    #[test]
    fn test_trim_on_add() {
        let mut history = InputHistory::new();
        history.add("  hello world  ");

        assert_eq!(history.entries()[0], "hello world");
    }

    #[test]
    fn test_navigate_up_down_cycle() {
        let mut history = InputHistory::new();
        history.add("oldest");
        history.add("middle");
        history.add("newest");

        // Save draft
        history.navigate_up("my draft");

        // Go to oldest
        history.navigate_up("");
        history.navigate_up("");
        assert_eq!(history.current(), Some("oldest"));

        // Come back to newest
        history.navigate_down();
        assert_eq!(history.current(), Some("middle"));

        history.navigate_down();
        assert_eq!(history.current(), Some("newest"));

        // Final down should return None (signal to restore draft)
        let result = history.navigate_down();
        assert!(result.is_none());
        assert_eq!(history.saved_draft(), Some("my draft"));
    }
}
