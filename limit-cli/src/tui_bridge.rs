use crate::agent_bridge::{AgentBridge, AgentEvent};
use crate::error::CliError;
use crate::session::SessionManager;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use limit_tui::components::{ChatView, Message, ProgressBar, Spinner};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Debug log to file (bypasses tracing)
fn debug_log(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()) + "/.limit/logs/tui.log")
    {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}
/// TUI state for displaying agent events
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TuiState {
    #[default]
    Idle,
    Thinking,
    ToolExecuting {
        name: String,
        progress: f32,
    },
    #[allow(dead_code)]
    Error(String),
}

/// Bridge connecting limit-cli REPL to limit-tui components
pub struct TuiBridge {
    /// Agent bridge for processing messages (wrapped for thread-safe access)
    agent_bridge: Arc<Mutex<AgentBridge>>,
    /// Event receiver from the agent
    event_rx: mpsc::UnboundedReceiver<AgentEvent>,
    /// Current TUI state
    state: Arc<Mutex<TuiState>>,
    /// Chat view for displaying conversation
    chat_view: Arc<Mutex<ChatView>>,
    /// Progress bar for tool execution
    progress_bar: Arc<Mutex<ProgressBar>>,
    /// Spinner for thinking state
    spinner: Arc<Mutex<Spinner>>,
    /// Conversation history
    messages: Arc<Mutex<Vec<limit_llm::Message>>>,
    /// Total input tokens for the session
    total_input_tokens: Arc<Mutex<u64>>,
    /// Total output tokens for the session
    total_output_tokens: Arc<Mutex<u64>>,
    /// Session manager for persistence
    session_manager: Arc<Mutex<SessionManager>>,
    /// Current session ID
    session_id: Arc<Mutex<String>>,
}

impl TuiBridge {
    /// Create a new TuiBridge with the given agent bridge and event channel
    pub fn new(
        agent_bridge: AgentBridge,
        event_rx: mpsc::UnboundedReceiver<AgentEvent>,
    ) -> Result<Self, CliError> {
        let session_manager = SessionManager::new().map_err(|e| {
            CliError::ConfigError(format!("Failed to create session manager: {}", e))
        })?;

        // Always create a new session on TUI startup
        let session_id = session_manager
            .create_new_session()
            .map_err(|e| CliError::ConfigError(format!("Failed to create session: {}", e)))?;
        tracing::info!("Created new TUI session: {}", session_id);

        // Start with empty messages - never load previous session
        let messages: Vec<limit_llm::Message> = Vec::new();

        // Get token counts from session info
        let sessions = session_manager.list_sessions().unwrap_or_default();
        let session_info = sessions.iter().find(|s| s.id == session_id);
        let initial_input = session_info.map(|s| s.total_input_tokens).unwrap_or(0);
        let initial_output = session_info.map(|s| s.total_output_tokens).unwrap_or(0);

        let chat_view = Arc::new(Mutex::new(ChatView::new()));

        // Add loaded messages to chat view for display
        for msg in &messages {
            match msg.role {
                limit_llm::Role::User => {
                    let chat_msg = Message::user(msg.content.clone().unwrap_or_default());
                    chat_view.lock().unwrap().add_message(chat_msg);
                }
                limit_llm::Role::Assistant => {
                    let content = msg.content.clone().unwrap_or_default();
                    let chat_msg = Message::assistant(content);
                    chat_view.lock().unwrap().add_message(chat_msg);
                }
                limit_llm::Role::System => {
                    // Skip system messages in display
                }
                limit_llm::Role::Tool => {
                    // Skip tool messages in display
                }
            }
        }

        tracing::info!("Loaded {} messages into chat view", messages.len());

        // Add system message to indicate this is a new session
        let session_short_id = format!("...{}", &session_id[session_id.len().saturating_sub(8)..]);
        let welcome_msg =
            Message::system(format!("🆕 New TUI session started: {}", session_short_id));
        chat_view.lock().unwrap().add_message(welcome_msg);

        Ok(Self {
            agent_bridge: Arc::new(Mutex::new(agent_bridge)),
            event_rx,
            state: Arc::new(Mutex::new(TuiState::Idle)),
            chat_view,
            progress_bar: Arc::new(Mutex::new(ProgressBar::new("Tool execution"))),
            spinner: Arc::new(Mutex::new(Spinner::new("Thinking..."))),
            messages: Arc::new(Mutex::new(messages)),
            total_input_tokens: Arc::new(Mutex::new(initial_input)),
            total_output_tokens: Arc::new(Mutex::new(initial_output)),
            session_manager: Arc::new(Mutex::new(session_manager)),
            session_id: Arc::new(Mutex::new(session_id)),
        })
    }

    /// Get a clone of the agent bridge Arc for spawning tasks
    pub fn agent_bridge_arc(&self) -> Arc<Mutex<AgentBridge>> {
        self.agent_bridge.clone()
    }

    /// Get locked access to the agent bridge (for compatibility)
    #[allow(dead_code)]
    pub fn agent_bridge(&self) -> std::sync::MutexGuard<'_, AgentBridge> {
        self.agent_bridge.lock().unwrap()
    }

    /// Get a reference to the progress bar
    #[allow(dead_code)]
    pub fn progress_bar(&self) -> &Arc<Mutex<ProgressBar>> {
        &self.progress_bar
    }

    /// Get the current TUI state
    pub fn state(&self) -> TuiState {
        self.state.lock().unwrap().clone()
    }

    /// Get a reference to the chat view
    pub fn chat_view(&self) -> &Arc<Mutex<ChatView>> {
        &self.chat_view
    }

    /// Get a reference to the spinner
    pub fn spinner(&self) -> &Arc<Mutex<Spinner>> {
        &self.spinner
    }

    /// Process events from the agent and update TUI state
    pub fn process_events(&mut self) -> Result<(), CliError> {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                AgentEvent::Thinking => {
                    *self.state.lock().unwrap() = TuiState::Thinking;
                }
                AgentEvent::ToolStart { name, args: _ } => {
                    *self.state.lock().unwrap() = TuiState::ToolExecuting {
                        name: name.clone(),
                        progress: 0.0,
                    };
                    self.progress_bar.lock().unwrap().set_value(0.0);
                }
                AgentEvent::ToolComplete { name: _, result: _ } => {
                    *self.state.lock().unwrap() = TuiState::Idle;
                    self.progress_bar.lock().unwrap().set_value(1.0);
                }
                AgentEvent::ContentChunk(chunk) => {
                    // Append to last assistant message instead of creating new ones
                    self.chat_view
                        .lock()
                        .unwrap()
                        .append_to_last_assistant(&chunk);
                }
                AgentEvent::TokenUsage {
                    input_tokens,
                    output_tokens,
                } => {
                    // Accumulate token counts
                    *self.total_input_tokens.lock().unwrap() += input_tokens;
                    *self.total_output_tokens.lock().unwrap() += output_tokens;
                }
                AgentEvent::Done => {
                    *self.state.lock().unwrap() = TuiState::Idle;
                }
                AgentEvent::Error(err) => {
                    // Reset state to Idle so user can continue
                    *self.state.lock().unwrap() = TuiState::Idle;
                    let chat_msg = Message::system(format!("❌ Error: {}", err));
                    self.chat_view.lock().unwrap().add_message(chat_msg);
                }
            }
        }
        Ok(())
    }

    /// Add a user message to the chat
    pub fn add_user_message(&self, content: String) {
        let msg = Message::user(content);
        self.chat_view.lock().unwrap().add_message(msg);
    }

    /// Tick the spinner animation
    pub fn tick_spinner(&self) {
        self.spinner.lock().unwrap().tick();
    }

    /// Check if agent is busy
    pub fn is_busy(&self) -> bool {
        !matches!(self.state(), TuiState::Idle)
    }

    /// Get total input tokens for the session
    pub fn total_input_tokens(&self) -> u64 {
        *self.total_input_tokens.lock().unwrap()
    }

    /// Get total output tokens for the session
    pub fn total_output_tokens(&self) -> u64 {
        *self.total_output_tokens.lock().unwrap()
    }

    /// Get the current session ID
    pub fn session_id(&self) -> String {
        self.session_id.lock().unwrap().clone()
    }

    /// Save the current session
    pub fn save_session(&self) -> Result<(), CliError> {
        let session_id = self.session_id.lock().unwrap().clone();
        let messages = self.messages.lock().unwrap().clone();
        let input_tokens = self.total_input_tokens();
        let output_tokens = self.total_output_tokens();

        tracing::debug!(
            "Saving session {} with {} messages, {} in tokens, {} out tokens",
            session_id,
            messages.len(),
            input_tokens,
            output_tokens
        );

        let session_manager = self.session_manager.lock().unwrap();
        session_manager.save_session(&session_id, &messages, input_tokens, output_tokens)?;
        tracing::info!(
            "✓ Session {} saved successfully ({} messages, {} in tokens, {} out tokens)",
            session_id,
            messages.len(),
            input_tokens,
            output_tokens
        );
        Ok(())
    }
}

/// TUI Application for running the limit CLI in a terminal UI
pub struct TuiApp {
    tui_bridge: TuiBridge,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    running: bool,
    input_text: String,
    cursor_pos: usize,
    status_message: String,
    status_is_error: bool,
    cursor_blink_state: bool,
    cursor_blink_timer: std::time::Instant,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new(tui_bridge: TuiBridge) -> Result<Self, CliError> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal =
            Terminal::new(backend).map_err(|e| CliError::IoError(io::Error::other(e)))?;

        let session_id = tui_bridge.session_id();
        tracing::info!("TUI started with session: {}", session_id);

        Ok(Self {
            tui_bridge,
            terminal,
            running: true,
            input_text: String::new(),
            cursor_pos: 0,
            status_message: "Ready - Type a message and press Enter".to_string(),
            status_is_error: false,
            cursor_blink_state: true,
            cursor_blink_timer: std::time::Instant::now(),
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

        crossterm::terminal::enable_raw_mode()
            .map_err(|e| CliError::IoError(io::Error::other(e)))?;

        // Guard to ensure cleanup on panic
        struct AlternateScreenGuard;
        impl Drop for AlternateScreenGuard {
            fn drop(&mut self) {
                let _ = crossterm::terminal::disable_raw_mode();
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
            if crossterm::event::poll(std::time::Duration::from_millis(50))
                .map_err(|e| CliError::IoError(io::Error::other(e)))?
            {
                match event::read().map_err(|e| CliError::IoError(io::Error::other(e)))? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key_event(key)?;
                    }
                    Event::Mouse(mouse) => match mouse.kind {
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

        Ok(())
    }

    fn update_status(&mut self) {
        let session_id = self.tui_bridge.session_id();
        match self.tui_bridge.state() {
            TuiState::Idle => {
                self.status_message = format!(
                    "Ready - Type a message and press Enter | Session: {}",
                    session_id.chars().take(8).collect::<String>()
                );
                self.status_is_error = false;
            }
            TuiState::Thinking => {
                let spinner = self.tui_bridge.spinner().lock().unwrap();
                self.status_message = format!(
                    "{} Thinking... | Session: {}",
                    spinner.current_frame(),
                    session_id.chars().take(8).collect::<String>()
                );
                self.status_is_error = false;
            }
            TuiState::ToolExecuting { name, progress } => {
                let pct = (progress * 100.0) as u32;
                self.status_message = format!(
                    "⏳ Executing: {} ({}%) | Session: {}",
                    name,
                    pct,
                    session_id.chars().take(8).collect::<String>()
                );
                self.status_is_error = false;
            }
            TuiState::Error(msg) => {
                self.status_message = format!(
                    "❌ Error: {} | Session: {}",
                    msg,
                    session_id.chars().take(8).collect::<String>()
                );
                self.status_is_error = true;
            }
        }
    }

    fn tick_cursor_blink(&mut self) {
        // Blink every 500ms for standard terminal cursor behavior
        if self.cursor_blink_timer.elapsed().as_millis() > 500 {
            self.cursor_blink_state = !self.cursor_blink_state;
            self.cursor_blink_timer = std::time::Instant::now();
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<(), CliError> {
        // Direct file logging (always works)
        debug_log(&format!(
            "Key: {:?} mod={:?} kind={:?}",
            key.code, key.modifiers, key.kind
        ));

        // Allow Ctrl+C to exit anytime
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
            debug_log("Ctrl+C - exiting");
            self.running = false;
            return Ok(());
        }

        // Allow scrolling even when agent is busy
        // Calculate actual viewport height dynamically
        let term_height = self.terminal.size().map(|s| s.height).unwrap_or(24);
        let viewport_height = term_height.saturating_sub(1) // status bar - input area (6 lines) - borders (~2)
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
                let mut chat = self.tui_bridge.chat_view().lock().unwrap();
                chat.scroll_up();
                return Ok(());
            }
            KeyCode::Down => {
                let mut chat = self.tui_bridge.chat_view().lock().unwrap();
                chat.scroll_down();
                return Ok(());
            }
            _ => {}
        }

        // Don't accept input while agent is busy
        if self.tui_bridge.is_busy() {
            debug_log("Agent busy, ignoring");
            return Ok(());
        }

        // Handle backspace - try multiple detection methods
        if self.handle_backspace(&key) {
            debug_log(&format!("Backspace handled, input: {:?}", self.input_text));
            return Ok(());
        }

        match key.code {
            KeyCode::Delete => {
                if self.cursor_pos < self.input_text.len() {
                    let next_pos = self.next_char_pos();
                    self.input_text.drain(self.cursor_pos..next_pos);
                    debug_log(&format!("Delete: input now: {:?}", self.input_text));
                }
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos = self.prev_char_pos();
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input_text.len() {
                    self.cursor_pos = self.next_char_pos();
                }
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
            }
            KeyCode::End => {
                self.cursor_pos = self.input_text.len();
            }
            KeyCode::Enter => {
                self.handle_enter()?;
            }
            KeyCode::Esc => {
                debug_log("Esc pressed, exiting");
                self.running = false;
            }
            // Regular character input (including UTF-8)
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
            {
                // Insert the character at cursor position
                self.input_text.insert(self.cursor_pos, c);
                self.cursor_pos += c.len_utf8();
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
            debug_log("Backspace detected via KeyCode::Backspace");
            self.delete_char_before_cursor();
            return true;
        }

        // Method 2: Ctrl+H (common backspace mapping)
        if key.code == KeyCode::Char('h') && key.modifiers == KeyModifiers::CONTROL {
            debug_log("Backspace detected via Ctrl+H");
            self.delete_char_before_cursor();
            return true;
        }

        // Method 3: Check for DEL (127) or BS (8) characters
        if let KeyCode::Char(c) = key.code {
            if c == '\x7f' || c == '\x08' {
                debug_log(&format!("Backspace detected via char code: {}", c as u8));
                self.delete_char_before_cursor();
                return true;
            }
        }

        false
    }

    fn delete_char_before_cursor(&mut self) {
        debug_log(&format!(
            "delete_char: cursor={}, len={}, input={:?}",
            self.cursor_pos,
            self.input_text.len(),
            self.input_text
        ));
        if self.cursor_pos > 0 {
            let prev_pos = self.prev_char_pos();
            debug_log(&format!("draining {}..{}", prev_pos, self.cursor_pos));
            self.input_text.drain(prev_pos..self.cursor_pos);
            self.cursor_pos = prev_pos;
            debug_log(&format!(
                "after delete: cursor={}, input={:?}",
                self.cursor_pos, self.input_text
            ));
        } else {
            debug_log("cursor at 0, nothing to delete");
        }
    }

    fn handle_enter(&mut self) -> Result<(), CliError> {
        let text = self.input_text.trim().to_string();

        // Clear input FIRST for immediate visual feedback
        self.input_text.clear();
        self.cursor_pos = 0;

        if text.is_empty() {
            return Ok(());
        }

        tracing::info!("Enter pressed with text: {:?}", text);

        // Handle commands locally (no LLM)
        let text_lower = text.to_lowercase();
        if text_lower == "/exit"
            || text_lower == "/quit"
            || text_lower == "exit"
            || text_lower == "quit"
        {
            tracing::info!("Exit command detected, exiting");
            self.running = false;
            return Ok(());
        }

        if text_lower == "/clear" || text_lower == "clear" {
            tracing::info!("Clear command detected");
            self.tui_bridge.chat_view().lock().unwrap().clear();
            return Ok(());
        }

        if text_lower == "/help" || text_lower == "help" {
            tracing::info!("Help command detected");
            let help_msg = Message::system(
                "Available commands:\n\
                 /help  - Show this help message\n\
                 /clear - Clear chat history\n\
                 /exit  - Exit the application\n\
                 /quit  - Exit the application\n\
                 \n\
                 Page Up/Down - Scroll chat history"
                    .to_string(),
            );
            self.tui_bridge
                .chat_view()
                .lock()
                .unwrap()
                .add_message(help_msg);
            return Ok(());
        }

        // Add user message to chat (for display)
        self.tui_bridge.add_user_message(text.clone());

        // Clone Arcs for the spawned thread
        let messages = self.tui_bridge.messages.clone();
        let chat_view = self.tui_bridge.chat_view().clone();
        let agent_bridge = self.tui_bridge.agent_bridge_arc();
        let session_manager = self.tui_bridge.session_manager.clone();
        let session_id = self.tui_bridge.session_id();
        let total_input_tokens = self.tui_bridge.total_input_tokens.clone();
        let total_output_tokens = self.tui_bridge.total_output_tokens.clone();

        tracing::debug!("Spawning LLM processing thread");

        // Spawn a thread to process the message without blocking the UI
        std::thread::spawn(move || {
            // Create a new tokio runtime for this thread
            let rt = tokio::runtime::Runtime::new().unwrap();

            // Safe: we're in a dedicated thread, this won't cause issues
            #[allow(clippy::await_holding_lock)]
            rt.block_on(async {
                let mut messages_guard = messages.lock().unwrap();
                let mut bridge = agent_bridge.lock().unwrap();

                match bridge.process_message(&text, &mut messages_guard).await {
                    Ok(response) => {
                        // Add assistant response to chat view for display
                        chat_view
                            .lock()
                            .unwrap()
                            .add_message(Message::assistant(response));

                        // Auto-save session after successful response
                        let msgs = messages_guard.clone();
                        let input_tokens = *total_input_tokens.lock().unwrap();
                        let output_tokens = *total_output_tokens.lock().unwrap();

                        if let Err(e) = session_manager.lock().unwrap().save_session(
                            &session_id,
                            &msgs,
                            input_tokens,
                            output_tokens,
                        ) {
                            tracing::error!("✗ Failed to auto-save session {}: {}", session_id, e);
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
                        tracing::error!("LLM error: {}", e);
                    }
                }
            });
        });

        Ok(())
    }

    fn prev_char_pos(&self) -> usize {
        if self.cursor_pos == 0 {
            return 0;
        }
        // Start ONE position before cursor, then find char boundary
        let mut pos = self.cursor_pos - 1;
        while pos > 0 && !self.input_text.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    }

    fn next_char_pos(&self) -> usize {
        if self.cursor_pos >= self.input_text.len() {
            return self.input_text.len();
        }
        // Start ONE position after cursor, then find char boundary
        let mut pos = self.cursor_pos + 1;
        while pos < self.input_text.len() && !self.input_text.is_char_boundary(pos) {
            pos += 1;
        }
        pos
    }

    fn draw(&mut self) -> Result<(), CliError> {
        let chat_view = self.tui_bridge.chat_view().clone();
        let state = self.tui_bridge.state();
        let input_text = self.input_text.clone();
        let cursor_pos = self.cursor_pos;
        let status_message = self.status_message.clone();
        let status_is_error = self.status_is_error;
        let cursor_blink_state = self.cursor_blink_state;
        let tui_bridge = &self.tui_bridge;

        self.terminal
            .draw(|f| {
                Self::draw_ui(
                    f,
                    &chat_view,
                    state,
                    &input_text,
                    cursor_pos,
                    &status_message,
                    status_is_error,
                    cursor_blink_state,
                    tui_bridge,
                );
            })
            .map_err(|e| CliError::IoError(io::Error::other(e)))?;

        Ok(())
    }

    /// Draw the TUI interface
    #[allow(clippy::too_many_arguments)]
    fn draw_ui(
        f: &mut Frame,
        chat_view: &Arc<Mutex<ChatView>>,
        _state: TuiState,
        input_text: &str,
        cursor_pos: usize,
        status_message: &str,
        status_is_error: bool,
        cursor_blink_state: bool,
        tui_bridge: &TuiBridge,
    ) {
        let size = f.area();

        // Split the screen: chat, status, input
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(60), // Chat view (60% of height)
                    Constraint::Length(1), // Status bar
                    Constraint::Length(6), // Input area (increased for wrapped text)
                ]
                .as_ref(),
            )
            .split(size);

        // Draw chat view with border
        {
            let chat = chat_view.lock().unwrap();
            let total_input = tui_bridge.total_input_tokens();
            let total_output = tui_bridge.total_output_tokens();
            let title = format!(" Chat (In: {} | Out: {}) ", total_input, total_output);
            let chat_block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(&*chat, chat_block.inner(chunks[0]));
            f.render_widget(chat_block, chunks[0]);
        }

        // Draw status bar
        {
            let status_style = if status_is_error {
                Style::default().fg(Color::Red).bg(Color::Reset)
            } else {
                Style::default().fg(Color::Yellow)
            };

            let status = Paragraph::new(Line::from(vec![
                Span::styled(" ● ", Style::default().fg(Color::Green)),
                Span::styled(status_message, status_style),
            ]));
            f.render_widget(status, chunks[1]);
        }

        // Draw input area with border
        {
            let input_block = Block::default()
                .borders(Borders::ALL)
                .title(" Input (Esc to quit) ")
                .title_style(Style::default().fg(Color::Cyan));

            let input_inner = input_block.inner(chunks[2]);
            f.render_widget(input_block, chunks[2]);

            // Build input line with cursor
            let before_cursor = &input_text[..cursor_pos];
            let at_cursor = if cursor_pos < input_text.len() {
                &input_text[cursor_pos
                    ..cursor_pos
                        + input_text[cursor_pos..]
                            .chars()
                            .next()
                            .map(|c| c.len_utf8())
                            .unwrap_or(0)]
            } else {
                " "
            };
            let after_cursor = if cursor_pos < input_text.len() {
                &input_text[cursor_pos + at_cursor.len()..]
            } else {
                ""
            };

            let cursor_style = if cursor_blink_state {
                Style::default().bg(Color::White).fg(Color::Black)
            } else {
                Style::default().bg(Color::Reset).fg(Color::Reset)
            };

            let input_line = if input_text.is_empty() {
                Line::from(vec![Span::styled(
                    "Type your message here...",
                    Style::default().fg(Color::DarkGray),
                )])
            } else {
                Line::from(vec![
                    Span::raw(before_cursor),
                    Span::styled(at_cursor, cursor_style),
                    Span::raw(after_cursor),
                ])
            };

            let input_para = Paragraph::new(input_line).wrap(Wrap { trim: false });
            f.render_widget(input_para, input_inner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a test config for AgentBridge
    fn create_test_config() -> limit_llm::Config {
        use limit_llm::ProviderConfig;
        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                api_key: Some("test-key".to_string()),
                model: "claude-3-5-sonnet-20241022".to_string(),
                base_url: None,
                max_tokens: 4096,
                timeout: 60,
            },
        );
        limit_llm::Config {
            provider: "anthropic".to_string(),
            providers,
        }
    }

    #[test]
    fn test_tui_bridge_new() {
        let config = create_test_config();
        let agent_bridge = AgentBridge::new(config).unwrap();
        let (_tx, rx) = mpsc::unbounded_channel();

        let tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();
        assert_eq!(tui_bridge.state(), TuiState::Idle);
    }

    #[test]
    fn test_tui_bridge_state() {
        let config = create_test_config();
        let agent_bridge = AgentBridge::new(config).unwrap();
        let (tx, rx) = mpsc::unbounded_channel();

        let mut tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

        tx.send(AgentEvent::Thinking).unwrap();
        tui_bridge.process_events().unwrap();
        assert!(matches!(tui_bridge.state(), TuiState::Thinking));

        tx.send(AgentEvent::Done).unwrap();
        tui_bridge.process_events().unwrap();
        assert_eq!(tui_bridge.state(), TuiState::Idle);
    }

    #[test]
    fn test_tui_bridge_chat_view() {
        let config = create_test_config();
        let agent_bridge = AgentBridge::new(config).unwrap();
        let (_tx, rx) = mpsc::unbounded_channel();

        let tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

        tui_bridge.add_user_message("Hello".to_string());
        assert_eq!(tui_bridge.chat_view().lock().unwrap().message_count(), 1);
    }

    #[test]
    fn test_tui_state_default() {
        let state = TuiState::default();
        assert_eq!(state, TuiState::Idle);
    }
}
