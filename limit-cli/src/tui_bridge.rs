use crate::agent_bridge::{AgentBridge, AgentEvent};
use crate::error::CliError;
use limit_tui::components::{ChatView, Message, ProgressBar, Spinner};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame, Terminal,
};
use std::io;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// TUI state for displaying agent events
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum TuiState {
    #[default]
    Idle,
    Thinking,
    ToolExecuting { name: String, progress: f32 },
    Error(String),
}


/// Bridge connecting limit-cli REPL to limit-tui components
///
/// This struct manages the TUI rendering and event handling for the agent.
pub struct TuiBridge {
    /// Agent bridge for processing messages
    agent_bridge: AgentBridge,
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
}

impl TuiBridge {
    /// Create a new TuiBridge with the given agent bridge and event channel
    ///
    /// # Arguments
    /// * `agent_bridge` - The agent bridge for processing messages
    /// * `event_rx` - The event receiver channel from the agent
    ///
    /// # Returns
    /// A new TuiBridge instance
    pub fn new(agent_bridge: AgentBridge, event_rx: mpsc::UnboundedReceiver<AgentEvent>) -> Self {
        Self {
            agent_bridge,
            event_rx,
            state: Arc::new(Mutex::new(TuiState::Idle)),
            chat_view: Arc::new(Mutex::new(ChatView::new())),
            progress_bar: Arc::new(Mutex::new(ProgressBar::new("Tool execution"))),
            spinner: Arc::new(Mutex::new(Spinner::new("Thinking..."))),
        }
    }

    /// Get a reference to the agent bridge
    pub fn agent_bridge(&self) -> &AgentBridge {
        &self.agent_bridge
    }

    /// Get a mutable reference to the agent bridge
    pub fn agent_bridge_mut(&mut self) -> &mut AgentBridge {
        &mut self.agent_bridge
    }

    /// Get the current TUI state
    pub fn state(&self) -> TuiState {
        self.state.lock().unwrap().clone()
    }

    /// Get a reference to the chat view
    pub fn chat_view(&self) -> &Arc<Mutex<ChatView>> {
        &self.chat_view
    }

    /// Get a reference to the progress bar
    pub fn progress_bar(&self) -> &Arc<Mutex<ProgressBar>> {
        &self.progress_bar
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
                AgentEvent::ToolStart { name, args } => {
                    *self.state.lock().unwrap() = TuiState::ToolExecuting {
                        name: name.clone(),
                        progress: 0.0,
                    };
                    // Update progress bar
                    self.progress_bar.lock().unwrap().set_value(0.0);
                    // Add system message about tool start
                    let chat_msg = Message::system(format!("Tool: {} ({})", name, args));
                    self.chat_view.lock().unwrap().add_message(chat_msg);
                }
                AgentEvent::ToolComplete { name: _, result } => {
                    *self.state.lock().unwrap() = TuiState::Idle;
                    // Update progress bar to complete
                    self.progress_bar.lock().unwrap().set_value(1.0);
                    // Add result message
                    let result_msg = if result.len() > 500 {
                        format!("Result: {}...", &result[..500])
                    } else {
                        format!("Result: {}", result)
                    };
                    let chat_msg = Message::system(result_msg);
                    self.chat_view.lock().unwrap().add_message(chat_msg);
                }
                AgentEvent::ContentChunk(chunk) => {
                    // Add content as a new assistant message
                    // For better UX, we'd merge consecutive chunks, but this is a simple implementation
                    let chat_msg = Message::assistant(chunk);
                    self.chat_view.lock().unwrap().add_message(chat_msg);
                }
                AgentEvent::Done => {
                    *self.state.lock().unwrap() = TuiState::Idle;
                }
                AgentEvent::Error(err) => {
                    *self.state.lock().unwrap() = TuiState::Error(err.clone());
                    let chat_msg = Message::system(format!("Error: {}", err));
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
}



/// TUI Application for running the limit CLI in a terminal UI
pub struct TuiApp {
    tui_bridge: TuiBridge,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    running: bool,
}

impl TuiApp {
    /// Create a new TUI application
    ///
    /// # Arguments
    /// * `tui_bridge` - The TUI bridge for managing agent events
    ///
    /// # Returns
    /// A new TuiApp instance or an error
    pub fn new(tui_bridge: TuiBridge) -> Result<Self, CliError> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)
            .map_err(|e| CliError::IoError(io::Error::other(e)))?;

        Ok(Self {
            tui_bridge,
            terminal,
            running: true,
        })
    }

    /// Run the TUI event loop
    pub fn run(&mut self) -> Result<(), CliError> {
        crossterm::terminal::enable_raw_mode()
            .map_err(|e| CliError::IoError(io::Error::other(e)))?;

        let result = self.run_inner();

        crossterm::terminal::disable_raw_mode()
            .map_err(|e| CliError::IoError(io::Error::other(e)))?;

        result
    }

    fn run_inner(&mut self) -> Result<(), CliError> {
        while self.running {
            // Process events from the agent
            self.tui_bridge.process_events()?;

            // Update spinner if in thinking state
            if matches!(self.tui_bridge.state(), TuiState::Thinking) {
                self.tui_bridge.tick_spinner();
            }

            // Draw the TUI
            {
                let chat_view = self.tui_bridge.chat_view().clone();
                let progress_bar = self.tui_bridge.progress_bar().clone();
                let spinner = self.tui_bridge.spinner().clone();
                let state = self.tui_bridge.state();

                self.terminal
                    .draw(|f| Self::draw_ui(f, &chat_view, &progress_bar, &spinner, state))
                    .map_err(|e| CliError::IoError(io::Error::other(e)))?;
            }

            // Check for user input (simplified - in a real implementation, we'd use crossterm events)
            std::thread::sleep(std::time::Duration::from_millis(100));

            // Exit condition (simplified)
            if matches!(self.tui_bridge.state(), TuiState::Idle) {
                break;
            }
        }

        Ok(())
    }

    /// Draw the TUI interface
    fn draw_ui(
        f: &mut Frame,
        chat_view: &Arc<Mutex<ChatView>>,
        progress_bar: &Arc<Mutex<ProgressBar>>,
        spinner: &Arc<Mutex<Spinner>>,
        state: TuiState,
    ) {
        let size = f.area();

        // Split the screen into sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Min(10),   // Chat view
                    Constraint::Length(3), // Status bar
                ]
                .as_ref(),
            )
            .split(size);

        // Draw chat view
        {
            let chat = chat_view.lock().unwrap();
            f.render_widget(&*chat, chunks[0]);
        }

        // Draw status bar based on state
        match state {
            TuiState::Thinking => {
                let sp = spinner.lock().unwrap();
                sp.render(chunks[1], f.buffer_mut());
            }
            TuiState::ToolExecuting { progress, .. } => {
                let mut pb = progress_bar.lock().unwrap();
                pb.set_value(progress);
                pb.render(chunks[1], f.buffer_mut());
            }
            TuiState::Idle => {
                let paragraph = Paragraph::new("Ready")
                    .style(Style::default().fg(Color::Green))
                    .alignment(Alignment::Center);
                f.render_widget(paragraph, chunks[1]);
            }
            TuiState::Error(err) => {
                let paragraph = Paragraph::new(format!("Error: {}", err))
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center);
                f.render_widget(paragraph, chunks[1]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use limit_llm::Config as LlmConfig;

    #[test]
    fn test_tui_bridge_new() {
        let config = LlmConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            timeout: 60,
        };

        let agent_bridge = AgentBridge::new(config).unwrap();
        let (_tx, rx) = mpsc::unbounded_channel();

        let tui_bridge = TuiBridge::new(agent_bridge, rx);
        assert_eq!(tui_bridge.state(), TuiState::Idle);
    }

    #[test]
    fn test_tui_bridge_state() {
        let config = LlmConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            timeout: 60,
        };

        let agent_bridge = AgentBridge::new(config).unwrap();
        let (tx, rx) = mpsc::unbounded_channel();

        let mut tui_bridge = TuiBridge::new(agent_bridge, rx);

        // Send thinking event
        tx.send(AgentEvent::Thinking).unwrap();
        tui_bridge.process_events().unwrap();
        assert!(matches!(tui_bridge.state(), TuiState::Thinking));

        // Send tool start event
        tx.send(AgentEvent::ToolStart {
            name: "test_tool".to_string(),
            args: serde_json::json!({"arg": "value"}),
        })
        .unwrap();
        tui_bridge.process_events().unwrap();
        assert!(matches!(tui_bridge.state(), TuiState::ToolExecuting { .. }));

        // Send tool complete event
        tx.send(AgentEvent::ToolComplete {
            name: "test_tool".to_string(),
            result: "success".to_string(),
        })
        .unwrap();
        tui_bridge.process_events().unwrap();
        assert_eq!(tui_bridge.state(), TuiState::Idle);

        // Send error event
        tx.send(AgentEvent::Error("test error".to_string()))
            .unwrap();
        tui_bridge.process_events().unwrap();
        assert!(matches!(tui_bridge.state(), TuiState::Error(_)));
    }

    #[test]
    fn test_tui_bridge_chat_view() {
        let config = LlmConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            timeout: 60,
        };

        let agent_bridge = AgentBridge::new(config).unwrap();
        let (_tx, rx) = mpsc::unbounded_channel();

        let tui_bridge = TuiBridge::new(agent_bridge, rx);

        // Add user message
        tui_bridge.add_user_message("Hello".to_string());
        assert_eq!(tui_bridge.chat_view().lock().unwrap().message_count(), 1);

        // Add another user message
        tui_bridge.add_user_message("World".to_string());
        assert_eq!(tui_bridge.chat_view().lock().unwrap().message_count(), 2);
    }

    #[test]
    fn test_tui_bridge_content_chunk() {
        let config = LlmConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            timeout: 60,
        };

        let agent_bridge = AgentBridge::new(config).unwrap();
        let (tx, rx) = mpsc::unbounded_channel();

        let mut tui_bridge = TuiBridge::new(agent_bridge, rx);

        // Send content chunk
        tx.send(AgentEvent::ContentChunk("Hello".to_string()))
            .unwrap();
        tui_bridge.process_events().unwrap();

        // Should have created an assistant message
        assert_eq!(tui_bridge.chat_view().lock().unwrap().message_count(), 1);
    }

    #[test]
    fn test_tui_bridge_spinner() {
        let config = LlmConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            timeout: 60,
        };

        let agent_bridge = AgentBridge::new(config).unwrap();
        let (_tx, rx) = mpsc::unbounded_channel();

        let tui_bridge = TuiBridge::new(agent_bridge, rx);

        // Initial frame
        let initial_str = {
            let spinner = tui_bridge.spinner().lock().unwrap();
            spinner.current_frame().to_string()
        };

        // Tick spinner
        tui_bridge.tick_spinner();

        // Frame should have changed
        let new_str = {
            let spinner = tui_bridge.spinner().lock().unwrap();
            spinner.current_frame().to_string()
        };
        assert_ne!(initial_str, new_str);
    }

    #[test]
    fn test_tui_state_default() {
        let state = TuiState::default();
        assert_eq!(state, TuiState::Idle);
    }
}
