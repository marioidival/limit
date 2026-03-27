//! TUI Application module
//!
//! Contains the main TUI application loop (`TuiApp`)

use crate::clipboard::ClipboardManager;
use crate::error::CliError;
use crate::tui::autocomplete::FileAutocompleteManager;
use crate::tui::bridge::TuiBridge;
use crate::tui::input::{InputEditor, InputHandler};
use crate::tui::ui::UiRenderer;
use crate::tui::TuiState;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use limit_tui::components::Message;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct TuiApp {
    tui_bridge: TuiBridge,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    running: bool,
    /// Input text editor
    input_editor: InputEditor,
    /// History file path
    history_path: std::path::PathBuf,
    status_message: String,
    status_is_error: bool,
    /// Mouse selection state
    mouse_selection_start: Option<(u16, u16)>,
    /// Clipboard manager (shared with CommandContext)
    clipboard: Option<Arc<Mutex<ClipboardManager>>>,
    /// File autocomplete manager
    autocomplete_manager: FileAutocompleteManager,
    /// Cancellation token for current LLM operation
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
    /// Input handler for keyboard/mouse events
    input_handler: InputHandler,
    /// Command registry for handling /commands
    command_registry: crate::tui::commands::CommandRegistry,
    /// Pending image attachments (from clipboard paste)
    pending_images: Vec<std::path::PathBuf>,
    /// Input queue for managing messages during async operations
    input_queue: crate::tui::input_queue::InputQueue,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new(tui_bridge: TuiBridge) -> Result<Self, CliError> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal =
            Terminal::new(backend).map_err(|e| CliError::IoError(io::Error::other(e)))?;

        let session_id = tui_bridge.session_id();
        tracing::info!("TUI started with session: {}", session_id);

        // Initialize history path (~/.limit/input_history.bin)
        let home_dir = dirs::home_dir()
            .ok_or_else(|| CliError::ConfigError("Failed to get home directory".to_string()))?;
        let limit_dir = home_dir.join(".limit");
        let history_path = limit_dir.join("input_history.bin");

        // Load input editor with history
        let input_editor = InputEditor::with_history(&history_path)
            .map_err(|e| CliError::ConfigError(format!("Failed to load input history: {}", e)))?;

        let clipboard = match ClipboardManager::new() {
            Ok(cb) => {
                tracing::info!("Clipboard initialized successfully");
                Some(Arc::new(Mutex::new(cb)))
            }
            Err(e) => {
                tracing::debug!("✗ Clipboard initialization failed: {}", e);
                tracing::warn!("Clipboard unavailable: {}", e);
                None
            }
        };

        // Initialize autocomplete manager with current directory
        let working_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let autocomplete_manager = FileAutocompleteManager::new(working_dir);

        // Initialize command registry
        let command_registry = crate::tui::commands::create_default_registry();

        Ok(Self {
            tui_bridge,
            terminal,
            running: true,
            input_editor,
            history_path,
            status_message: "Ready - Type a message and press Enter".to_string(),
            status_is_error: false,
            mouse_selection_start: None,
            clipboard,
            autocomplete_manager,
            cancellation_token: None,
            input_handler: InputHandler::new(),
            command_registry,
            pending_images: Vec::new(),
            input_queue: crate::tui::input_queue::InputQueue::new(),
        })
    }

    /// Run the TUI event loop
    pub fn run(&mut self) -> Result<(), CliError> {
        // Enter alternate screen - creates a clean buffer for TUI
        execute!(std::io::stdout(), EnterAlternateScreen)
            .map_err(|e| CliError::IoError(io::Error::other(e)))?;

        // Enable mouse capture for scroll support
        execute!(std::io::stdout(), EnableMouseCapture)
            .map_err(|e| CliError::IoError(io::Error::other(e)))?;

        // Enable bracketed paste for multi-line paste support
        execute!(std::io::stdout(), EnableBracketedPaste)
            .map_err(|e| CliError::IoError(io::Error::other(e)))?;

        crossterm::terminal::enable_raw_mode()
            .map_err(|e| CliError::IoError(io::Error::other(e)))?;

        // Guard to ensure cleanup on panic
        struct AlternateScreenGuard;
        impl Drop for AlternateScreenGuard {
            fn drop(&mut self) {
                let _ = crossterm::terminal::disable_raw_mode();
                let _ = execute!(std::io::stdout(), DisableBracketedPaste);
                let _ = execute!(std::io::stdout(), DisableMouseCapture);
                let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
            }
        }
        let _guard = AlternateScreenGuard;

        self.run_inner()
    }

    fn run_inner(&mut self) -> Result<(), CliError> {
        while self.running {
            // Process events from the agent
            self.tui_bridge.process_events()?;

            // Update spinner if in thinking state
            if matches!(self.tui_bridge.state(), TuiState::Thinking) {
                self.tui_bridge.tick_spinner();
            }

            // Update status based on state
            self.update_status();

            // Handle user input with poll timeout
            if event::poll(Duration::from_millis(100))
                .map_err(|e| CliError::IoError(io::Error::other(e)))?
            {
                match event::read().map_err(|e| CliError::IoError(io::Error::other(e)))? {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            self.handle_key_event(key)?;
                        }
                    }
                    Event::Mouse(mouse) => {
                        match mouse.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                self.mouse_selection_start = Some((mouse.column, mouse.row));
                                // Map screen position to message/offset and start selection
                                let chat = self.tui_bridge.chat_view().lock().unwrap();
                                if let Some((msg_idx, char_offset)) =
                                    chat.screen_to_text_pos(mouse.column, mouse.row)
                                {
                                    drop(chat);
                                    self.tui_bridge
                                        .chat_view()
                                        .lock()
                                        .unwrap()
                                        .start_selection(msg_idx, char_offset);
                                } else {
                                    drop(chat);
                                    self.tui_bridge
                                        .chat_view()
                                        .lock()
                                        .unwrap()
                                        .clear_selection();
                                }
                            }
                            MouseEventKind::Drag(MouseButton::Left) => {
                                if self.mouse_selection_start.is_some() {
                                    // Extend selection to current position
                                    let chat = self.tui_bridge.chat_view().lock().unwrap();
                                    if let Some((msg_idx, char_offset)) =
                                        chat.screen_to_text_pos(mouse.column, mouse.row)
                                    {
                                        drop(chat);
                                        self.tui_bridge
                                            .chat_view()
                                            .lock()
                                            .unwrap()
                                            .extend_selection(msg_idx, char_offset);
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
                        }
                    }
                    Event::Paste(pasted) => {
                        if !self.tui_bridge.is_busy() {
                            self.insert_paste(&pasted);
                        }
                    }
                    _ => {}
                }
            } else {
                // No key event - tick cursor blink
                self.tick_cursor_blink();
            }

            // Draw the TUI
            self.draw()?;
        }

        // Save session before exiting
        if let Err(e) = self.tui_bridge.save_session() {
            tracing::error!("Failed to save session: {}", e);
        }

        // Save input history before exiting
        if let Err(e) = self.input_editor.save_history(&self.history_path) {
            tracing::error!("Failed to save input history: {}", e);
        }

        Ok(())
    }

    fn update_status(&mut self) {
        // Don't override status if showing image attachment message
        if self.status_message.starts_with("Image attached") {
            return;
        }

        let session_id = self.tui_bridge.session_id();
        let has_activity = self
            .tui_bridge
            .activity_feed()
            .lock()
            .unwrap()
            .has_in_progress();

        match self.tui_bridge.state() {
            TuiState::Idle => {
                // Send queued messages when transitioning to idle
                self.maybe_send_next_queued_input();

                if has_activity {
                    // Show spinner when there are in-progress activities
                    let spinner = self.tui_bridge.spinner().lock().unwrap();
                    self.status_message = format!("{} Processing...", spinner.current_frame());
                } else if self.input_queue.has_queued_messages()
                    || self.input_queue.has_pending_steers()
                {
                    // Show queue status
                    let queued = self.input_queue.queued_count();
                    let steers = self.input_queue.steer_count();
                    let mut parts = vec![];
                    if queued > 0 {
                        parts.push(format!("{} queued", queued));
                    }
                    if steers > 0 {
                        parts.push(format!("{} pending", steers));
                    }
                    self.status_message =
                        format!("Ready | {} message(s) in queue", parts.join(", "));
                } else {
                    self.status_message = format!(
                        "Ready | Session: {}",
                        session_id.chars().take(8).collect::<String>()
                    );
                }
                self.status_is_error = false;
            }
            TuiState::Thinking => {
                let spinner = self.tui_bridge.spinner().lock().unwrap();
                if self.input_queue.has_queued_messages() || self.input_queue.has_pending_steers() {
                    let queued = self.input_queue.queued_count();
                    let steers = self.input_queue.steer_count();
                    let mut parts = vec![];
                    if queued > 0 {
                        parts.push(format!("{} queued", queued));
                    }
                    if steers > 0 {
                        parts.push(format!("{} pending", steers));
                    }
                    self.status_message = format!(
                        "{} Thinking... | {} message(s) waiting",
                        spinner.current_frame(),
                        parts.join(", ")
                    );
                } else {
                    self.status_message = format!("{} Thinking...", spinner.current_frame());
                }
                self.status_is_error = false;
            }
        }
    }

    /// Insert pasted text at cursor position without submitting
    fn insert_paste(&mut self, text: &str) {
        let truncated = self.input_editor.insert_paste(text);
        if truncated {
            self.status_message = "Paste truncated (too large)".to_string();
            self.status_is_error = true;
        }
    }

    /// Check if the current key event is a copy/paste shortcut
    /// Returns true for Ctrl+C/V on Linux/Windows, Cmd+C/V on macOS
    /// Note: Some macOS terminals report Cmd as CONTROL instead of SUPER
    fn is_copy_paste_modifier(&self, key: &KeyEvent, char: char) -> bool {
        #[cfg(target_os = "macos")]
        {
            // Accept both SUPER and CONTROL on macOS since terminal emulators vary
            let has_super = key.modifiers.contains(KeyModifiers::SUPER);
            let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            let result = key.code == KeyCode::Char(char) && (has_super || has_ctrl);
            tracing::trace!("is_copy_paste_modifier('{}') macOS: code={:?}, mod={:?}, super={}, ctrl={}, result={}",
                char, key.code, key.modifiers, has_super, has_ctrl, result);
            result
        }
        #[cfg(not(target_os = "macos"))]
        {
            let result =
                key.code == KeyCode::Char(char) && key.modifiers.contains(KeyModifiers::CONTROL);
            tracing::trace!(
                "is_copy_paste_modifier('{}') non-macOS: code={:?}, mod={:?}, ctrl={:?}, result={}",
                char,
                key.code,
                key.modifiers,
                KeyModifiers::CONTROL,
                result
            );
            result
        }
    }

    fn tick_cursor_blink(&mut self) {
        // Delegate to InputHandler
        self.input_handler.tick_cursor_blink();
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<(), CliError> {
        // Copy selection to clipboard (Ctrl/Cmd+C)
        if self.is_copy_paste_modifier(&key, 'c') {
            tracing::trace!("✓ Copy shortcut CONFIRMED - processing...");
            let mut chat = self.tui_bridge.chat_view().lock().unwrap();
            let has_selection = chat.has_selection();
            tracing::trace!("has_selection={}", has_selection);

            if has_selection {
                if let Some(selected) = chat.get_selected_text() {
                    tracing::trace!("Selected text length={}", selected.len());
                    if !selected.is_empty() {
                        if let Some(ref clipboard) = self.clipboard {
                            tracing::trace!("Attempting to copy to clipboard...");
                            match clipboard.lock().unwrap().set_text(&selected) {
                                Ok(()) => {
                                    tracing::trace!("✓ Clipboard copy successful");
                                    self.status_message = "Copied to clipboard".to_string();
                                    self.status_is_error = false;
                                }
                                Err(e) => {
                                    tracing::debug!("✗ Clipboard copy failed: {}", e);
                                    self.status_message = format!("Clipboard error: {}", e);
                                    self.status_is_error = true;
                                }
                            }
                        } else {
                            tracing::debug!("✗ Clipboard not available (None)");
                            self.status_message = "Clipboard not available".to_string();
                            self.status_is_error = true;
                        }
                    } else {
                        tracing::trace!("Selected text is empty");
                    }
                    chat.clear_selection();
                } else {
                    tracing::trace!("get_selected_text() returned None");
                }
                return Ok(());
            }

            // No selection - do nothing (Ctrl+C is only for copying text)
            tracing::trace!("Ctrl/Cmd+C with no selection - ignoring");
            return Ok(());
        }

        // Paste from clipboard (Ctrl/Cmd+V or Alt+V for image)
        // On macOS, Alt+V is handled separately below
        let is_paste_shortcut = {
            #[cfg(target_os = "macos")]
            {
                let has_mod = key.modifiers.contains(KeyModifiers::SUPER)
                    || key.modifiers.contains(KeyModifiers::CONTROL);
                let is_v = key.code == KeyCode::Char('v');
                is_v && has_mod
            }
            #[cfg(not(target_os = "macos"))]
            {
                self.is_copy_paste_modifier(&key, 'v')
            }
        };

        // Alt+V for image paste (macOS and Linux)
        if key.code == KeyCode::Char('v')
            && key.modifiers.contains(KeyModifiers::ALT)
            && !self.tui_bridge.is_busy()
        {
            tracing::trace!("Attempting image paste (Alt+V)...");

            // Check if current provider supports vision
            let model = self
                .tui_bridge
                .agent_bridge_arc()
                .lock()
                .unwrap()
                .model()
                .to_lowercase();
            let provider = self
                .tui_bridge
                .agent_bridge_arc()
                .lock()
                .unwrap()
                .provider_name()
                .to_lowercase();

            let supports_vision = {
                // OpenAI vision models
                if provider == "openai" || provider == "openai-compatible" {
                    model.contains("gpt-4o")
                        || model.contains("gpt-4-turbo")
                        || model.contains("gpt-4-vision")
                // Anthropic Claude 3+ all support vision
                } else if provider == "anthropic" || provider == "claude" {
                    model.contains("claude-3")
                // Google Gemini models
                } else if provider == "google" || provider == "gemini" {
                    model.contains("gemini")
                // z.ai and other unsupported providers
                } else {
                    false
                }
            };

            if !supports_vision {
                self.status_message = "Current provider/model does not support images. Use a vision-capable model like gpt-4o or claude-3.".to_string();
                self.status_is_error = true;
                return Ok(());
            }

            match crate::clipboard_paste::paste_image_to_temp_png() {
                Ok((_path, info)) => {
                    tracing::debug!(
                        "pasted image size={}x{} format={}",
                        info.width,
                        info.height,
                        info.encoded_format.label()
                    );
                    self.pending_images.push(_path);
                    self.status_message = format!(
                        "Image attached ({}x{}) - {} image(s) pending. Press Enter to send.",
                        info.width,
                        info.height,
                        self.pending_images.len()
                    );
                    self.status_is_error = false;
                }
                Err(err) => {
                    tracing::warn!("failed to paste image: {err}");
                    self.status_message = format!("Failed to paste image: {err}");
                    self.status_is_error = true;
                }
            }
            return Ok(());
        }

        // Regular text paste (Ctrl/Cmd+V)
        if is_paste_shortcut && !self.tui_bridge.is_busy() {
            let clipboard_result = if let Some(ref clipboard) = self.clipboard {
                tracing::trace!("Attempting to read from clipboard...");
                Some(clipboard.lock().unwrap().get_text())
            } else {
                None
            };

            match clipboard_result {
                Some(Ok(text)) if !text.is_empty() => {
                    tracing::trace!("Read {} chars from clipboard", text.len());
                    self.insert_paste(&text);
                }
                Some(Ok(_)) => {
                    tracing::trace!("Clipboard is empty");
                }
                Some(Err(e)) => {
                    tracing::debug!("✗ Failed to read clipboard: {}", e);
                    self.status_message = format!("Could not read clipboard: {}", e);
                    self.status_is_error = true;
                }
                None => {
                    tracing::debug!("✗ Clipboard not available (None)");
                    self.status_message = "Clipboard not available".to_string();
                    self.status_is_error = true;
                }
            }
            return Ok(());
        }

        // Handle autocomplete navigation FIRST (before general scrolling)
        let autocomplete_active = self.autocomplete_manager.is_active();
        tracing::trace!(
            "Key handling: autocomplete_active={}, is_busy={}, history_len={}",
            autocomplete_active,
            self.tui_bridge.is_busy(),
            self.input_editor.history().len()
        );

        if autocomplete_active {
            match key.code {
                KeyCode::Up => {
                    self.autocomplete_manager.navigate_up();
                    return Ok(());
                }
                KeyCode::Down => {
                    self.autocomplete_manager.navigate_down();
                    return Ok(());
                }
                KeyCode::Enter | KeyCode::Tab => {
                    self.accept_file_completion();
                    return Ok(());
                }
                KeyCode::Esc => {
                    self.autocomplete_manager.deactivate();
                    return Ok(());
                }
                _ => {}
            }
        }

        // Handle ESC for cancellation (must be before is_busy check)
        if key.code == KeyCode::Esc {
            // If autocomplete is active, cancel it
            if self.autocomplete_manager.is_active() {
                self.autocomplete_manager.deactivate();
            } else if self.tui_bridge.is_busy() {
                // Double-ESC to cancel current operation
                let now = Instant::now();
                let last_esc_time = self.input_handler.last_esc_time();
                let should_cancel = if let Some(last_esc) = last_esc_time {
                    now.duration_since(last_esc) < Duration::from_millis(1000)
                } else {
                    false
                };

                if should_cancel {
                    self.cancel_current_operation();
                } else {
                    // First ESC - show feedback
                    self.status_message = "Press ESC again to cancel".to_string();
                    self.status_is_error = false;
                    self.input_handler.set_last_esc_time(now);
                }
            } else {
                tracing::debug!("Esc pressed, exiting");
                self.running = false;
            }
            return Ok(());
        }

        // Allow scrolling even when agent is busy (PageUp/PageDown only)
        // Calculate actual viewport height dynamically
        let term_height = self.terminal.size().map(|s| s.height).unwrap_or(24);
        let viewport_height = term_height
            .saturating_sub(1) // status bar - input area (6 lines) - borders (~2)
            .saturating_sub(7); // status (1) + input (6) + top/bottom borders (2) = 9
        match key.code {
            KeyCode::PageUp => {
                let mut chat = self.tui_bridge.chat_view().lock().unwrap();
                chat.scroll_page_up(viewport_height);
                return Ok(());
            }
            KeyCode::PageDown => {
                let mut chat = self.tui_bridge.chat_view().lock().unwrap();
                chat.scroll_page_down(viewport_height);
                return Ok(());
            }
            KeyCode::Up => {
                // Use for history navigation
                tracing::debug!(
                    "Up arrow: navigating history up, history_len={}",
                    self.input_editor.history().len()
                );
                let navigated = self.input_editor.navigate_history_up();
                tracing::debug!(
                    "Up arrow: navigated={}, text='{}'",
                    navigated,
                    self.input_editor.text()
                );
                return Ok(());
            }
            KeyCode::Down => {
                // Use for history navigation
                tracing::debug!(
                    "Down arrow: navigating history down, is_navigating={}",
                    self.input_editor.is_navigating_history()
                );
                let navigated = self.input_editor.navigate_history_down();
                tracing::debug!(
                    "Down arrow: navigated={}, text='{}'",
                    navigated,
                    self.input_editor.text()
                );
                return Ok(());
            }
            _ => {}
        }

        // Don't accept input while agent is busy - queue messages instead
        if self.tui_bridge.is_busy() {
            // Allow character input and queue it
            match key.code {
                KeyCode::Char(c)
                    if key.modifiers == KeyModifiers::NONE
                        || key.modifiers == KeyModifiers::SHIFT =>
                {
                    // Insert character
                    self.input_editor.insert_char(c);
                    tracing::debug!("Agent busy, queuing character: {}", c);
                }
                KeyCode::Backspace => {
                    self.delete_char_before_cursor();
                }
                KeyCode::Delete => {
                    self.input_editor.delete_char_at();
                }
                KeyCode::Left => {
                    self.input_editor.move_left();
                }
                KeyCode::Right => {
                    self.input_editor.move_right();
                }
                KeyCode::Home => {
                    self.input_editor.move_to_start();
                }
                KeyCode::End => {
                    self.input_editor.move_to_end();
                }
                KeyCode::Enter => {
                    // Queue the message
                    let text = self.input_editor.take_and_add_to_history();
                    if !text.is_empty() {
                        self.input_queue.queue_message(text);
                        tracing::info!("Message queued while agent is busy");
                    }
                }
                KeyCode::Esc => {
                    // If we have pending steers, mark for immediate submit after interrupt
                    if self.input_queue.has_pending_steers() {
                        self.input_queue.set_submit_after_interrupt(true);
                        self.cancel_current_operation();
                        tracing::info!("Interrupting with pending steers to send immediately");
                    }
                }
                _ => {}
            }
            return Ok(());
        }

        // Handle backspace - try multiple detection methods
        if self.handle_backspace(&key) {
            tracing::debug!("Backspace handled, input: {:?}", self.input_editor.text());
            return Ok(());
        }

        match key.code {
            KeyCode::Delete => {
                if self.input_editor.delete_char_at() {
                    tracing::debug!("Delete: input now: {:?}", self.input_editor.text());
                }
            }
            KeyCode::Left => {
                tracing::debug!(
                    "KeyCode::Left: has_pasted_content={}",
                    self.input_editor.has_pasted_content()
                );
                self.input_editor.move_left();
            }
            KeyCode::Right => {
                tracing::debug!(
                    "KeyCode::Right: has_pasted_content={}",
                    self.input_editor.has_pasted_content()
                );
                self.input_editor.move_right();
            }
            KeyCode::Home => {
                self.input_editor.move_to_start();
            }
            KeyCode::End => {
                self.input_editor.move_to_end();
            }
            KeyCode::Enter => {
                // If autocomplete is active, it's already handled above
                self.handle_enter()?;
            }
            // Regular character input (including UTF-8)
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
            {
                // Check if we're triggering autocomplete with @
                if c == '@' {
                    // Insert @ character
                    self.input_editor.insert_char('@');

                    // Activate autocomplete
                    self.activate_file_autocomplete();
                } else if self.autocomplete_manager.is_active() {
                    // Update autocomplete and insert character
                    self.autocomplete_manager.append_char(c);
                    self.input_editor.insert_char(c);
                } else {
                    // Normal character insertion
                    self.input_editor.insert_char(c);
                }
            }
            _ => {
                // Ignore other keys
            }
        }

        Ok(())
    }

    /// Handle backspace with multiple detection methods
    fn handle_backspace(&mut self, key: &KeyEvent) -> bool {
        // Method 1: Standard Backspace keycode
        if key.code == KeyCode::Backspace {
            tracing::debug!("Backspace detected via KeyCode::Backspace");
            self.delete_char_before_cursor();
            return true;
        }

        // Method 2: Ctrl+H (common backspace mapping)
        if key.code == KeyCode::Char('h') && key.modifiers == KeyModifiers::CONTROL {
            tracing::debug!("Backspace detected via Ctrl+H");
            self.delete_char_before_cursor();
            return true;
        }

        // Method 3: Check for DEL (127) or BS (8) characters
        if let KeyCode::Char(c) = key.code {
            if c == '\x7f' || c == '\x08' {
                tracing::debug!("Backspace detected via char code: {}", c as u8);
                self.delete_char_before_cursor();
                return true;
            }
        }

        false
    }

    fn delete_char_before_cursor(&mut self) {
        tracing::debug!(
            "delete_char: cursor={}, len={}, input={:?}",
            self.input_editor.cursor(),
            self.input_editor.text().len(),
            self.input_editor.text()
        );

        // If autocomplete is active, handle backspace specially
        if self.autocomplete_manager.is_active() {
            let should_close = self.autocomplete_manager.backspace();

            if should_close
                && self.input_editor.cursor() > 0
                && self.input_editor.char_before_cursor() == Some('@')
            {
                self.input_editor.delete_char_before();
                self.autocomplete_manager.deactivate();
                return;
            }
        }

        // Normal backspace
        self.input_editor.delete_char_before();
    }

    /// Activate file autocomplete
    fn activate_file_autocomplete(&mut self) {
        let trigger_pos = self.input_editor.cursor() - 1; // Position of @
        self.autocomplete_manager.activate(trigger_pos);

        tracing::info!(
            "🔍 ACTIVATED AUTOCOMPLETE: trigger_pos={}, cursor={}, text='{}'",
            trigger_pos,
            self.input_editor.cursor(),
            self.input_editor.text()
        );
    }

    /// Accept selected file completion
    fn accept_file_completion(&mut self) {
        // Get the selected match WITHOUT using accept_completion()
        // We do this manually to avoid the trailing space issue
        let selected = self.autocomplete_manager.selected_match().cloned();

        if let Some(selected) = selected {
            let trigger_pos = self.autocomplete_manager.trigger_pos().unwrap_or(0);
            let current_cursor = self.input_editor.cursor();

            tracing::info!(
                "🎯 ACCEPT: trigger_pos={}, cursor={}, path='{}'",
                trigger_pos,
                current_cursor,
                selected.path
            );

            // Calculate what to remove: from @+1 to current cursor
            let remove_start = trigger_pos + 1;
            let remove_end = current_cursor;

            tracing::info!(
                "🎯 REMOVE RANGE: {}..{} = '{}'",
                remove_start,
                remove_end,
                &self.input_editor.text()
                    [remove_start..remove_end.min(self.input_editor.text().len())]
            );

            // Use replace_range to atomically replace the query with the completion
            // Use selected.path WITHOUT trailing space
            if remove_end > remove_start {
                self.input_editor
                    .replace_range(remove_start, remove_end, &selected.path);
            } else {
                // Nothing to remove, just insert after @
                self.input_editor.set_cursor(remove_start);
                self.input_editor.insert_str(&selected.path);
            }

            // Add trailing space AFTER the completion
            self.input_editor.insert_char(' ');

            tracing::info!(
                "🎯 FINAL: '{}', cursor={}",
                self.input_editor.text(),
                self.input_editor.cursor()
            );
        }

        // Close autocomplete
        self.autocomplete_manager.deactivate();
    }

    fn handle_enter(&mut self) -> Result<(), CliError> {
        let text = self.input_editor.take_and_add_to_history();

        if text.is_empty() {
            return Ok(());
        }

        tracing::info!("Enter pressed with text: {:?}", text);

        // Check if it's a command (starts with /)
        if text.starts_with('/') {
            use crate::tui::commands::{CommandContext, CommandResult};

            let mut cmd_ctx = CommandContext::new(
                self.tui_bridge.chat_view().clone(),
                self.tui_bridge.session_manager(),
                self.tui_bridge.session_id(),
                self.tui_bridge.state_arc(),
                self.tui_bridge.messages(),
                self.tui_bridge.total_input_tokens_arc(),
                self.tui_bridge.total_output_tokens_arc(),
                self.clipboard.clone(),
                self.autocomplete_manager.base_path().to_path_buf(),
            );

            // Execute command via registry
            match self.command_registry.parse_and_execute(&text, &mut cmd_ctx) {
                Ok(Some(result)) => {
                    // Update session_id if changed by command
                    self.tui_bridge
                        .session_id_arc()
                        .lock()
                        .unwrap()
                        .clone_from(&cmd_ctx.session_id);

                    match result {
                        CommandResult::Exit => {
                            self.running = false;
                            return Ok(());
                        }
                        CommandResult::ClearChat => {
                            // Already handled by command
                            return Ok(());
                        }
                        CommandResult::NewSession | CommandResult::LoadSession(_) => {
                            // Session was changed, sync state
                            *self.tui_bridge.messages().lock().unwrap() =
                                cmd_ctx.messages.lock().unwrap().clone();
                            *self.tui_bridge.total_input_tokens_arc().lock().unwrap() =
                                *cmd_ctx.total_input_tokens.lock().unwrap();
                            *self.tui_bridge.total_output_tokens_arc().lock().unwrap() =
                                *cmd_ctx.total_output_tokens.lock().unwrap();
                            return Ok(());
                        }
                        CommandResult::Continue
                        | CommandResult::Message(_)
                        | CommandResult::Share(_) => {
                            return Ok(());
                        }
                    }
                }
                Ok(None) => {
                    // Not a command, fall through to LLM processing
                }
                Err(e) => {
                    tracing::error!("Command error: {}", e);
                    // Show error to user instead of crashing
                    self.tui_bridge
                        .chat_view()
                        .lock()
                        .unwrap()
                        .add_message(Message::system(format!("Error: {}", e)));
                    return Ok(());
                }
            }
        }

        // Handle bare commands (without /) for backwards compatibility
        let text_lower = text.to_lowercase();
        if text_lower == "exit" || text_lower == "quit" {
            tracing::info!("Exit command detected, exiting");
            self.running = false;
            return Ok(());
        }

        if text_lower == "clear" {
            tracing::info!("Clear command detected");
            self.tui_bridge.chat_view().lock().unwrap().clear();
            return Ok(());
        }

        if text_lower == "help" {
            tracing::info!("Help command detected");
            let help_msg = Message::system(
                "Available commands:\n\
                 /help  - Show this help message\n\
                 /clear - Clear chat history\n\
                 /exit  - Exit the application\n\
                 /quit  - Exit the application\n\
                 /session list  - List all sessions\n\
                 /session new   - Create a new session\n\
                 /session load  <id> - Load a session by ID\n\
                 /share         - Copy session to clipboard (markdown)\n\
                 /share md      - Export session as markdown file\n\
                 /share json    - Export session as JSON file\n\
                 \n\
                 Page Up/Down - Scroll chat history\n\
                 Up/Down (empty input) - Navigate input history"
                    .to_string(),
            );
            self.tui_bridge
                .chat_view()
                .lock()
                .unwrap()
                .add_message(help_msg);
            return Ok(());
        }

        // Add user message to chat immediately for visual feedback
        // Check if we have pending images
        let _content = if self.pending_images.is_empty() {
            limit_llm::MessageContent::text(text.clone())
        } else {
            // Build multimodal content with text and images
            let mut parts = vec![limit_llm::ContentPart::text(text.clone())];

            for image_path in self.pending_images.drain(..) {
                // Read image file and convert to base64
                match std::fs::read(&image_path) {
                    Ok(image_data) => {
                        // Detect image type from extension
                        let media_type = image_path
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| match e.to_lowercase().as_str() {
                                "png" => "image/png",
                                "jpg" | "jpeg" => "image/jpeg",
                                "gif" => "image/gif",
                                "webp" => "image/webp",
                                _ => "image/png",
                            })
                            .unwrap_or("image/png");

                        let base64_data = base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &image_data,
                        );

                        parts.push(limit_llm::ContentPart::image_base64(
                            media_type,
                            &base64_data,
                        ));

                        tracing::info!(
                            "Attached image: {} ({} bytes, {})",
                            image_path.display(),
                            image_data.len(),
                            media_type
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to read image {}: {}", image_path.display(), e);
                    }
                }
            }

            self.status_message = "Ready - Type a message and press Enter".to_string();
            limit_llm::MessageContent::parts(parts)
        };

        self.tui_bridge.add_user_message(text.clone());

        // Get new operation ID and ensure state is Idle
        let operation_id = self.tui_bridge.next_operation_id();
        tracing::debug!("handle_enter: new operation_id={}", operation_id);
        self.tui_bridge.set_state(TuiState::Idle);

        // Create cancellation token for this operation
        let cancel_token = tokio_util::sync::CancellationToken::new();
        self.cancellation_token = Some(cancel_token.clone());

        // Clone Arcs for the spawned thread
        let messages = self.tui_bridge.messages();
        let agent_bridge = self.tui_bridge.agent_bridge_arc();
        let session_manager = self.tui_bridge.session_manager();
        let session_id = self.tui_bridge.session_id();
        let total_input_tokens = self.tui_bridge.total_input_tokens_arc();
        let total_output_tokens = self.tui_bridge.total_output_tokens_arc();

        tracing::debug!("Spawning LLM processing thread");

        // Spawn a thread to process the message without blocking the UI
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // Create a new tokio runtime for this thread
                let rt = tokio::runtime::Runtime::new().unwrap();

                // Safe: we're in a dedicated thread, this won't cause issues
                #[allow(clippy::await_holding_lock)]
                rt.block_on(async {
                    // Check for cancellation BEFORE acquiring locks
                    if cancel_token.is_cancelled() {
                        tracing::debug!("Operation cancelled before acquiring locks");
                        return;
                    }

                    // Try to acquire locks with timeout to avoid blocking indefinitely
                    let messages_guard = {
                        let mut attempts = 0;
                        loop {
                            if cancel_token.is_cancelled() {
                                tracing::debug!(
                                    "Operation cancelled while waiting for messages lock"
                                );
                                return;
                            }
                            match messages.try_lock() {
                                Ok(guard) => break guard,
                                Err(std::sync::TryLockError::WouldBlock) => {
                                    attempts += 1;
                                    if attempts > 50 {
                                        tracing::error!("Timeout waiting for messages lock");
                                        return;
                                    }
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                }
                                Err(e) => {
                                    tracing::error!("Failed to lock messages: {}", e);
                                    return;
                                }
                            }
                        }
                    };

                    let mut messages_guard = messages_guard;

                    // Check cancellation again before acquiring bridge lock
                    if cancel_token.is_cancelled() {
                        tracing::debug!("Operation cancelled before acquiring bridge lock");
                        return;
                    }

                    let bridge_guard = {
                        let mut attempts = 0;
                        loop {
                            if cancel_token.is_cancelled() {
                                tracing::debug!(
                                    "Operation cancelled while waiting for bridge lock"
                                );
                                return;
                            }
                            match agent_bridge.try_lock() {
                                Ok(guard) => break guard,
                                Err(std::sync::TryLockError::WouldBlock) => {
                                    attempts += 1;
                                    if attempts > 50 {
                                        tracing::error!("Timeout waiting for bridge lock");
                                        return;
                                    }
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                }
                                Err(e) => {
                                    tracing::error!("Failed to lock agent_bridge: {}", e);
                                    return;
                                }
                            }
                        }
                    };

                    let mut bridge = bridge_guard;

                    // Set cancellation token and operation ID
                    bridge.set_cancellation_token(cancel_token.clone(), operation_id);

                    match bridge.process_message(&text, &mut messages_guard).await {
                        Ok(result) => {
                            {
                                let mut input = total_input_tokens.lock().unwrap();
                                let mut output = total_output_tokens.lock().unwrap();
                                *input += result.input_tokens;
                                *output += result.output_tokens;
                            }

                            let msgs = messages_guard.clone();
                            let input_tokens = *total_input_tokens.lock().unwrap();
                            let output_tokens = *total_output_tokens.lock().unwrap();

                            if let Err(e) = session_manager.lock().unwrap().save_session(
                                &session_id,
                                &msgs,
                                input_tokens,
                                output_tokens,
                            ) {
                                tracing::error!(
                                    "✗ Failed to auto-save session {}: {}",
                                    session_id,
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "✓ Session {} auto-saved ({} messages, {} in, {} out tokens)",
                                    session_id,
                                    msgs.len(),
                                    input_tokens,
                                    output_tokens
                                );
                            }
                        }
                        Err(e) => {
                            // Check if it was a cancellation
                            let error_msg = e.to_string();
                            if error_msg.contains("cancelled") {
                                tracing::info!("Request cancelled by user");
                            } else {
                                tracing::error!("LLM error: {}", e);
                            }
                        }
                    }

                    // Clear cancellation token
                    bridge.clear_cancellation_token();
                });
            }));

            if let Err(panic_payload) = result {
                let msg = panic_payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| panic_payload.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("unknown panic");
                tracing::error!("LLM thread panicked: {}", msg);
            }
        });

        Ok(())
    }

    /// Cancel current LLM operation
    fn cancel_current_operation(&mut self) {
        if let Some(ref token) = self.cancellation_token {
            token.cancel();
            tracing::debug!("Cancellation token triggered");

            // Increment operation ID to ignore subsequent events from old operation
            self.tui_bridge.next_operation_id();

            // Check if we should send pending steers immediately after interrupt
            let should_send_steers = self.input_queue.should_submit_after_interrupt();

            if should_send_steers {
                // Drain steers and send as new turn
                if let Some(merged) = self.input_queue.merge_all() {
                    tracing::info!(
                        "Sending pending steers immediately after interrupt: {}",
                        merged
                    );

                    // Add user message to chat
                    self.tui_bridge.add_user_message(merged.clone());

                    // TODO: Submit to agent - this would need to call handle_enter logic
                    // For now, just restore to input editor
                    self.input_editor.set_text(&merged);
                    self.status_message =
                        "Steers restored to input - press Enter to send".to_string();
                }
                self.input_queue.set_submit_after_interrupt(false);
            } else {
                // Force reset TUI state to Idle
                self.tui_bridge.set_state(TuiState::Idle);

                // Update UI state
                self.status_message = "Operation cancelled".to_string();
                self.status_is_error = false;

                // Clear activity feed
                self.tui_bridge
                    .activity_feed()
                    .lock()
                    .unwrap()
                    .complete_all();

                // Add cancellation message to chat
                let cancel_msg = Message::system("⚠ Operation cancelled by user".to_string());
                self.tui_bridge
                    .chat_view()
                    .lock()
                    .unwrap()
                    .add_message(cancel_msg);
            }
        }
        self.cancellation_token = None;
        // Reset ESC time via input_handler
        self.input_handler.reset_esc_time();
    }

    fn draw(&mut self) -> Result<(), CliError> {
        let chat_view = self.tui_bridge.chat_view().clone();
        let display_text = self.input_editor.display_text_combined();
        let cursor_pos = self.input_editor.cursor();
        let cursor_blink_state = self.input_handler.cursor_blink_state();
        let tui_bridge = &self.tui_bridge;
        let file_autocomplete = self.autocomplete_manager.to_legacy_state();

        // Build pending input preview from queue
        let pending_input_preview =
            if self.input_queue.has_queued_messages() || self.input_queue.has_pending_steers() {
                let mut preview = limit_tui::components::PendingInputPreview::new();
                preview.pending_steers = self.input_queue.steer_texts();
                preview.queued_messages = self.input_queue.queued_texts();
                Some(preview)
            } else {
                None
            };

        self.terminal
            .draw(|f| {
                UiRenderer::render(
                    f,
                    f.area(),
                    &chat_view,
                    &display_text,
                    cursor_pos,
                    &self.status_message,
                    self.status_is_error,
                    cursor_blink_state,
                    tui_bridge,
                    &file_autocomplete,
                    pending_input_preview.as_ref(),
                );
            })
            .map_err(|e| CliError::IoError(io::Error::other(e)))?;

        Ok(())
    }

    /// Send next queued input if available (called when transitioning to Idle)
    fn maybe_send_next_queued_input(&mut self) {
        if self.input_queue.is_autosend_suppressed() {
            return;
        }

        if self.tui_bridge.is_busy() {
            return;
        }

        // Pop the next queued message
        if let Some(msg) = self.input_queue.pop_queued() {
            tracing::info!("Sending queued message: {}", msg.text);

            // Set the text directly in the editor
            self.input_editor.clear();
            for ch in msg.text.chars() {
                self.input_editor.insert_char(ch);
            }

            // Add to history
            self.input_editor.history_mut().add(&msg.text);

            // Trigger the message submission
            // This will handle the LLM processing in a separate thread
            drop(self.handle_enter());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_bridge::AgentBridge;
    use crate::tui::bridge::TuiBridge;
    use std::io::IsTerminal;
    use tokio::sync::mpsc;

    /// Create a test config for AgentBridge
    fn create_test_config() -> limit_llm::Config {
        use limit_llm::{BrowserConfigSection, ProviderConfig};
        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                api_key: Some("test-key".to_string()),
                model: "claude-3-5-sonnet-20241022".to_string(),
                base_url: None,
                max_tokens: 4096,
                timeout: 60,
                max_iterations: 100,
                thinking_enabled: false,
                clear_thinking: true,
            },
        );
        limit_llm::Config {
            provider: "anthropic".to_string(),
            providers,
            browser: BrowserConfigSection::default(),
            compaction: limit_llm::CompactionSettings::default(),
            cache: limit_llm::CacheSettings::default(),
        }
    }

    #[test]
    fn test_tui_app_new() {
        // Terminal::new() calls backend.size() which fails without a TTY
        if std::io::stdout().is_terminal() {
            let config = create_test_config();
            let agent_bridge = AgentBridge::new(config).unwrap();
            let (_tx, rx) = mpsc::unbounded_channel();

            let tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();
            let app = TuiApp::new(tui_bridge);
            assert!(app.is_ok());
        }
    }
}
