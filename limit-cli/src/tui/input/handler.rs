//! Input event handler for TUI
//!
//! Handles keyboard events, mouse events, and delegates to appropriate handlers.

use crate::error::CliError;
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use std::time::Instant;

/// Actions that can result from input handling
#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    None,
    Submit(String),
    Exit,
    Cancel,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    StartAutocomplete,
    UpdateAutocomplete(char),
    AutocompleteUp,
    AutocompleteDown,
    AutocompleteAccept,
    AutocompleteCancel,
    CopySelection,
    Paste,
    ShowHelp,
    ClearChat,
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
        _cursor_pos: usize,
        is_busy: bool,
        has_autocomplete: bool,
    ) -> Result<InputAction, CliError> {
        tracing::trace!(
            "handle_key: code={:?} mod={:?} kind={:?}",
            key.code,
            key.modifiers,
            key.kind
        );

        if key.kind != KeyEventKind::Press {
            return Ok(InputAction::None);
        }

        // Handle copy/paste shortcuts
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

        // Handle ESC
        if key.code == KeyCode::Esc {
            return self.handle_esc(is_busy, has_autocomplete);
        }

        // Handle scrolling
        match key.code {
            KeyCode::PageUp => return Ok(InputAction::PageUp),
            KeyCode::PageDown => return Ok(InputAction::PageDown),
            KeyCode::Up => return Ok(InputAction::ScrollUp),
            KeyCode::Down => return Ok(InputAction::ScrollDown),
            _ => {}
        }

        if is_busy {
            return Ok(InputAction::None);
        }

        // Handle command/character input
        match key.code {
            KeyCode::Enter => {
                let text = input_text.trim();
                if text.is_empty() {
                    Ok(InputAction::None)
                } else if text.starts_with('/') {
                    Ok(InputAction::HandleCommand(text.to_string()))
                } else {
                    Ok(InputAction::Submit(text.to_string()))
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

    /// Handle ESC key (double-ESC to cancel when busy)
    fn handle_esc(
        &mut self,
        is_busy: bool,
        has_autocomplete: bool,
    ) -> Result<InputAction, CliError> {
        if has_autocomplete {
            return Ok(InputAction::AutocompleteCancel);
        }

        if is_busy {
            let now = Instant::now();
            let should_cancel = self
                .last_esc_time
                .map(|last| now.duration_since(last) < std::time::Duration::from_millis(1000))
                .unwrap_or(false);

            self.last_esc_time = Some(now);

            return Ok(if should_cancel {
                InputAction::Cancel
            } else {
                InputAction::None
            });
        }

        Ok(InputAction::Exit)
    }

    /// Handle a mouse event
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<bool, CliError> {
        Ok(matches!(
            mouse.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
                | MouseEventKind::ScrollUp
                | MouseEventKind::ScrollDown
        ))
    }

    /// Check if the current key event is a copy/paste shortcut
    #[inline]
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
    #[inline]
    pub fn tick_cursor_blink(&mut self) {
        if self.cursor_blink_timer.elapsed().as_millis() > 500 {
            self.cursor_blink_state = !self.cursor_blink_state;
            self.cursor_blink_timer = Instant::now();
        }
    }

    /// Get current cursor blink state
    #[inline]
    pub fn cursor_blink_state(&self) -> bool {
        self.cursor_blink_state
    }

    /// Reset last ESC time
    #[inline]
    pub fn reset_esc_time(&mut self) {
        self.last_esc_time = None;
    }

    /// Get last ESC time
    #[inline]
    pub fn last_esc_time(&self) -> Option<Instant> {
        self.last_esc_time
    }

    /// Set last ESC time
    #[inline]
    pub fn set_last_esc_time(&mut self, time: Instant) {
        self.last_esc_time = Some(time);
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
    fn test_cursor_blink() {
        let mut handler = InputHandler::new();
        let initial_state = handler.cursor_blink_state();

        handler.tick_cursor_blink();
        assert_eq!(handler.cursor_blink_state(), initial_state);
    }
}
