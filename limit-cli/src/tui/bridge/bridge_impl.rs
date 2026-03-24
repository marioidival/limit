//! TUI Bridge module
//!
//! Bridge connecting limit-cli REPL to limit-tui components

use crate::agent_bridge::{AgentBridge, AgentEvent};
use crate::error::CliError;
use crate::session::SessionManager;
use crate::tui::{activity::format_activity_message, TuiState};
use limit_tui::components::{ActivityFeed, ChatView, Message, Spinner};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::trace;

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
    /// Activity feed for showing tool activities
    activity_feed: Arc<Mutex<ActivityFeed>>,
    /// Spinner for thinking state
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
    /// Current operation ID (to ignore events from old operations)
    operation_id: Arc<Mutex<u64>>,
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

        Self::with_session_manager(agent_bridge, event_rx, session_manager)
    }

    /// Create a new TuiBridge for testing with a temporary session manager
    #[cfg(test)]
    pub fn new_for_test(
        agent_bridge: AgentBridge,
        event_rx: mpsc::UnboundedReceiver<AgentEvent>,
    ) -> Result<Self, CliError> {
        use tempfile::TempDir;

        // Create a temporary directory for the test
        let temp_dir = TempDir::new().map_err(|e| {
            CliError::ConfigError(format!("Failed to create temp directory: {}", e))
        })?;

        let db_path = temp_dir.path().join("session.db");
        let sessions_dir = temp_dir.path().join("sessions");

        let session_manager = SessionManager::with_paths(db_path, sessions_dir).map_err(|e| {
            CliError::ConfigError(format!("Failed to create session manager: {}", e))
        })?;

        Self::with_session_manager(agent_bridge, event_rx, session_manager)
    }

    /// Create a new TuiBridge with a custom session manager
    pub fn with_session_manager(
        agent_bridge: AgentBridge,
        event_rx: mpsc::UnboundedReceiver<AgentEvent>,
        session_manager: SessionManager,
    ) -> Result<Self, CliError> {
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

        // Add model info as system message
        let model_name = agent_bridge.model().to_string();
        if !model_name.is_empty() {
            let model_msg = Message::system(format!("Using model: {}", model_name));
            chat_view.lock().unwrap().add_message(model_msg);
        }

        Ok(Self {
            agent_bridge: Arc::new(Mutex::new(agent_bridge)),
            event_rx,
            state: Arc::new(Mutex::new(TuiState::Idle)),
            chat_view,
            activity_feed: Arc::new(Mutex::new(ActivityFeed::new())),
            spinner: Arc::new(Mutex::new(Spinner::new("Thinking..."))),
            messages: Arc::new(Mutex::new(messages)),
            total_input_tokens: Arc::new(Mutex::new(initial_input)),
            total_output_tokens: Arc::new(Mutex::new(initial_output)),
            session_manager: Arc::new(Mutex::new(session_manager)),
            session_id: Arc::new(Mutex::new(session_id)),
            operation_id: Arc::new(Mutex::new(0)),
        })
    }

    /// Trigger TLDR warm if enabled for the project
    pub fn trigger_tldr_warm_if_enabled(&self) {
        use crate::project_settings::ProjectSettings;
        use std::path::PathBuf;

        let project_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let settings = match ProjectSettings::new() {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!("Failed to get project settings: {}", e);
                return;
            }
        };

        if settings.is_warm_enabled(&project_path) {
            tracing::info!("TLDR warm enabled, triggering background warm");
            let agent_bridge = Arc::clone(&self.agent_bridge);
            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::error!("Failed to create tokio runtime for TLDR warm: {}", e);
                        return;
                    }
                };
                rt.block_on(async {
                    let tldr_tool = agent_bridge.lock().unwrap().tldr_tool();
                    if let Some(tldr_tool) = tldr_tool {
                        tldr_tool.run_warm().await;
                    }
                });
            });
        }
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

    /// Get a reference to the activity feed
    pub fn activity_feed(&self) -> &Arc<Mutex<ActivityFeed>> {
        &self.activity_feed
    }

    /// Process events from the agent and update TUI state
    pub fn process_events(&mut self) -> Result<(), CliError> {
        let mut event_count = 0;
        let current_op_id = self.operation_id();

        while let Ok(event) = self.event_rx.try_recv() {
            event_count += 1;

            // Get operation_id from event
            let event_op_id = match &event {
                AgentEvent::Thinking { operation_id } => *operation_id,
                AgentEvent::ToolStart { operation_id, .. } => *operation_id,
                AgentEvent::ToolComplete { operation_id, .. } => *operation_id,
                AgentEvent::ContentChunk { operation_id, .. } => *operation_id,
                AgentEvent::Done { operation_id } => *operation_id,
                AgentEvent::Cancelled { operation_id } => *operation_id,
                AgentEvent::Error { operation_id, .. } => *operation_id,
                AgentEvent::TokenUsage { operation_id, .. } => *operation_id,
            };

            trace!(
                "process_events: event_op_id={}, current_op_id={}, event={:?}",
                event_op_id,
                current_op_id,
                std::mem::discriminant(&event)
            );

            // Ignore events from old operations
            if event_op_id != current_op_id {
                trace!(
                    "process_events: Ignoring event from old operation {} (current: {})",
                    event_op_id,
                    current_op_id
                );
                continue;
            }

            match event {
                AgentEvent::Thinking { operation_id: _ } => {
                    trace!("process_events: Thinking event received - setting state to Thinking",);
                    *self.state.lock().unwrap() = TuiState::Thinking;
                    trace!("process_events: state is now {:?}", self.state());
                }
                AgentEvent::ToolStart {
                    operation_id: _,
                    name,
                    args,
                } => {
                    trace!("process_events: ToolStart event - {}", name);
                    let activity_msg = format_activity_message(&name, &args);
                    // Add to activity feed instead of changing state
                    self.activity_feed.lock().unwrap().add(activity_msg, true);
                }
                AgentEvent::ToolComplete {
                    operation_id: _,
                    name: _,
                    result: _,
                } => {
                    trace!("process_events: ToolComplete event");
                    // Mark current activity as complete
                    self.activity_feed.lock().unwrap().complete_current();
                }
                AgentEvent::ContentChunk {
                    operation_id: _,
                    chunk,
                } => {
                    trace!("process_events: ContentChunk event ({} chars)", chunk.len());
                    self.chat_view
                        .lock()
                        .unwrap()
                        .append_to_last_assistant(&chunk);
                }
                AgentEvent::Done { operation_id: _ } => {
                    trace!("process_events: Done event received");
                    *self.state.lock().unwrap() = TuiState::Idle;
                    // Mark all activities as complete when LLM finishes
                    self.activity_feed.lock().unwrap().complete_all();
                }
                AgentEvent::Cancelled { operation_id: _ } => {
                    trace!("process_events: Cancelled event received");
                    *self.state.lock().unwrap() = TuiState::Idle;
                    // Mark all activities as complete
                    self.activity_feed.lock().unwrap().complete_all();
                }
                AgentEvent::Error {
                    operation_id: _,
                    message,
                } => {
                    trace!("process_events: Error event - {}", message);
                    // Reset state to Idle so user can continue
                    *self.state.lock().unwrap() = TuiState::Idle;
                    let chat_msg = Message::system(format!("Error: {}", message));
                    self.chat_view.lock().unwrap().add_message(chat_msg);
                }
                AgentEvent::TokenUsage { .. } => {}
            }
        }
        if event_count > 0 {
            trace!("process_events: processed {} events", event_count);
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

    /// Get current operation ID
    #[inline]
    pub fn operation_id(&self) -> u64 {
        *self.operation_id.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Increment and get new operation ID
    pub fn next_operation_id(&self) -> u64 {
        let mut id = self.operation_id.lock().unwrap_or_else(|e| e.into_inner());
        *id += 1;
        *id
    }

    /// Get total input tokens for the session
    #[inline]
    pub fn total_input_tokens(&self) -> u64 {
        *self
            .total_input_tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Get total output tokens for the session
    #[inline]
    pub fn total_output_tokens(&self) -> u64 {
        *self
            .total_output_tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Get the current session ID
    pub fn session_id(&self) -> String {
        self.session_id
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| String::from("unknown"))
    }

    /// Save the current session
    pub fn save_session(&self) -> Result<(), CliError> {
        let session_id = self
            .session_id
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| String::from("unknown"));

        let messages = self
            .messages
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let input_tokens = self
            .total_input_tokens
            .lock()
            .map(|guard| *guard)
            .unwrap_or(0);

        let output_tokens = self
            .total_output_tokens
            .lock()
            .map(|guard| *guard)
            .unwrap_or(0);

        tracing::debug!(
            "Saving session {} with {} messages, {} in tokens, {} out tokens",
            session_id,
            messages.len(),
            input_tokens,
            output_tokens
        );

        let session_manager = self.session_manager.lock().map_err(|e| {
            CliError::ConfigError(format!("Failed to acquire session manager lock: {}", e))
        })?;

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

    /// Get session manager (for command handling)
    pub fn session_manager(&self) -> Arc<Mutex<SessionManager>> {
        self.session_manager.clone()
    }

    /// Get messages arc (for command handling)
    pub fn messages(&self) -> Arc<Mutex<Vec<limit_llm::Message>>> {
        self.messages.clone()
    }

    /// Get state arc (for command handling)
    pub fn state_arc(&self) -> Arc<Mutex<TuiState>> {
        self.state.clone()
    }

    /// Get total input tokens arc (for command handling)
    pub fn total_input_tokens_arc(&self) -> Arc<Mutex<u64>> {
        self.total_input_tokens.clone()
    }

    /// Get total output tokens arc (for command handling)
    pub fn total_output_tokens_arc(&self) -> Arc<Mutex<u64>> {
        self.total_output_tokens.clone()
    }

    /// Get session id arc (for command handling)
    pub fn session_id_arc(&self) -> Arc<Mutex<String>> {
        self.session_id.clone()
    }

    /// Set state (for cancellation)
    pub fn set_state(&self, new_state: TuiState) {
        *self.state.lock().unwrap() = new_state;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let op_id = tui_bridge.operation_id();
        tx.send(AgentEvent::Thinking {
            operation_id: op_id,
        })
        .unwrap();
        tui_bridge.process_events().unwrap();
        assert!(matches!(tui_bridge.state(), TuiState::Thinking));

        tx.send(AgentEvent::Done {
            operation_id: op_id,
        })
        .unwrap();
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
        assert_eq!(tui_bridge.chat_view().lock().unwrap().message_count(), 3); // 1 user + 2 system (welcome + model)
    }
}
