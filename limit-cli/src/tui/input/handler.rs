//! Input event handler for TUI
//!
//! Handles keyboard events, mouse events, and delegates to appropriate handlers.

use crate::error::CliError;
use crate::tui::MAX_PASTE_SIZE;
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use std::time::Instant;

/// Actions that can result from input handling
#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    /// No action taken
    None,
    /// Submit input text
    Submit(String),
    /// Exit the application
    Exit,
    /// Cancel current operation
    Cancel,
    /// Scroll up in chat
    ScrollUp,
    /// Scroll down in chat
    ScrollDown,
    /// Scroll up one page
    PageUp,
    /// Scroll down one page
    PageDown,
    /// Start file autocomplete
    StartAutocomplete,
    /// Update autocomplete query
    UpdateAutocomplete(char),
    /// Navigate autocomplete up
    AutocompleteUp,
    /// Navigate autocomplete down
    AutocompleteDown,
    /// Accept autocomplete selection
    AutocompleteAccept,
    /// Cancel autocomplete
    AutocompleteCancel,
    /// Copy selection to clipboard
    CopySelection,
    /// Paste from clipboard
    Paste,
    /// Show help message
    ShowHelp,
    /// Clear chat
    ClearChat,
    /// Handle command
    HandleCommand(String),
}

/// Input handler for processing keyboard and mouse events
pub struct InputHandler {
    /// Last ESC press time for double-ESC detection
    last_esc_time: Option<Instant>,
    /// Cursor blink state
    cursor_blink_state: bool,
    /// Cursor blink timer
    cursor_blink_timer: Instant,
}

impl InputHandler {
    /// Create a new input handler
    pub fn new() -> Self {
        Self {
            last_esc_time: None,
            cursor_blink_state: true,
            cursor_blink_timer: Instant::now(),
        }
    }

    /// Handle a keyboard event and return the action to take
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        input_text: &str,
        cursor_pos: usize,
        is_busy: bool,
        has_autocomplete: bool,
    ) -> Result<InputAction, CliError> {
        // Log key event for debugging
        tracing::trace!(
            "handle_key: code={:?} mod={:?} kind={:?}",
            key.code,
            key.modifiers,
            key.kind
        );

        // Only process Press events (not Release)
        if key.kind != KeyEventKind::Press {
            return Ok(InputAction::None);
        }

        // Handle copy/paste shortcuts first
        if self.is_copy_paste_modifier(&key, 'c') {
            return Ok(InputAction::CopySelection);
        }

        if self.is_copy_paste_modifier(&key, 'v') && !is_busy {
            return Ok(InputAction::Paste);
        }

        // Handle autocomplete navigation
        if has_autocomplete {
            match key.code {
                KeyCode::Up => return Ok(InputAction::AutocompleteUp),
                KeyCode::Down => return Ok(InputAction::AutocompleteDown),
                KeyCode::Enter | KeyCode::Tab => return Ok(InputAction::AutocompleteAccept),
                KeyCode::Esc => return Ok(InputAction::AutocompleteCancel),
                _ => {}
            }
        }

        // Handle ESC for cancellation or exit
        if key.code == KeyCode::Esc {
            if has_autocomplete {
                return Ok(InputAction::AutocompleteCancel);
            } else if is_busy {
                // Double-ESC to cancel
                let now = Instant::now();
                let should_cancel = if let Some(last_esc) = self.last_esc_time {
                    now.duration_since(last_esc) < std::time::Duration::from_millis(1000)
                } else {
                    false
                };

                self.last_esc_time = Some(now);

                if should_cancel {
                    return Ok(InputAction::Cancel);
                }
                // Single ESC - wait for second press
                return Ok(InputAction::None);
            } else {
                return Ok(InputAction::Exit);
            }
        }

        // Handle scrolling (allowed even when busy)
        match key.code {
            KeyCode::PageUp => return Ok(InputAction::PageUp),
            KeyCode::PageDown => return Ok(InputAction::PageDown),
            KeyCode::Up => return Ok(InputAction::ScrollUp),
            KeyCode::Down => return Ok(InputAction::ScrollDown),
            _ => {}
        }

        // Don't accept other input while busy
        if is_busy {
            return Ok(InputAction::None);
        }

        // Handle command/character input
        match key.code {
            KeyCode::Enter => {
                let text = input_text.trim().to_string();
                if text.is_empty() {
                    Ok(InputAction::None)
                } else if text.starts_with('/') {
                    Ok(InputAction::HandleCommand(text))
                } else {
                    Ok(InputAction::Submit(text))
                }
            }
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
            {
                if c == '@' {
                    Ok(InputAction::StartAutocomplete)
                } else if has_autocomplete {
                    Ok(InputAction::UpdateAutocomplete(c))
                } else {
                    Ok(InputAction::None)
                }
            }
            _ => Ok(InputAction::None),
        }
    }

    /// Handle a mouse event and return true if it was consumed
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<bool, CliError> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                tracing::trace!("MouseDown at ({}, {})", mouse.column, mouse.row);
                Ok(true)
            }
            MouseEventKind::Drag(MouseButton::Left) => Ok(true),
            MouseEventKind::Up(MouseButton::Left) => Ok(true),
            MouseEventKind::ScrollUp => Ok(true),
            MouseEventKind::ScrollDown => Ok(true),
            _ => Ok(false),
        }
    }

    /// Check if the current key event is a copy/paste shortcut
    /// Returns true for Ctrl+C/V on Linux/Windows, Cmd+C/V on macOS
    fn is_copy_paste_modifier(&self, key: &KeyEvent, char: char) -> bool {
        #[cfg(target_os = "macos")]
        {
            let has_super = key.modifiers.contains(KeyModifiers::SUPER);
            let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            key.code == KeyCode::Char(char) && (has_super || has_ctrl)
        }
        #[cfg(not(target_os = "macos"))]
        {
            key.code == KeyCode::Char(char) && key.modifiers.contains(KeyModifiers::CONTROL)
        }
    }

    /// Tick cursor blink animation
    pub fn tick_cursor_blink(&mut self) {
        if self.cursor_blink_timer.elapsed().as_millis() > 500 {
            self.cursor_blink_state = !self.cursor_blink_state;
            self.cursor_blink_timer = Instant::now();
        }
    }

    /// Get current cursor blink state
    pub fn cursor_blink_state(&self) -> bool {
        self.cursor_blink_state
    }

    /// Reset last ESC time (call when ESC is consumed)
    pub fn reset_esc_time(&mut self) {
        self.last_esc_time = None;
    }

    /// Get last ESC time
    pub fn last_esc_time(&self) -> Option<Instant> {
        self.last_esc_time
    }

    /// Set last ESC time
    pub fn set_last_esc_time(&mut self, time: Instant) {
        self.last_esc_time = Some(time);
    }

    /// Enforce paste size limit
    pub fn truncate_paste<'a>(&self, text: &'a str) -> (&'a str, bool) {
        if text.len() > MAX_PASTE_SIZE {
            // Find valid UTF-8 boundary
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

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_handler_creation() {
        let handler = InputHandler::new();
        assert!(handler.cursor_blink_state());
    }

    #[test]
    fn test_input_handler_default() {
        let handler = InputHandler::default();
        assert!(handler.cursor_blink_state());
    }

    #[test]
    fn test_truncate_paste() {
        let handler = InputHandler::new();

        // Small paste
        let (text, truncated) = handler.truncate_paste("hello");
        assert_eq!(text, "hello");
        assert!(!truncated);

        // Large paste
        let large_text = "x".repeat(200 * 1024);
        let (text, truncated) = handler.truncate_paste(&large_text);
        assert!(truncated);
        assert!(text.len() <= MAX_PASTE_SIZE);
    }

    #[test]
    fn test_cursor_blink() {
        let mut handler = InputHandler::new();
        let initial_state = handler.cursor_blink_state();

        // Blink should not change immediately
        handler.tick_cursor_blink();
        assert_eq!(handler.cursor_blink_state(), initial_state);
    }
}
