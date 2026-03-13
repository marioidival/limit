//! Input text editor for TUI
//!
//! Manages input text buffer with cursor position and editing operations.

use crate::tui::MAX_PASTE_SIZE;

/// Input text editor with cursor management
pub struct InputEditor {
    /// Text buffer
    text: String,
    /// Cursor position (byte offset)
    cursor: usize,
}

impl InputEditor {
    /// Create a new empty editor
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    /// Get the current text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get a mutable reference to the text
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// Get cursor position
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
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Check if cursor is at start
    pub fn is_cursor_at_start(&self) -> bool {
        self.cursor == 0
    }

    /// Check if cursor is at end
    pub fn is_cursor_at_end(&self) -> bool {
        self.cursor == self.text.len()
    }

    /// Insert a character at cursor position
    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Insert a string at cursor position
    pub fn insert_str(&mut self, s: &str) {
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    /// Insert paste with size limit
    /// Returns true if truncated
    pub fn insert_paste(&mut self, text: &str) -> bool {
        let (text, truncated) = self.truncate_paste(text);

        // Normalize newlines
        let normalized = text.replace("\r", "\n");
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
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.prev_char_pos();
        }
    }

    /// Move cursor right one character
    pub fn move_right(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor = self.next_char_pos();
        }
    }

    /// Move cursor to start
    pub fn move_to_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    pub fn move_to_end(&mut self) {
        self.cursor = self.text.len();
    }

    /// Clear all text
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    /// Get trimmed text and clear
    pub fn take_trimmed(&mut self) -> String {
        let trimmed = self.text.trim().to_string();
        self.clear();
        trimmed
    }

    /// Replace text in a range (used for autocomplete)
    pub fn replace_range(&mut self, start: usize, end: usize, replacement: &str) {
        self.text.drain(start..end);
        self.text.insert_str(start, replacement);
        self.cursor = start + replacement.len();
    }

    /// Delete from start to cursor
    pub fn delete_range_to_cursor(&mut self, start: usize) {
        if start < self.cursor {
            self.text.drain(start..self.cursor);
            self.cursor = start;
        }
    }

    /// Get text before cursor
    pub fn text_before_cursor(&self) -> &str {
        &self.text[..self.cursor]
    }

    /// Get text after cursor
    pub fn text_after_cursor(&self) -> &str {
        &self.text[self.cursor..]
    }

    /// Get character at cursor (if any)
    pub fn char_at_cursor(&self) -> Option<char> {
        self.text[self.cursor..].chars().next()
    }

    /// Get character before cursor (if any)
    pub fn char_before_cursor(&self) -> Option<char> {
        if self.cursor == 0 {
            return None;
        }
        let prev_pos = self.prev_char_pos_internal();
        self.text[prev_pos..self.cursor].chars().next()
    }

    /// Find previous char boundary
    fn prev_char_pos(&self) -> usize {
        self.prev_char_pos_internal()
    }

    fn prev_char_pos_internal(&self) -> usize {
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

    /// Truncate paste to max size
    fn truncate_paste<'a>(&self, text: &'a str) -> (&'a str, bool) {
        if text.len() > MAX_PASTE_SIZE {
            let truncated = &text[..text
                .char_indices()
                .nth(MAX_PASTE_SIZE)
                .map(|(i, _)| i)
                .unwrap_or(text.len())];
            (truncated, true)
        } else {
            (text, false)
        }
    }
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

        // Move to position after 'é'
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
}
