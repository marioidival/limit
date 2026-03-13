//! Input text editor for TUI
//!
//! Manages input text buffer with cursor position, editing operations, and history navigation.

use crate::tui::input::history::InputHistory;
use crate::tui::MAX_PASTE_SIZE;
use std::path::PathBuf;

/// Input text editor with cursor management and history
pub struct InputEditor {
    /// Text buffer
    text: String,
    /// Cursor position (byte offset)
    cursor: usize,
    /// Input history
    history: InputHistory,
}

impl InputEditor {
    /// Create a new empty editor
    pub fn new() -> Self {
        Self {
            text: String::with_capacity(256),
            cursor: 0,
            history: InputHistory::new(),
        }
    }

    /// Create editor with history loaded from file
    pub fn with_history(history_path: &PathBuf) -> Result<Self, String> {
        let history = InputHistory::load(history_path)?;
        Ok(Self {
            text: String::with_capacity(256),
            cursor: 0,
            history,
        })
    }

    /// Get the current text
    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get a mutable reference to the text
    #[inline]
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// Get cursor position
    #[inline]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Set cursor position (clamped to valid range)
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.text.len());
        // Ensure cursor is at char boundary
        while self.cursor > 0 && !self.text.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }
    }

    /// Check if text is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Check if cursor is at start
    #[inline]
    pub fn is_cursor_at_start(&self) -> bool {
        self.cursor == 0
    }

    /// Check if cursor is at end
    #[inline]
    pub fn is_cursor_at_end(&self) -> bool {
        self.cursor == self.text.len()
    }

    /// Insert a character at cursor position
    #[inline]
    pub fn insert_char(&mut self, c: char) {
        // Reset history navigation when user types
        if self.history.is_navigating() {
            self.history.reset_navigation();
        }

        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Insert a string at cursor position
    #[inline]
    pub fn insert_str(&mut self, s: &str) {
        // Reset history navigation when user types
        if self.history.is_navigating() {
            self.history.reset_navigation();
        }

        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    /// Insert paste with size limit
    /// Returns true if truncated
    pub fn insert_paste(&mut self, text: &str) -> bool {
        let (text, truncated) = truncate_paste(text);

        // Normalize newlines with pre-allocated capacity
        let normalized = if text.contains('\r') {
            let mut normalized = String::with_capacity(text.len());
            for c in text.chars() {
                normalized.push(if c == '\r' { '\n' } else { c });
            }
            normalized
        } else {
            return {
                self.insert_str(text);
                truncated
            };
        };

        self.insert_str(&normalized);
        truncated
    }

    /// Delete character before cursor (backspace)
    pub fn delete_char_before(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }

        let prev_pos = self.prev_char_pos();
        self.text.drain(prev_pos..self.cursor);
        self.cursor = prev_pos;
        true
    }

    /// Delete character at cursor (delete key)
    pub fn delete_char_at(&mut self) -> bool {
        if self.cursor >= self.text.len() {
            return false;
        }

        let next_pos = self.next_char_pos();
        self.text.drain(self.cursor..next_pos);
        true
    }

    /// Move cursor left one character
    #[inline]
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.prev_char_pos();
        }
    }

    /// Move cursor right one character
    #[inline]
    pub fn move_right(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor = self.next_char_pos();
        }
    }

    /// Move cursor to start
    #[inline]
    pub fn move_to_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    #[inline]
    pub fn move_to_end(&mut self) {
        self.cursor = self.text.len();
    }

    /// Clear all text
    #[inline]
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    /// Get trimmed text and clear
    pub fn take_trimmed(&mut self) -> String {
        let trimmed = self.text.trim();
        let result = String::from(trimmed);
        self.clear();
        result
    }

    /// Replace text in a range (used for autocomplete)
    pub fn replace_range(&mut self, start: usize, end: usize, replacement: &str) {
        self.text.drain(start..end);
        self.text.insert_str(start, replacement);
        self.cursor = start + replacement.len();
    }

    /// Delete from start to cursor
    #[inline]
    pub fn delete_range_to_cursor(&mut self, start: usize) {
        if start < self.cursor {
            self.text.drain(start..self.cursor);
            self.cursor = start;
        }
    }

    /// Get text before cursor
    #[inline]
    pub fn text_before_cursor(&self) -> &str {
        &self.text[..self.cursor]
    }

    /// Get text after cursor
    #[inline]
    pub fn text_after_cursor(&self) -> &str {
        &self.text[self.cursor..]
    }

    /// Get character at cursor (if any)
    #[inline]
    pub fn char_at_cursor(&self) -> Option<char> {
        self.text[self.cursor..].chars().next()
    }

    /// Get character before cursor (if any)
    pub fn char_before_cursor(&self) -> Option<char> {
        if self.cursor == 0 {
            return None;
        }
        let prev_pos = self.prev_char_pos();
        self.text[prev_pos..self.cursor].chars().next()
    }

    /// Navigate to previous (older) history entry
    /// Returns true if navigation happened
    pub fn navigate_history_up(&mut self) -> bool {
        let current = self.text.trim();

        if let Some(entry) = self.history.navigate_up(current) {
            self.text = entry.to_string();
            self.cursor = self.text.len();
            return true;
        }

        false
    }

    /// Navigate to next (newer) history entry
    /// Returns true if navigation happened (false if at newest)
    pub fn navigate_history_down(&mut self) -> bool {
        if let Some(entry) = self.history.navigate_down() {
            self.text = entry.to_string();
            self.cursor = self.text.len();
            return true;
        } else if !self.history.is_navigating() {
            // Restoring draft - handled by saved_draft()
            if let Some(draft) = self.history.saved_draft() {
                self.text = draft.to_string();
                self.cursor = self.text.len();
            } else {
                self.clear();
            }
            return true;
        }

        false
    }

    /// Check if currently navigating history
    pub fn is_navigating_history(&self) -> bool {
        self.history.is_navigating()
    }

    /// Add current text to history and clear
    pub fn take_and_add_to_history(&mut self) -> String {
        let text = self.text.trim().to_string();

        if !text.is_empty() {
            self.history.add(&text);
        }

        self.clear();
        text
    }

    /// Save history to file
    pub fn save_history(&self, path: &PathBuf) -> Result<(), String> {
        self.history.save(path)
    }

    /// Get reference to history (for debugging)
    pub fn history(&self) -> &InputHistory {
        &self.history
    }

    /// Get mutable reference to history (for testing)
    pub fn history_mut(&mut self) -> &mut InputHistory {
        &mut self.history
    }

    /// Find previous char boundary
    #[inline]
    fn prev_char_pos(&self) -> usize {
        if self.cursor == 0 {
            return 0;
        }
        let mut pos = self.cursor - 1;
        while pos > 0 && !self.text.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    }

    /// Find next char boundary
    #[inline]
    fn next_char_pos(&self) -> usize {
        if self.cursor >= self.text.len() {
            return self.text.len();
        }
        let mut pos = self.cursor + 1;
        while pos < self.text.len() && !self.text.is_char_boundary(pos) {
            pos += 1;
        }
        pos
    }
}

/// Truncate paste to max size (freestanding function for reuse)
#[inline]
fn truncate_paste(text: &str) -> (&str, bool) {
    if text.len() <= MAX_PASTE_SIZE {
        return (text, false);
    }

    let truncated = &text[..text
        .char_indices()
        .nth(MAX_PASTE_SIZE)
        .map(|(i, _)| i)
        .unwrap_or(text.len())];
    (truncated, true)
}

impl Default for InputEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_creation() {
        let editor = InputEditor::new();
        assert!(editor.is_empty());
        assert_eq!(editor.cursor(), 0);
    }

    #[test]
    fn test_insert_char() {
        let mut editor = InputEditor::new();
        editor.insert_char('h');
        editor.insert_char('i');
        assert_eq!(editor.text(), "hi");
        assert_eq!(editor.cursor(), 2);
    }

    #[test]
    fn test_delete_char_before() {
        let mut editor = InputEditor::new();
        editor.insert_str("hello");
        editor.set_cursor(3);

        assert!(editor.delete_char_before());
        assert_eq!(editor.text(), "helo");
        assert_eq!(editor.cursor(), 2);
    }

    #[test]
    fn test_navigation() {
        let mut editor = InputEditor::new();
        editor.insert_str("hello");

        editor.move_left();
        assert_eq!(editor.cursor(), 4);

        editor.move_to_start();
        assert_eq!(editor.cursor(), 0);

        editor.move_to_end();
        assert_eq!(editor.cursor(), 5);
    }

    #[test]
    fn test_utf8() {
        let mut editor = InputEditor::new();
        editor.insert_str("héllo");

        let pos = editor.text().char_indices().nth(2).map(|(i, _)| i).unwrap();
        editor.set_cursor(pos);

        assert_eq!(editor.cursor(), pos);
        assert_eq!(editor.char_before_cursor(), Some('é'));
    }

    #[test]
    fn test_take_trimmed() {
        let mut editor = InputEditor::new();
        editor.insert_str("  hello  ");
        let text = editor.take_trimmed();
        assert_eq!(text, "hello");
        assert!(editor.is_empty());
    }

    #[test]
    fn test_replace_range() {
        let mut editor = InputEditor::new();
        editor.insert_str("hello world");
        editor.replace_range(6, 11, "universe");
        assert_eq!(editor.text(), "hello universe");
    }

    #[test]
    fn test_utf8_emojis() {
        let mut editor = InputEditor::new();

        editor.insert_str("Hello 👋 World 🌍");
        assert_eq!(editor.text(), "Hello 👋 World 🌍");

        editor.move_to_start();
        editor.move_right();
        editor.move_right();

        editor.insert_char('🚀');
        assert_eq!(editor.text(), "He🚀llo 👋 World 🌍");
    }

    #[test]
    fn test_utf8_multibyte_chars() {
        let mut editor = InputEditor::new();

        editor.insert_str("日本語");
        assert_eq!(editor.text(), "日本語");
        assert_eq!(editor.cursor(), 9);

        editor.set_cursor(6);
        assert!(editor.delete_char_before());
        assert_eq!(editor.text(), "日語");
        assert_eq!(editor.cursor(), 3);
    }

    #[test]
    fn test_paste_size_limit() {
        let mut editor = InputEditor::new();

        let large_text = "x".repeat(150 * 1024);
        let truncated = editor.insert_paste(&large_text);

        assert!(truncated, "Should indicate paste was truncated");
        assert!(editor.text().len() <= MAX_PASTE_SIZE);
    }

    #[test]
    fn test_paste_normal_size() {
        let mut editor = InputEditor::new();

        let text = "normal text";
        let truncated = editor.insert_paste(text);

        assert!(!truncated, "Should not truncate normal-sized paste");
        assert_eq!(editor.text(), text);
    }

    #[test]
    fn test_paste_newline_normalization() {
        let mut editor = InputEditor::new();

        editor.insert_paste("line1\r\nline2\r\n");
        assert_eq!(editor.text(), "line1\n\nline2\n\n");
    }

    #[test]
    fn test_navigation_empty_text() {
        let mut editor = InputEditor::new();

        editor.move_left();
        assert_eq!(editor.cursor(), 0);

        editor.move_right();
        assert_eq!(editor.cursor(), 0);

        editor.move_to_start();
        assert_eq!(editor.cursor(), 0);

        editor.move_to_end();
        assert_eq!(editor.cursor(), 0);

        assert!(!editor.delete_char_before());
        assert!(!editor.delete_char_at());
    }

    #[test]
    fn test_replace_range_invalid() {
        let mut editor = InputEditor::new();
        editor.insert_str("hello");

        editor.replace_range(5, 5, " world");
        assert_eq!(editor.text(), "hello world");

        editor.replace_range(6, 11, "universe");
        assert_eq!(editor.text(), "hello universe");
    }

    #[test]
    fn test_replace_range_multibyte() {
        let mut editor = InputEditor::new();
        editor.insert_str("hello 世界");

        let world_start = editor.text().char_indices().nth(6).map(|(i, _)| i).unwrap();
        editor.replace_range(world_start, editor.text().len(), "🌍");
        assert_eq!(editor.text(), "hello 🌍");
    }

    #[test]
    fn test_cursor_boundary_safety() {
        let mut editor = InputEditor::new();
        editor.insert_str("héllo");

        editor.set_cursor(2);
        assert_ne!(editor.cursor(), 2, "Cursor should not be in middle of char");
        assert!(editor.text().is_char_boundary(editor.cursor()));
    }

    #[test]
    fn test_char_at_cursor() {
        let mut editor = InputEditor::new();
        editor.insert_str("hello");

        editor.set_cursor(0);
        assert_eq!(editor.char_at_cursor(), Some('h'));

        editor.set_cursor(5);
        assert_eq!(editor.char_at_cursor(), None);

        editor.clear();
        assert_eq!(editor.char_at_cursor(), None);
    }

    #[test]
    fn test_text_before_after_cursor() {
        let mut editor = InputEditor::new();
        editor.insert_str("hello world");
        editor.set_cursor(5);

        assert_eq!(editor.text_before_cursor(), "hello");
        assert_eq!(editor.text_after_cursor(), " world");
    }

    #[test]
    fn test_delete_range_to_cursor() {
        let mut editor = InputEditor::new();
        editor.insert_str("hello world");
        editor.set_cursor(11);

        editor.delete_range_to_cursor(6);
        assert_eq!(editor.text(), "hello ");
        assert_eq!(editor.cursor(), 6);
    }

    // History navigation tests

    #[test]
    fn test_navigate_history_up_empty_history() {
        let mut editor = InputEditor::new();

        let navigated = editor.navigate_history_up();
        assert!(!navigated);
        assert!(editor.is_empty());
    }

    #[test]
    fn test_navigate_history_up_with_entries() {
        let mut editor = InputEditor::new();
        editor.history_mut().add("previous");

        let navigated = editor.navigate_history_up();
        assert!(navigated);
        assert_eq!(editor.text(), "previous");
        assert!(editor.is_navigating_history());
    }

    #[test]
    fn test_navigate_history_cycle() {
        let mut editor = InputEditor::new();
        editor.history_mut().add("oldest");
        editor.history_mut().add("newest");

        // Navigate up to newest
        editor.navigate_history_up();
        assert_eq!(editor.text(), "newest");

        // Navigate up to oldest
        editor.navigate_history_up();
        assert_eq!(editor.text(), "oldest");

        // Navigate down to newest
        editor.navigate_history_down();
        assert_eq!(editor.text(), "newest");

        // Navigate down to draft (empty)
        editor.navigate_history_down();
        assert!(editor.is_empty());
        assert!(!editor.is_navigating_history());
    }

    #[test]
    fn test_navigate_history_saves_draft() {
        let mut editor = InputEditor::new();
        editor.insert_str("my draft");
        editor.history_mut().add("previous");

        editor.navigate_history_up();
        assert_eq!(editor.text(), "previous");

        // Restore draft
        editor.navigate_history_down();
        assert_eq!(editor.text(), "my draft");
    }

    #[test]
    fn test_take_and_add_to_history() {
        let mut editor = InputEditor::new();
        editor.insert_str("  hello world  ");

        let text = editor.take_and_add_to_history();
        assert_eq!(text, "hello world");
        assert!(editor.is_empty());
        assert_eq!(editor.history().len(), 1);
    }

    #[test]
    fn test_take_and_add_to_history_empty() {
        let mut editor = InputEditor::new();

        let text = editor.take_and_add_to_history();
        assert!(text.is_empty());
        assert_eq!(editor.history().len(), 0);
    }

    #[test]
    fn test_insert_char_resets_navigation() {
        let mut editor = InputEditor::new();
        editor.history_mut().add("previous");

        editor.navigate_history_up();
        assert!(editor.is_navigating_history());

        editor.insert_char('a');
        assert!(!editor.is_navigating_history());
    }
}
