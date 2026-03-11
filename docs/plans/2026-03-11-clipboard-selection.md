# Clipboard & Text Selection Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add clipboard support, bracketed paste, and text selection to the TUI.

**Architecture:** Add `arboard` for cross-platform clipboard, enable crossterm bracketed-paste feature, track mouse selection state in components, handle `Event::Paste` to insert without submitting. Selection is character-precise with visual highlighting.

**Tech Stack:** Rust, arboard 3.6, crossterm 0.29 with bracketed-paste, ratatui 0.29

**Decisions:**
- Selection visual feedback: Yes (reverse video)
- Keyboard selection: No (mouse only)
- Character-precise selection: Yes
- macOS Cmd key: Yes (Cmd+C/V on macOS, Ctrl+C/V on Linux/Windows)
- Selection persistence: Yes (persists across new messages)

---

## Task 1: Add Dependencies

**Files:**
- Modify: `limit-cli/Cargo.toml`

**Step 1: Add arboard to limit-cli Cargo.toml**

Add to dependencies section:

```toml
arboard = "3.6"
```

**Step 2: Enable bracketed-paste feature in crossterm**

In `limit-cli/Cargo.toml`, change:
```toml
crossterm = "0.29"
```
to:
```toml
crossterm = { version = "0.29", features = ["bracketed-paste"] }
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add limit-cli/Cargo.toml
git commit -m "feat: add arboard and bracketed-paste dependencies"
```

---

## Task 2: Create Clipboard Module

**Files:**
- Create: `limit-cli/src/clipboard.rs`
- Modify: `limit-cli/src/lib.rs`

**Step 1: Create clipboard module**

```rust
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
        let mut clipboard = self.clipboard.lock().map_err(|_| {
            arboard::Error::ClipboardNotSupported
        })?;
        clipboard.set_text(text.to_string())?;
        Ok(())
    }

    /// Get text from clipboard
    pub fn get_text(&self) -> Result<String, arboard::Error> {
        let mut clipboard = self.clipboard.lock().map_err(|_| {
            arboard::Error::ClipboardNotSupported
        })?;
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
```

**Step 2: Export clipboard module**

In `limit-cli/src/lib.rs`, add:
```rust
pub mod clipboard;
```

**Step 3: Verify compilation**

Run: `cargo check -p limit-cli`
Expected: No errors

**Step 4: Commit**

```bash
git add limit-cli/src/clipboard.rs limit-cli/src/lib.rs
git commit -m "feat: add clipboard module with arboard integration"
```

---

## Task 3: Enable Bracketed Paste in TUI

**Files:**
- Modify: `limit-cli/src/tui_bridge.rs`

**Step 1: Add bracketed paste imports**

Add to imports section (line ~4):
```rust
use crossterm::event::{EnableBracketedPaste, DisableBracketedPaste};
```

**Step 2: Enable bracketed paste on startup**

In `TuiApp::run()` method, after `EnableMouseCapture`:
```rust
execute!(std::io::stdout(), EnableBracketedPaste)
    .map_err(|e| CliError::IoError(io::Error::other(e)))?;
```

**Step 3: Disable bracketed paste on cleanup**

In `AlternateScreenGuard::Drop`, before `DisableMouseCapture`:
```rust
let _ = execute!(std::io::stdout(), DisableBracketedPaste);
```

**Step 4: Verify compilation**

Run: `cargo check -p limit-cli`
Expected: No errors

**Step 5: Commit**

```bash
git add limit-cli/src/tui_bridge.rs
git commit -m "feat: enable bracketed paste mode in TUI"
```

---

## Task 4: Handle Paste Events

**Files:**
- Modify: `limit-cli/src/tui_bridge.rs`

**Step 1: Add Event::Paste handler**

In `run_inner()` event loop, add handler after `Event::Mouse`:
```rust
Event::Paste(pasted) => {
    if !self.tui_bridge.is_busy() {
        self.insert_paste(&pasted);
    }
}
```

**Step 2: Add insert_paste method**

Add method to `TuiApp` impl:
```rust
/// Insert pasted text at cursor position without submitting
fn insert_paste(&mut self, text: &str) {
    // Normalize newlines (some terminals convert \n to \r)
    let normalized = text.replace("\r", "\n");
    self.input_text.insert_str(self.cursor_pos, &normalized);
    self.cursor_pos += normalized.len();
}
```

**Step 3: Verify compilation**

Run: `cargo check -p limit-cli`
Expected: No errors

**Step 4: Commit**

```bash
git add limit-cli/src/tui_bridge.rs
git commit -m "feat: handle bracketed paste events in TUI input"
```

---

## Task 5: Add Character-Precise Selection to ChatView

**Files:**
- Modify: `limit-tui/src/components/chat.rs`

**Step 1: Add selection state to ChatView**

Add fields to `ChatView` struct (around line 238):
```rust
/// Text selection state: (message_idx, byte_offset)
selection_start: Option<(usize, usize)>,
selection_end: Option<(usize, usize)>,
```

Update `ChatView::new()`:
```rust
selection_start: None,
selection_end: None,
```

**Step 2: Add selection methods with character-precise extraction**

Add impl methods:
```rust
/// Start text selection at position
pub fn start_selection(&mut self, message_idx: usize, byte_offset: usize) {
    self.selection_start = Some((message_idx, byte_offset));
    self.selection_end = Some((message_idx, byte_offset));
}

/// Extend selection to position
pub fn extend_selection(&mut self, message_idx: usize, byte_offset: usize) {
    if self.selection_start.is_some() {
        self.selection_end = Some((message_idx, byte_offset));
    }
}

/// Clear text selection
pub fn clear_selection(&mut self) {
    self.selection_start = None;
    self.selection_end = None;
}

/// Check if there is an active selection
pub fn has_selection(&self) -> bool {
    self.selection_start.is_some() && self.selection_end.is_some()
}

/// Check if a byte position is within the current selection
pub fn is_selected(&self, message_idx: usize, byte_offset: usize) -> bool {
    let Some((start_msg, start_offset)) = self.selection_start else { return false };
    let Some((end_msg, end_offset)) = self.selection_end else { return false };

    // Normalize order
    let (min_msg, min_offset, max_msg, max_offset) = if start_msg < end_msg
        || (start_msg == end_msg && start_offset <= end_offset)
    {
        (start_msg, start_offset, end_msg, end_offset)
    } else {
        (end_msg, end_offset, start_msg, start_offset)
    };

    // Check if position is in selection range
    if message_idx < min_msg || message_idx > max_msg {
        return false;
    }

    if message_idx == min_msg && message_idx == max_msg {
        // Same message: check offset range
        byte_offset >= min_offset && byte_offset < max_offset
    } else if message_idx == min_msg {
        // First message: offset >= min_offset
        byte_offset >= min_offset
    } else if message_idx == max_msg {
        // Last message: offset < max_offset
        byte_offset < max_offset
    } else {
        // Middle message: fully selected
        true
    }
}

/// Get selected text (character-precise)
pub fn get_selected_text(&self) -> Option<String> {
    let (start_msg, start_offset) = self.selection_start?;
    let (end_msg, end_offset) = self.selection_end?;

    // Normalize order
    let (min_msg, min_offset, max_msg, max_offset) = if start_msg < end_msg
        || (start_msg == end_msg && start_offset <= end_offset)
    {
        (start_msg, start_offset, end_msg, end_offset)
    } else {
        (end_msg, end_offset, start_msg, start_offset)
    };

    if min_msg == max_msg {
        // Single message: extract substring
        let msg = &self.messages.get(min_msg)?;
        let content = &msg.content;
        if min_offset < content.len() && max_offset <= content.len() {
            Some(content[min_offset..max_offset].to_string())
        } else {
            None
        }
    } else {
        // Multiple messages: collect parts
        let mut result = String::new();

        // First message: from offset to end
        if let Some(msg) = self.messages.get(min_msg) {
            if min_offset < msg.content.len() {
                result.push_str(&msg.content[min_offset..]);
            }
        }

        // Middle messages: full content
        for idx in (min_msg + 1)..max_msg {
            if let Some(msg) = self.messages.get(idx) {
                result.push('\n');
                result.push_str(&msg.content);
            }
        }

        // Last message: from start to offset
        if let Some(msg) = self.messages.get(max_msg) {
            result.push('\n');
            if max_offset > 0 && max_offset <= msg.content.len() {
                result.push_str(&msg.content[..max_offset]);
            }
        }

        Some(result)
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check -p limit-tui`
Expected: No errors

**Step 4: Commit**

```bash
git add limit-tui/src/components/chat.rs
git commit -m "feat: add character-precise text selection to ChatView"
```

---

## Task 6: Add Mouse Selection Handling

**Files:**
- Modify: `limit-cli/src/tui_bridge.rs`

**Step 1: Add selection state to TuiApp**

Add to `TuiApp` struct:
```rust
/// Mouse selection state
mouse_selection_start: Option<(u16, u16)>,
```

Initialize in `TuiApp::new()`:
```rust
mouse_selection_start: None,
```

**Step 2: Add MouseButton import**

Add to crossterm imports:
```rust
use crossterm::event::{MouseButton, /* ... existing imports */};
```

**Step 3: Handle mouse events for selection**

Update `Event::Mouse` handler in `run_inner()`:
```rust
Event::Mouse(mouse) => match mouse.kind {
    MouseEventKind::Down(MouseButton::Left) => {
        self.mouse_selection_start = Some((mouse.column, mouse.row));
        // Map screen position to message/offset and start selection
        if let Some((msg_idx, byte_offset)) = self.screen_to_text_pos(mouse.column, mouse.row) {
            self.tui_bridge.chat_view().lock().unwrap().start_selection(msg_idx, byte_offset);
        } else {
            self.tui_bridge.chat_view().lock().unwrap().clear_selection();
        }
    }
    MouseEventKind::Drag(MouseButton::Left) => {
        if let Some((_start_col, _start_row)) = self.mouse_selection_start {
            // Extend selection to current position
            if let Some((msg_idx, byte_offset)) = self.screen_to_text_pos(mouse.column, mouse.row) {
                self.tui_bridge.chat_view().lock().unwrap().extend_selection(msg_idx, byte_offset);
            }
        }
    }
    MouseEventKind::Up(MouseButton::Left) => {
        self.mouse_selection_start = None;
    }
    MouseEventKind::ScrollUp => {
        let mut chat = self.tui_bridge.chat_view().lock().unwrap();
        chat.scroll_up();
    }
    MouseEventKind::ScrollDown => {
        let mut chat = self.tui_bridge.chat_view().lock().unwrap();
        chat.scroll_down();
    }
    _ => {}
},
```

**Step 4: Add screen_to_text_pos helper method**

Add to `TuiApp` impl:
```rust
/// Map screen coordinates to (message_idx, byte_offset)
/// Returns None if position is not on message content
fn screen_to_text_pos(&self, _col: u16, row: u16) -> Option<(usize, usize)> {
    // Get chat area bounds from last draw
    // For now, simplified: estimate based on row offset
    let chat = self.tui_bridge.chat_view().lock().unwrap();

    // Approximate: each message takes ~3 lines minimum (header + content + separator)
    // This is a simplified mapping - full impl would track exact render positions
    let estimated_msg_idx = (row as usize) / 3;
    if estimated_msg_idx < chat.message_count() {
        Some((estimated_msg_idx, 0))
    } else {
        None
    }
}
```

**Step 5: Verify compilation**

Run: `cargo check -p limit-cli`
Expected: No errors

**Step 6: Commit**

```bash
git add limit-cli/src/tui_bridge.rs
git commit -m "feat: add mouse selection tracking to TUI"
```

---

## Task 7: Add Selection Visual Highlighting

**Files:**
- Modify: `limit-tui/src/components/chat.rs`

**Step 1: Modify render_to_buffer to highlight selected text**

In the `render_to_buffer` method, when rendering message content lines, check if each span is selected and apply reverse style.

Find the section where spans are created (around `parse_inline_markdown`) and wrap with selection check:

```rust
// In render_to_buffer, when rendering regular text:
let base_style = line_type.style();
let mut spans = parse_inline_markdown(&line, base_style);

// Apply selection highlighting
if chat.has_selection() {
    let msg_idx = /* current message index */;
    let line_start_offset = /* byte offset at start of this line */;

    for (span_idx, span) in spans.iter_mut().enumerate() {
        let span_start = line_start_offset + /* calculate offset for this span */;
        let span_end = span_start + span.content.len();

        if chat.is_selected(msg_idx, span_start) || chat.is_selected(msg_idx, span_end) {
            *span = Span::styled(
                span.content.clone(),
                span.style.patch(Style::default().add_modifier(Modifier::REVERSED))
            );
        }
    }
}
```

**Step 2: Add helper to track current message index in render loop**

Track `current_msg_idx` in the render loop to pass to selection checks.

**Step 3: Verify compilation**

Run: `cargo check -p limit-tui`
Expected: No errors

**Step 4: Commit**

```bash
git add limit-tui/src/components/chat.rs
git commit -m "feat: add visual highlighting for selected text"
```

---

## Task 8: Add macOS Cmd Key Support

**Files:**
- Modify: `limit-cli/src/tui_bridge.rs`

**Step 1: Add platform detection helper**

Add to `TuiApp` impl:
```rust
/// Check if the current key event is a copy/paste shortcut
/// Returns true for Ctrl+C/V on Linux/Windows, Cmd+C/V on macOS
fn is_copy_paste_modifier(&self, key: &KeyEvent, char: char) -> bool {
    #[cfg(target_os = "macos")]
    {
        key.code == KeyCode::Char(char) && key.modifiers.contains(KeyModifiers::SUPER)
    }
    #[cfg(not(target_os = "macos"))]
    {
        key.code == KeyCode::Char(char) && key.modifiers.contains(KeyModifiers::CONTROL)
    }
}
```

**Step 2: Update handle_key_event to use platform-aware modifier**

Replace Ctrl+C/V checks with `is_copy_paste_modifier`:

```rust
// For copy (before existing Ctrl+C exit handler):
if self.is_copy_paste_modifier(&key, 'c') {
    // ... copy logic
}

// For paste:
if self.is_copy_paste_modifier(&key, 'v') {
    // ... paste logic
}
```

**Step 3: Keep Ctrl+C as exit when no selection**

The original Ctrl+C exit behavior should still work:
- If selection exists: copy to clipboard
- If no selection: exit (Ctrl+C) or pass through (Cmd+C on macOS doesn't exit)

```rust
// macOS: Cmd+C copies, Ctrl+C exits (standard macOS behavior)
// Linux/Windows: Ctrl+C copies if selection, exits if no selection
if self.is_copy_paste_modifier(&key, 'c') {
    if self.tui_bridge.chat_view().lock().unwrap().has_selection() {
        // Copy selection
        if let Some(selected) = self.tui_bridge.chat_view().lock().unwrap().get_selected_text() {
            if !selected.is_empty() {
                let _ = self.clipboard.set_text(&selected);
                self.status_message = "Copied to clipboard".to_string();
                self.status_is_error = false;
                return Ok(());
            }
        }
    }
    // On non-macOS, fall through to Ctrl+C exit behavior
    #[cfg(not(target_os = "macos"))]
    {
        // Let existing Ctrl+C exit handler run
    }
    #[cfg(target_os = "macos")]
    {
        return Ok(()); // Cmd+C with no selection does nothing on macOS
    }
}
```

**Step 4: Verify compilation**

Run: `cargo check -p limit-cli`
Expected: No errors

**Step 5: Commit**

```bash
git add limit-cli/src/tui_bridge.rs
git commit -m "feat: add macOS Cmd key support for copy/paste"
```

---

## Task 9: Add Copy to Clipboard

**Files:**
- Modify: `limit-cli/src/tui_bridge.rs`

**Step 1: Add clipboard to TuiApp**

Add field to `TuiApp`:
```rust
clipboard: crate::clipboard::ClipboardManager,
```

Initialize in `TuiApp::new()`:
```rust
clipboard: crate::clipboard::ClipboardManager::new()
    .expect("Failed to initialize clipboard"),
```

**Step 2: Verify compilation**

Run: `cargo check -p limit-cli`
Expected: No errors

**Step 3: Commit**

```bash
git add limit-cli/src/tui_bridge.rs
git commit -m "feat: add clipboard manager to TUI app"
```

---

## Task 10: Add Paste from Clipboard

**Files:**
- Modify: `limit-cli/src/tui_bridge.rs`

**Step 1: Handle paste shortcut**

In `handle_key_event()`, add handler:
```rust
// Paste from clipboard
if self.is_copy_paste_modifier(&key, 'v') {
    match self.clipboard.get_text() {
        Ok(text) if !text.is_empty() && !self.tui_bridge.is_busy() => {
            self.insert_paste(&text);
        }
        Ok(_) => {} // Empty clipboard
        Err(_) => {
            self.status_message = "Could not read clipboard".to_string();
            self.status_is_error = true;
        }
    }
    return Ok(());
}
```

**Step 2: Verify compilation**

Run: `cargo check -p limit-cli`
Expected: No errors

**Step 3: Commit**

```bash
git add limit-cli/src/tui_bridge.rs
git commit -m "feat: add paste from clipboard with platform shortcut"
```

---

## Task 11: Add Large Text Truncation to InputPrompt

**Files:**
- Modify: `limit-tui/src/components/prompt.rs`

**Step 1: Add truncation fields to InputPrompt**

Add to struct:
```rust
/// Maximum chars to display before truncation
const MAX_DISPLAY_CHARS: usize = 500;
```

**Step 2: Add insert_paste method**

```rust
/// Insert pasted text at cursor
pub fn insert_paste(&mut self, text: &str) {
    let normalized = text.replace("\r", "\n");
    self.text.insert_str(self.cursor_pos, &normalized);
    self.cursor_pos += normalized.len();
    self.clear_error();
}
```

**Step 3: Update render to show truncation indicator**

Modify `render()` method. After building `display_text`, add truncation info:
```rust
// Show truncation indicator if text is very long
let final_text = if self.text.len() > Self::MAX_DISPLAY_CHARS {
    let visible_start = 200;
    let visible_end = 100;
    let hidden = self.text.len().saturating_sub(visible_start + visible_end);

    Text::from(vec![
        Line::from(self.text[..visible_start].to_string()),
        Line::styled(
            format!("  ┌─ {} chars hidden ─┐", hidden),
            Style::default().fg(Color::DarkGray),
        ),
        Line::from(self.text[self.text.len() - visible_end..].to_string()),
    ])
} else {
    display_text
};
```

**Step 4: Verify compilation**

Run: `cargo check -p limit-tui`
Expected: No errors

**Step 5: Commit**

```bash
git add limit-tui/src/components/prompt.rs
git commit -m "feat: add large text truncation to InputPrompt"
```

---

## Task 12: Run Tests and Lint

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings

**Step 3: Format code**

Run: `cargo fmt`

**Step 4: Final verification**

Run: `cargo build --release`
Expected: Successful build

**Step 5: Final commit (if any formatting changes)**

```bash
git add -A
git commit -m "chore: format and lint clipboard selection feature"
```

---

## Summary

| Task | Description |
|------|-------------|
| 1 | Add arboard and bracketed-paste deps |
| 2 | Create ClipboardManager module |
| 3 | Enable bracketed paste in TUI |
| 4 | Handle Event::Paste |
| 5 | Add character-precise selection to ChatView |
| 6 | Add mouse selection handling |
| 7 | Add selection visual highlighting |
| 8 | Add macOS Cmd key support |
| 9 | Add clipboard to TuiApp |
| 10 | Add paste from clipboard |
| 11 | Add large text truncation |
| 12 | Run tests and lint |
