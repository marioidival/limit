//! Input text editor for TUI
//!
//! Manages input text buffer with cursor position, editing operations, and history navigation.

use crate::tui::input::history::InputHistory;
use crate::tui::MAX_PASTE_SIZE;
use std::path::PathBuf;

/// Maximum length for displaying pasted text before using placeholder
const MAX_DISPLAY_LENGTH: usize = 100;

/// Content from a large paste operation
struct PastedContent {
    /// Full text content for submission
    full_text: String,
    /// Display placeholder text for UI
    display_text: String,
    /// Length of text that existed before the paste (for display positioning)
    text_before_len: usize,
}

impl PastedContent {
    /// Create a new PastedContent from pasted text
    fn new(text: String) -> Self {
        let display_text = format_paste_placeholder(&text);
        Self {
            full_text: text,
            display_text,
            text_before_len: 0,
        }
    }

    /// Create PastedContent with existing text before the paste
    fn with_prefix(prefix: &str, pasted: &str) -> Self {
        let full_text = format!("{}{}", prefix, pasted);
        let display_text = format_paste_placeholder(pasted);
        Self {
            full_text,
            display_text,
            text_before_len: prefix.len(),
        }
    }
}

/// Format pasted text as a display placeholder
fn format_paste_placeholder(text: &str) -> String {
    let lines = text.lines().count();

    if lines > 1 {
        format!("[Pasted text + {} lines]", lines)
    } else {
        format!("[Pasted text + {} chars]", text.len())
    }
}

/// Compute display text zones for cursor navigation
/// Returns (text_before_end, placeholder_start, placeholder_end, total_len)
#[inline]
fn compute_display_zones(
    text_before_len: usize,
    placeholder_len: usize,
    total_text_len: usize,
) -> (usize, usize, usize, usize) {
    let text_after_len = total_text_len.saturating_sub(text_before_len);

    // Display structure: "{before} {placeholder} {after}"
    // But if before is empty: "{placeholder} {after}"
    // If after is empty: "{before} {placeholder}"
    // If both empty: "{placeholder}"

    let (text_before_end, placeholder_start, placeholder_end) = if text_before_len > 0 {
        (
            text_before_len,                       // end of text_before
            text_before_len + 1,                   // start of placeholder (after space)
            text_before_len + 1 + placeholder_len, // end of placeholder
        )
    } else {
        (0, 0, placeholder_len) // no text before, placeholder starts at 0
    };

    let total_len = if text_before_len > 0 && text_after_len > 0 {
        text_before_len + 1 + placeholder_len + 1 + text_after_len
    } else if text_before_len > 0 {
        text_before_len + 1 + placeholder_len
    } else if text_after_len > 0 {
        placeholder_len + 1 + text_after_len
    } else {
        placeholder_len
    };

    (
        text_before_end,
        placeholder_start,
        placeholder_end,
        total_len,
    )
}

/// Input text editor with cursor management and history
pub struct InputEditor {
    /// Text buffer (for normal typing or appended content after paste)
    text: String,
    /// Cursor position (byte offset)
    cursor: usize,
    /// Input history
    history: InputHistory,
    /// Large pasted content (displayed as placeholder)
    pasted_content: Option<PastedContent>,
    /// Cursor position in display text when pasted content exists
    /// This allows navigation through the combined display
    display_cursor: usize,
}

impl InputEditor {
    /// Create a new empty editor
    pub fn new() -> Self {
        Self {
            text: String::with_capacity(256),
            cursor: 0,
            history: InputHistory::new(),
            pasted_content: None,
            display_cursor: 0,
        }
    }

    /// Create editor with history loaded from file
    pub fn with_history(history_path: &PathBuf) -> Result<Self, String> {
        let history = InputHistory::load(history_path)?;
        Ok(Self {
            text: String::with_capacity(256),
            cursor: 0,
            history,
            pasted_content: None,
            display_cursor: 0,
        })
    }

    /// Get the current text (for submission)
    /// Returns full pasted content + any typed text after it
    #[inline]
    pub fn text(&self) -> String {
        if let Some(ref pasted) = self.pasted_content {
            let text_before_len = pasted.text_before_len;

            if self.text.len() <= text_before_len {
                // No text after paste - just return full_text
                pasted.full_text.clone()
            } else {
                // Text after paste - append it to full_text
                let text_after = &self.text[text_before_len..];
                format!("{}{}", pasted.full_text, text_after)
            }
        } else {
            self.text.clone()
        }
    }

    /// Get text for display in UI
    /// Returns placeholder for pasted content (use display_text_combined() for full display)
    #[inline]
    pub fn display_text(&self) -> &str {
        if let Some(ref pasted) = self.pasted_content {
            &pasted.display_text
        } else {
            &self.text
        }
    }

    /// Get display text with appended typed text
    /// Shows: "{text before} {placeholder} {text after}" depending on context
    pub fn display_text_combined(&self) -> std::borrow::Cow<'_, str> {
        if let Some(ref pasted) = self.pasted_content {
            let text_before_len = pasted.text_before_len;

            if text_before_len > 0 || self.text.len() > text_before_len {
                // Split text into before and after paste
                let text_before = &self.text[..text_before_len.min(self.text.len())];
                let text_after = if self.text.len() > text_before_len {
                    &self.text[text_before_len..]
                } else {
                    ""
                };

                // Build display: "{before} {placeholder} {after}"
                let mut result = String::new();
                if !text_before.is_empty() {
                    result.push_str(text_before);
                    result.push(' ');
                }
                result.push_str(&pasted.display_text);
                if !text_after.is_empty() {
                    result.push(' ');
                    result.push_str(text_after);
                }
                std::borrow::Cow::Owned(result)
            } else {
                std::borrow::Cow::Borrowed(&pasted.display_text)
            }
        } else {
            std::borrow::Cow::Borrowed(&self.text)
        }
    }

    /// Get a mutable reference to the text buffer
    /// Note: This only affects the typed text, not pasted content
    #[inline]
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// Get cursor position
    /// For pasted content, returns display_cursor position
    #[inline]
    pub fn cursor(&self) -> usize {
        if self.pasted_content.is_some() {
            self.display_cursor
        } else {
            self.cursor
        }
    }

    /// Calculate display text zones for cursor navigation
    /// Returns (text_before_end, placeholder_start, placeholder_end, total_len)
    fn display_zones(&self) -> (usize, usize, usize, usize) {
        if let Some(ref pasted) = self.pasted_content {
            compute_display_zones(
                pasted.text_before_len,
                pasted.display_text.len(),
                self.text.len(),
            )
        } else {
            (0, 0, 0, self.text.len())
        }
    }

    /// Set cursor position (clamped to valid range)
    pub fn set_cursor(&mut self, pos: usize) {
        if self.pasted_content.is_some() {
            let (_, _, _, total_len) = self.display_zones();
            self.display_cursor = pos.min(total_len);
        } else {
            self.cursor = pos.min(self.text.len());
            // Ensure cursor is at char boundary
            while self.cursor > 0 && !self.text.is_char_boundary(self.cursor) {
                self.cursor -= 1;
            }
        }
    }

    /// Check if text is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pasted_content.is_none() && self.text.is_empty()
    }

    /// Check if we have large pasted content (displayed as placeholder)
    #[inline]
    pub fn has_pasted_content(&self) -> bool {
        self.pasted_content.is_some()
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
    /// Note: If there's pasted content, position depends on cursor zone
    #[inline]
    pub fn insert_char(&mut self, c: char) {
        // Reset history navigation when user types
        if self.history.is_navigating() {
            self.history.reset_navigation();
        }

        // If we have pasted content, handle based on cursor position
        if let Some(ref pasted) = self.pasted_content {
            let (text_before_end, _, _, _) = compute_display_zones(
                pasted.text_before_len,
                pasted.display_text.len(),
                self.text.len(),
            );

            // If cursor is in text_before zone, insert there
            if self.display_cursor < text_before_end {
                let offset_in_before = self.display_cursor;
                self.text.insert(offset_in_before, c);
                // Need to update pasted content too
                if let Some(ref mut pasted) = self.pasted_content {
                    pasted.text_before_len += 1;
                    pasted.full_text.insert(offset_in_before, c);
                }
                self.display_cursor += c.len_utf8();
                return;
            }

            // Otherwise, append to text_after
            self.text.push(c);
            // Update display_cursor to end of combined text
            if let Some(ref pasted) = self.pasted_content {
                let (_, _, _, total_len) = compute_display_zones(
                    pasted.text_before_len,
                    pasted.display_text.len(),
                    self.text.len(),
                );
                self.display_cursor = total_len;
            }
            return;
        }

        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Insert a string at cursor position
    /// Note: If there's pasted content, position depends on cursor zone
    #[inline]
    pub fn insert_str(&mut self, s: &str) {
        // Reset history navigation when user types
        if self.history.is_navigating() {
            self.history.reset_navigation();
        }

        // If we have pasted content, handle based on cursor position
        if let Some(ref pasted) = self.pasted_content {
            let (text_before_end, _, _, _) = compute_display_zones(
                pasted.text_before_len,
                pasted.display_text.len(),
                self.text.len(),
            );

            // If cursor is in text_before zone, insert there
            if self.display_cursor < text_before_end {
                let offset_in_before = self.display_cursor;
                self.text.insert_str(offset_in_before, s);
                // Need to update pasted content too
                if let Some(ref mut pasted) = self.pasted_content {
                    pasted.text_before_len += s.len();
                    pasted.full_text.insert_str(offset_in_before, s);
                }
                self.display_cursor += s.len();
                return;
            }

            // Otherwise, append to text_after
            self.text.push_str(s);
            // Update display_cursor to end of combined text
            if let Some(ref pasted) = self.pasted_content {
                let (_, _, _, total_len) = compute_display_zones(
                    pasted.text_before_len,
                    pasted.display_text.len(),
                    self.text.len(),
                );
                self.display_cursor = total_len;
            }
            return;
        }

        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    /// Insert paste with size limit
    /// Returns true if truncated
    pub fn insert_paste(&mut self, text: &str) -> bool {
        tracing::trace!(
            "insert_paste: input len={}, existing text len={}, has_pasted_content={}",
            text.len(),
            self.text.len(),
            self.pasted_content.is_some()
        );

        let (text, truncated) = truncate_paste(text);

        // Normalize newlines with pre-allocated capacity
        let normalized = if text.contains('\r') {
            let mut normalized = String::with_capacity(text.len());
            for c in text.chars() {
                normalized.push(if c == '\r' { '\n' } else { c });
            }
            normalized
        } else {
            text.to_string()
        };

        tracing::trace!(
            "insert_paste: normalized len={}, will_create_placeholder={}",
            normalized.len(),
            normalized.len() > MAX_DISPLAY_LENGTH
        );

        // If we already have pasted content, append to it
        if let Some(ref mut pasted) = self.pasted_content {
            tracing::trace!("insert_paste: appending to existing pasted content");
            pasted.full_text.push_str(&normalized);
            pasted.display_text = format_paste_placeholder(&pasted.full_text);
            return truncated;
        }

        // If we have existing typed text, handle paste separately
        // to keep existing text visible
        if !self.text.is_empty() {
            tracing::trace!(
                "insert_paste: existing text '{}' (len={}), paste len={}",
                &self.text,
                self.text.len(),
                normalized.len()
            );

            // If paste is large, create placeholder for paste only
            // Keep existing text in self.text for visibility
            if normalized.len() > MAX_DISPLAY_LENGTH {
                tracing::trace!(
                    "insert_paste: paste is large, creating placeholder for paste only"
                );
                // Use with_prefix to track the text that came before the paste
                let original_text = self.text.clone();
                self.pasted_content = Some(PastedContent::with_prefix(&original_text, &normalized));
                // Keep original text in self.text so it's displayed before the placeholder
                // self.text already contains the original text, don't change it
                // Set display_cursor to end of combined text
                self.display_cursor = self.display_zones().3;
                return truncated;
            }

            // Small paste - append normally
            self.text.push_str(&normalized);
            tracing::trace!(
                "insert_paste: small paste appended, total len={}",
                self.text.len()
            );

            // If combined text is now large, convert to placeholder
            if self.text.len() > MAX_DISPLAY_LENGTH {
                tracing::trace!(
                    "insert_paste: combined text len={} > {}, converting to pasted content",
                    self.text.len(),
                    MAX_DISPLAY_LENGTH
                );
                let full_text = std::mem::take(&mut self.text);
                self.pasted_content = Some(PastedContent::new(full_text));
                self.display_cursor = self.display_zones().3;
            }

            return truncated;
        }

        // No existing content - if paste is large, use placeholder display
        if normalized.len() > MAX_DISPLAY_LENGTH {
            tracing::trace!("insert_paste: no existing content, creating placeholder");
            self.pasted_content = Some(PastedContent::new(normalized));
            self.display_cursor = self.display_zones().3;
            return truncated;
        }

        // Small paste with no existing content - insert normally
        tracing::trace!("insert_paste: small paste, inserting normally");
        self.text = normalized;
        self.cursor = self.text.len();
        truncated
    }

    /// Delete character before cursor (backspace)
    pub fn delete_char_before(&mut self) -> bool {
        // If we have pasted content, handle based on cursor position in display
        if let Some(ref pasted) = self.pasted_content {
            let text_before_len = pasted.text_before_len;
            let text_after_len = self.text.len().saturating_sub(text_before_len);
            let (text_before_end, placeholder_start, placeholder_end, total_len) =
                compute_display_zones(text_before_len, pasted.display_text.len(), self.text.len());
            let text_after_start = placeholder_end + 1; // +1 for space

            tracing::trace!(
                "delete_char_before: display_cursor={}, zones=({},{},{},{}), text_after_len={}",
                self.display_cursor,
                text_before_end,
                placeholder_start,
                placeholder_end,
                total_len,
                text_after_len
            );

            // Case 1: Cursor is in text_after zone - delete char from text_after
            if self.display_cursor > text_after_start && text_after_len > 0 {
                // Find the char before cursor in text_after
                let text_after = &self.text[text_before_len..];
                let offset_in_after = self.display_cursor - text_after_start;

                if offset_in_after > 0 {
                    // Delete the char at offset_in_after - 1
                    let char_offset = text_after.chars().take(offset_in_after).count();
                    if char_offset > 0 {
                        let delete_char_offset = char_offset - 1;
                        let delete_start = text_after
                            .char_indices()
                            .nth(delete_char_offset)
                            .map(|(i, _)| i)
                            .unwrap_or(0);

                        self.text.remove(text_before_len + delete_start);

                        // Recalculate zones after deletion
                        let new_text_after_len = self.text.len().saturating_sub(text_before_len);
                        if new_text_after_len == 0 {
                            // No more text_after, cursor goes to placeholder_end
                            self.display_cursor = placeholder_end;
                        } else {
                            // Update cursor position
                            self.display_cursor = text_after_start + delete_start;
                        }
                        return true;
                    }
                }
            }

            // Case 2: Cursor is at start of text_after (right after space) - jump to placeholder end
            if self.display_cursor == text_after_start && text_after_len > 0 {
                self.display_cursor = placeholder_end;
                return true;
            }

            // Case 3: Cursor is right after placeholder (at placeholder_end) - delete the placeholder
            // This DISCARDS the pasted content, keeping only typed text
            if self.display_cursor == placeholder_end {
                // Keep only text_before and text_after (typed text), discard pasted content
                let text_before = if text_before_len > 0 {
                    self.text[..text_before_len].to_string()
                } else {
                    String::new()
                };
                let text_after = if text_after_len > 0 {
                    self.text[text_before_len..].to_string()
                } else {
                    String::new()
                };

                // Clear pasted content (discard it)
                self.pasted_content = None;

                // Set text to: text_before + text_after (no pasted_text)
                self.text = format!("{}{}", text_before, text_after);

                // Set cursor to end of text_before
                self.cursor = text_before.len();
                self.display_cursor = 0;
                return true;
            }

            // Case 4: Cursor is inside placeholder - jump to start
            if self.display_cursor > placeholder_start && self.display_cursor < placeholder_end {
                self.display_cursor = placeholder_start;
                return true;
            }

            // Case 5: Cursor is at placeholder start (right after space or at beginning)
            if self.display_cursor == placeholder_start {
                if text_before_len > 0 {
                    // Jump to end of text_before
                    self.display_cursor = text_before_end;
                    return true;
                } else {
                    // No text before - discard the placeholder and all pasted content
                    self.pasted_content = None;
                    self.text.clear();
                    self.cursor = 0;
                    self.display_cursor = 0;
                    return true;
                }
            }

            // Case 6: Cursor is in the space between text_before and placeholder
            if self.display_cursor > text_before_end && self.display_cursor <= placeholder_start {
                self.display_cursor = text_before_end;
                return true;
            }

            // Case 7: Cursor is in text_before zone - delete char
            if self.display_cursor > 0 && self.display_cursor <= text_before_end {
                let text_before = &self.text[..text_before_len];
                let offset_in_before = self.display_cursor;

                if offset_in_before > 0 {
                    let char_offset = text_before.chars().take(offset_in_before).count();
                    if char_offset > 0 {
                        let delete_char_offset = char_offset - 1;
                        let delete_start = text_before
                            .char_indices()
                            .nth(delete_char_offset)
                            .map(|(i, _)| i)
                            .unwrap_or(0);

                        self.text.remove(delete_start);

                        // Update pasted content
                        if let Some(ref mut pasted) = self.pasted_content {
                            pasted.text_before_len -= 1;
                            let paste_portion = if pasted.full_text.len() > text_before_len {
                                pasted.full_text[text_before_len..].to_string()
                            } else {
                                String::new()
                            };
                            pasted.full_text = format!(
                                "{}{}",
                                &self.text[..pasted.text_before_len],
                                paste_portion
                            );
                        }

                        self.display_cursor = delete_start;
                        return true;
                    }
                }
            }

            // Case 8: Cursor is at start (0) - nothing to delete
            if self.display_cursor == 0 {
                return false;
            }

            return false;
        }

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
        // If we have pasted content, handle based on cursor position in display
        if let Some(ref pasted) = self.pasted_content {
            let text_before_len = pasted.text_before_len;
            let text_after_len = self.text.len().saturating_sub(text_before_len);
            let (text_before_end, placeholder_start, placeholder_end, total_len) =
                compute_display_zones(text_before_len, pasted.display_text.len(), self.text.len());
            let text_after_start = placeholder_end + 1;

            tracing::trace!(
                "delete_char_at: display_cursor={}, zones=({},{},{},{}), text_after_len={}",
                self.display_cursor,
                text_before_end,
                placeholder_start,
                placeholder_end,
                total_len,
                text_after_len
            );

            // Case 1: Cursor is in text_after zone - delete char at cursor
            if self.display_cursor >= text_after_start && text_after_len > 0 {
                let text_after = &self.text[text_before_len..];
                let offset_in_after = self.display_cursor - text_after_start;

                if offset_in_after < text_after.len() {
                    self.text.remove(text_before_len + offset_in_after);
                    // Keep cursor position (or adjust if needed)
                    let new_total = self.display_zones().3;
                    if self.display_cursor > new_total {
                        self.display_cursor = new_total;
                    }
                    return true;
                }
            }

            // Case 2: Cursor is at placeholder_end (before space to text_after) - delete placeholder
            // This DISCARDS the pasted content, keeping only typed text
            if self.display_cursor == placeholder_end {
                let text_before = if text_before_len > 0 {
                    self.text[..text_before_len].to_string()
                } else {
                    String::new()
                };
                let text_after = if text_after_len > 0 {
                    self.text[text_before_len..].to_string()
                } else {
                    String::new()
                };

                // Discard pasted content
                self.pasted_content = None;
                self.text = format!("{}{}", text_before, text_after);
                self.cursor = text_before.len();
                self.display_cursor = 0;
                return true;
            }

            // Case 3: Cursor is inside placeholder - delete placeholder
            // This DISCARDS the pasted content, keeping only typed text
            if self.display_cursor >= placeholder_start && self.display_cursor < placeholder_end {
                let text_before = if text_before_len > 0 {
                    self.text[..text_before_len].to_string()
                } else {
                    String::new()
                };
                let text_after = if text_after_len > 0 {
                    self.text[text_before_len..].to_string()
                } else {
                    String::new()
                };

                // Discard pasted content
                self.pasted_content = None;
                self.text = format!("{}{}", text_before, text_after);
                self.cursor = text_before.len();
                self.display_cursor = 0;
                return true;
            }

            // Case 4: Cursor is at placeholder_start - delete placeholder
            // This DISCARDS the pasted content, keeping only typed text
            if self.display_cursor == placeholder_start {
                let text_before = if text_before_len > 0 {
                    self.text[..text_before_len].to_string()
                } else {
                    String::new()
                };
                let text_after = if text_after_len > 0 {
                    self.text[text_before_len..].to_string()
                } else {
                    String::new()
                };

                // Discard pasted content
                self.pasted_content = None;
                self.text = format!("{}{}", text_before, text_after);
                self.cursor = text_before.len();
                self.display_cursor = 0;
                return true;
            }

            // Case 5: Cursor is in text_before zone - delete char after cursor
            if self.display_cursor < text_before_end && text_before_len > 0 {
                let offset_in_before = self.display_cursor;
                if offset_in_before < text_before_len {
                    self.text.remove(offset_in_before);

                    if let Some(ref mut pasted) = self.pasted_content {
                        pasted.text_before_len -= 1;
                        let paste_portion = if pasted.full_text.len() > text_before_len {
                            pasted.full_text[text_before_len..].to_string()
                        } else {
                            String::new()
                        };
                        pasted.full_text =
                            format!("{}{}", &self.text[..pasted.text_before_len], paste_portion);
                    }
                    return true;
                }
            }

            return false;
        }

        if self.cursor >= self.text.len() {
            return false;
        }

        let next_pos = self.next_char_pos();
        self.text.drain(self.cursor..next_pos);
        true
    }

    /// Move cursor left one character
    /// For pasted content, navigates through display text zones
    #[inline]
    pub fn move_left(&mut self) {
        if let Some(ref pasted) = self.pasted_content {
            let (text_before_end, placeholder_start, placeholder_end, total_len) =
                self.display_zones();

            tracing::trace!(
                "move_left: display_cursor={}, zones=({},{},{},{}), text_before_len={}",
                self.display_cursor,
                text_before_end,
                placeholder_start,
                placeholder_end,
                total_len,
                pasted.text_before_len
            );

            if self.display_cursor == 0 {
                return;
            }

            // Determine which zone we're in and handle movement
            if self.display_cursor > placeholder_end {
                // In text_after zone - move left one char
                let text_after_start = placeholder_end + 1; // +1 for space
                if self.display_cursor > text_after_start {
                    // Find char boundary in text_after
                    let _text_after_len = self.text.len().saturating_sub(pasted.text_before_len);
                    let offset_in_after = self.display_cursor - text_after_start;
                    if offset_in_after > 0 {
                        let text_after = &self.text[pasted.text_before_len..];
                        // Find previous char boundary
                        let char_offset = text_after.chars().take(offset_in_after).count();
                        if char_offset > 0 {
                            let new_char_offset = char_offset - 1;
                            let new_byte_offset = text_after
                                .char_indices()
                                .nth(new_char_offset)
                                .map(|(i, _)| i)
                                .unwrap_or(0);
                            self.display_cursor = text_after_start + new_byte_offset;
                        } else {
                            self.display_cursor = text_after_start;
                        }
                    } else {
                        self.display_cursor = placeholder_end;
                    }
                } else {
                    self.display_cursor = placeholder_end;
                }
            } else if self.display_cursor > placeholder_start {
                // Inside placeholder - jump to start of placeholder
                self.display_cursor = placeholder_start;
            } else if self.display_cursor > text_before_end {
                // In the space between text_before and placeholder
                self.display_cursor = text_before_end;
            } else if self.display_cursor > 0 {
                // In text_before zone - move left one char
                let text_before = &self.text[..pasted.text_before_len];
                let offset_in_before = self.display_cursor;
                if offset_in_before > 0 {
                    let char_offset = text_before.chars().take(offset_in_before).count();
                    if char_offset > 0 {
                        let new_char_offset = char_offset - 1;
                        let new_byte_offset = text_before
                            .char_indices()
                            .nth(new_char_offset)
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        self.display_cursor = new_byte_offset;
                    } else {
                        self.display_cursor = 0;
                    }
                }
            }
            return;
        }

        if self.cursor > 0 {
            self.cursor = self.prev_char_pos();
        }
    }

    /// Move cursor right one character
    /// For pasted content, navigates through display text zones
    #[inline]
    pub fn move_right(&mut self) {
        if let Some(ref pasted) = self.pasted_content {
            let (text_before_end, placeholder_start, placeholder_end, total_len) =
                self.display_zones();

            tracing::trace!(
                "move_right: display_cursor={}, zones=({},{},{},{}), text_before_len={}",
                self.display_cursor,
                text_before_end,
                placeholder_start,
                placeholder_end,
                total_len,
                pasted.text_before_len
            );

            if self.display_cursor >= total_len {
                return;
            }

            // Determine which zone we're in and handle movement
            if self.display_cursor >= placeholder_end {
                // At or after placeholder end - move in text_after zone
                let text_after_start = placeholder_end + 1; // +1 for space
                let text_after_len = self.text.len().saturating_sub(pasted.text_before_len);

                if text_after_len > 0 && self.display_cursor >= text_after_start {
                    let text_after = &self.text[pasted.text_before_len..];
                    let offset_in_after = self.display_cursor - text_after_start;

                    // Find next char boundary
                    let char_offset = text_after.chars().take(offset_in_after).count();
                    let total_chars = text_after.chars().count();

                    if char_offset < total_chars {
                        let new_char_offset = char_offset + 1;
                        let new_byte_offset = text_after
                            .char_indices()
                            .nth(new_char_offset)
                            .map(|(i, _)| i)
                            .unwrap_or(text_after.len());
                        self.display_cursor = (text_after_start + new_byte_offset).min(total_len);
                    }
                } else if self.display_cursor == placeholder_end && text_after_len > 0 {
                    // At placeholder end with text_after - jump to start of text_after
                    self.display_cursor = text_after_start;
                }
            } else if self.display_cursor >= placeholder_start {
                // Inside or at start of placeholder - jump to end
                self.display_cursor = placeholder_end;
            } else if self.display_cursor >= text_before_end {
                // In the space between text_before and placeholder - jump to placeholder end
                self.display_cursor = placeholder_end;
            } else {
                // In text_before zone - move right one char
                let text_before = &self.text[..pasted.text_before_len];
                let offset_in_before = self.display_cursor;
                let char_offset = text_before.chars().take(offset_in_before).count();
                let total_chars = text_before.chars().count();

                if char_offset < total_chars {
                    let new_char_offset = char_offset + 1;
                    let new_byte_offset = text_before
                        .char_indices()
                        .nth(new_char_offset)
                        .map(|(i, _)| i)
                        .unwrap_or(text_before.len());
                    self.display_cursor = new_byte_offset;
                } else {
                    // At end of text_before - jump to placeholder end (skip space and placeholder)
                    self.display_cursor = placeholder_end;
                }
            }
            return;
        }

        if self.cursor < self.text.len() {
            self.cursor = self.next_char_pos();
        }
    }

    /// Move cursor to start
    #[inline]
    pub fn move_to_start(&mut self) {
        if self.pasted_content.is_some() {
            self.display_cursor = 0;
        } else {
            self.cursor = 0;
        }
    }

    /// Move cursor to end
    #[inline]
    pub fn move_to_end(&mut self) {
        if self.pasted_content.is_some() {
            self.display_cursor = self.display_zones().3;
        } else {
            self.cursor = self.text.len();
        }
    }

    /// Clear all text
    #[inline]
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.pasted_content = None;
        self.display_cursor = 0;
    }

    /// Get trimmed text and clear
    pub fn take_trimmed(&mut self) -> String {
        let full_text = self.text();
        let trimmed = full_text.trim();
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
        let current = self.text();
        let current = current.trim();

        if let Some(entry) = self.history.navigate_up(current) {
            self.text = entry.to_string();
            self.cursor = self.text.len();
            self.pasted_content = None;
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
        let full_text = self.text();
        let text = full_text.trim().to_string();

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
        assert_eq!(editor.text(), "hi".to_string());
        assert_eq!(editor.cursor(), 2);
    }

    #[test]
    fn test_delete_char_before() {
        let mut editor = InputEditor::new();
        editor.insert_str("hello");
        editor.set_cursor(3);

        assert!(editor.delete_char_before());
        assert_eq!(editor.text(), "helo".to_string());
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
        assert_eq!(editor.text(), "hello universe".to_string());
    }

    #[test]
    fn test_utf8_emojis() {
        let mut editor = InputEditor::new();

        editor.insert_str("Hello 👋 World 🌍");
        assert_eq!(editor.text(), "Hello 👋 World 🌍".to_string());

        editor.move_to_start();
        editor.move_right();
        editor.move_right();

        editor.insert_char('🚀');
        assert_eq!(editor.text(), "He🚀llo 👋 World 🌍".to_string());
    }

    #[test]
    fn test_utf8_multibyte_chars() {
        let mut editor = InputEditor::new();

        editor.insert_str("日本語");
        assert_eq!(editor.text(), "日本語".to_string());
        assert_eq!(editor.cursor(), 9);

        editor.set_cursor(6);
        assert!(editor.delete_char_before());
        assert_eq!(editor.text(), "日語".to_string());
        assert_eq!(editor.cursor(), 3);
    }

    #[test]
    fn test_paste_size_limit() {
        let mut editor = InputEditor::new();

        // Create text larger than MAX_PASTE_SIZE (300KB)
        let large_text = "x".repeat(350 * 1024);
        let truncated = editor.insert_paste(&large_text);

        assert!(truncated, "Should indicate paste was truncated");
        // Text should be truncated to MAX_PASTE_SIZE
        assert!(
            editor.text().len() <= MAX_PASTE_SIZE,
            "Text length {} should be <= MAX_PASTE_SIZE {}",
            editor.text().len(),
            MAX_PASTE_SIZE
        );
        // Display should show placeholder
        assert!(editor.display_text().starts_with("[Pasted text +"));
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
        assert_eq!(editor.text(), "line1\n\nline2\n\n".to_string());
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
        assert_eq!(editor.text(), "hello world".to_string());

        editor.replace_range(6, 11, "universe");
        assert_eq!(editor.text(), "hello universe".to_string());
    }

    #[test]
    fn test_replace_range_multibyte() {
        let mut editor = InputEditor::new();
        editor.insert_str("hello 世界");

        let world_start = editor.text().char_indices().nth(6).map(|(i, _)| i).unwrap();
        editor.replace_range(world_start, editor.text().len(), "🌍");
        assert_eq!(editor.text(), "hello 🌍".to_string());
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
        assert_eq!(editor.text(), "hello ".to_string());
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
        assert_eq!(editor.text(), "previous".to_string());
        assert!(editor.is_navigating_history());
    }

    #[test]
    fn test_navigate_history_cycle() {
        let mut editor = InputEditor::new();
        editor.history_mut().add("oldest");
        editor.history_mut().add("newest");

        // Navigate up to newest
        editor.navigate_history_up();
        assert_eq!(editor.text(), "newest".to_string());

        // Navigate up to oldest
        editor.navigate_history_up();
        assert_eq!(editor.text(), "oldest".to_string());

        // Navigate down to newest
        editor.navigate_history_down();
        assert_eq!(editor.text(), "newest".to_string());

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
        assert_eq!(editor.text(), "previous".to_string());

        // Restore draft
        editor.navigate_history_down();
        assert_eq!(editor.text(), "my draft".to_string());
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

    // Paste placeholder tests

    #[test]
    fn test_large_paste_creates_placeholder() {
        let mut editor = InputEditor::new();

        let large_text = "x".repeat(150);
        let truncated = editor.insert_paste(&large_text);

        assert!(!truncated, "Should not truncate text under MAX_PASTE_SIZE");
        assert_eq!(
            editor.text(),
            large_text,
            "text() should return full content"
        );
        assert_eq!(editor.display_text(), "[Pasted text + 150 chars]");
        assert!(!editor.is_empty());
    }

    #[test]
    fn test_multiline_paste_shows_lines() {
        let mut editor = InputEditor::new();

        // Create multiline text with > 100 chars to trigger placeholder
        let multiline = "line number one with some text\nline number two with more text\nline number three here\nline number four with content\nline number five has text";
        assert!(multiline.len() > 100, "Test text should be > 100 chars");
        editor.insert_paste(multiline);

        assert_eq!(editor.display_text(), "[Pasted text + 5 lines]");
        assert_eq!(editor.text(), multiline);
    }

    #[test]
    fn test_small_paste_no_placeholder() {
        let mut editor = InputEditor::new();

        let small_text = "hello world";
        editor.insert_paste(small_text);

        assert_eq!(editor.display_text(), small_text);
        assert_eq!(editor.text(), small_text);
    }

    #[test]
    fn test_typing_after_paste_appends() {
        let mut editor = InputEditor::new();

        let large_text = "x".repeat(150);
        editor.insert_paste(&large_text);

        assert_eq!(editor.display_text(), "[Pasted text + 150 chars]");

        // Type characters - should be stored separately and shown after placeholder
        editor.insert_char('!');
        editor.insert_char(' ');
        editor.insert_char('t');
        editor.insert_char('e');
        editor.insert_char('s');
        editor.insert_char('t');

        // Display should show placeholder + space + typed text
        assert_eq!(
            editor.display_text_combined(),
            "[Pasted text + 150 chars] ! test"
        );

        // Full text should be paste + typed text
        assert_eq!(editor.text(), format!("{}! test", large_text));

        // Should still have pasted_content
        assert!(editor.has_pasted_content());
    }

    #[test]
    fn test_clear_removes_pasted_content() {
        let mut editor = InputEditor::new();

        let large_text = "x".repeat(150);
        editor.insert_paste(&large_text);

        assert!(!editor.is_empty());

        editor.clear();

        assert!(editor.is_empty());
        assert!(editor.pasted_content.is_none());
    }

    #[test]
    fn test_take_and_add_to_history_with_pasted_content() {
        let mut editor = InputEditor::new();

        let large_text = "x".repeat(150);
        editor.insert_paste(&large_text);

        let text = editor.take_and_add_to_history();

        assert_eq!(text, large_text);
        assert!(editor.is_empty());
        assert_eq!(editor.history().len(), 1);
    }

    #[test]
    fn test_paste_appends_to_typed_text() {
        let mut editor = InputEditor::new();

        editor.insert_str("Hello ");

        let paste_text = "x".repeat(150);
        editor.insert_paste(&paste_text);

        // Should show placeholder
        assert!(editor.display_text().starts_with("[Pasted text +"));
        // Full text should be "Hello " + paste
        assert!(editor.text().starts_with("Hello "));
        assert!(editor.text().ends_with(&paste_text));
    }

    #[test]
    fn test_paste_appends_to_existing_paste() {
        let mut editor = InputEditor::new();

        let paste1 = "x".repeat(150);
        editor.insert_paste(&paste1);

        let paste2 = "y".repeat(50);
        editor.insert_paste(&paste2);

        // Should still show placeholder
        assert!(editor.display_text().starts_with("[Pasted text +"));
        // Full text should be both pastes combined
        assert!(editor.text().starts_with(&paste1));
        assert!(editor.text().ends_with(&paste2));
    }

    #[test]
    fn test_cursor_navigation_enabled_with_paste() {
        let mut editor = InputEditor::new();

        let large_text = "x".repeat(150);
        editor.insert_paste(&large_text);

        let initial_cursor = editor.cursor();

        // Cursor navigation should now work
        editor.move_left();
        assert!(editor.cursor() < initial_cursor, "Cursor should move left");

        editor.move_right();
        assert_eq!(
            editor.cursor(),
            initial_cursor,
            "Cursor should return to end"
        );
    }

    #[test]
    fn test_backspace_deletes_placeholder_discards_content() {
        let mut editor = InputEditor::new();

        let large_text = "x".repeat(150);
        editor.insert_paste(&large_text);

        assert!(!editor.is_empty());

        // Move cursor left to be at placeholder end position
        editor.move_left();

        // Backspace at placeholder position should delete the placeholder
        // and DISCARD the pasted content
        let deleted = editor.delete_char_before();
        assert!(deleted);
        // After deleting placeholder, pasted content is removed
        assert!(editor.pasted_content.is_none());
        // Text should be empty since there was no typed text before/after
        assert!(editor.is_empty());
        assert_eq!(editor.text(), "");
    }

    #[test]
    fn test_delete_key_deletes_placeholder_discards_content() {
        let mut editor = InputEditor::new();

        let large_text = "x".repeat(150);
        editor.insert_paste(&large_text);

        assert!(!editor.is_empty());

        // Move cursor to start of placeholder
        editor.move_to_start();

        // Delete key at placeholder should delete the placeholder
        // and DISCARD the pasted content
        let deleted = editor.delete_char_at();
        assert!(deleted);
        // After deleting placeholder, pasted content is removed
        assert!(editor.pasted_content.is_none());
        // Text should be empty since there was no typed text before/after
        assert!(editor.is_empty());
        assert_eq!(editor.text(), "");
    }

    #[test]
    fn test_backspace_placeholder_keeps_typed_text() {
        let mut editor = InputEditor::new();

        // Type text first
        editor.insert_str("hello ");

        // Paste large content
        let large_text = "x".repeat(150);
        editor.insert_paste(&large_text);

        // Display: "hello [Pasted text + 150 chars]"
        assert!(editor.has_pasted_content());

        // Move cursor to end of display (after placeholder)
        editor.move_to_end();

        // Backspace to delete the placeholder
        let deleted = editor.delete_char_before();
        assert!(deleted);

        // Pasted content should be discarded, but "hello " should remain
        assert!(editor.pasted_content.is_none());
        assert_eq!(editor.text(), "hello ");
    }

    #[test]
    fn test_cursor_at_end_of_placeholder() {
        let mut editor = InputEditor::new();

        let large_text = "x".repeat(150);
        editor.insert_paste(&large_text);

        // Cursor should be at end of display text (placeholder)
        let display = editor.display_text();
        assert_eq!(editor.cursor(), display.len());
    }

    #[test]
    fn test_small_paste_converts_to_pasted_content_when_combined() {
        let mut editor = InputEditor::new();

        // Two small pastes that together exceed 100 chars
        let paste1 = "x".repeat(60);
        let paste2 = "y".repeat(60);

        editor.insert_paste(&paste1);
        assert_eq!(editor.display_text(), &paste1); // No placeholder yet

        editor.insert_paste(&paste2);
        // Now combined > 100, should show placeholder
        assert!(editor.display_text().starts_with("[Pasted text +"));
        assert_eq!(editor.text(), format!("{}{}", paste1, paste2));
    }

    #[test]
    fn test_cursor_position_after_paste_with_typed_text() {
        let mut editor = InputEditor::new();

        // Type some text first
        editor.insert_str("Hello ");

        // Paste large content
        let large_text = "x".repeat(150);
        editor.insert_paste(&large_text);

        // Type more text after paste
        editor.insert_char('!');
        editor.insert_char(' ');
        editor.insert_char('w');
        editor.insert_char('o');
        editor.insert_char('r');
        editor.insert_char('l');
        editor.insert_char('d');

        // The combined display text is: "[Pasted text + N chars] ! world"
        let combined = editor.display_text_combined();
        let combined_len = combined.len();
        let cursor_pos = editor.cursor();

        // BUG: cursor() should be at the end of combined text for proper rendering
        // Currently cursor() returns pasted.display_text.len() which is shorter
        // than the combined text when there's additional typed content
        assert_eq!(
            cursor_pos, combined_len,
            "Cursor should be at end of combined display text! cursor={}, combined_len={}",
            cursor_pos, combined_len
        );
    }

    #[test]
    fn test_text_before_and_after_paste() {
        let mut editor = InputEditor::new();

        // Type text before paste
        editor.insert_str("ola");

        // Paste large content
        let large_text = "x".repeat(150);
        editor.insert_paste(&large_text);

        // Type text after paste
        editor.insert_char(' ');
        editor.insert_char('m');
        editor.insert_char('u');
        editor.insert_char('n');
        editor.insert_char('d');
        editor.insert_char('o');

        // Display should show: "ola [Pasted text + 150 chars] mundo"
        let display = editor.display_text_combined();
        assert!(
            display.starts_with("ola [Pasted text +"),
            "Display should start with 'ola [Pasted text +', got: '{}'",
            display
        );
        assert!(
            display.ends_with(" mundo"),
            "Display should end with ' mundo', got: '{}'",
            display
        );

        // Full text should be: "ola" + paste + " mundo"
        let full_text = editor.text();
        assert!(full_text.starts_with("ola"));
        assert!(full_text.ends_with(" mundo"));

        // Cursor should be at end of display
        let cursor_pos = editor.cursor();
        let display_len = display.len();
        assert_eq!(
            cursor_pos, display_len,
            "Cursor should be at end of display: cursor={}, display_len={}",
            cursor_pos, display_len
        );
    }
}
