//! Command registry and trait definition
//!
//! Provides the core command system architecture.

use crate::error::CliError;
use crate::session::SessionManager;
use crate::tui::team_progress::TeamProgressState;
use crate::tui::TuiState;
use limit_agent::team::TeamProgressEvent;
use limit_tui::components::{ChatView, Message};
use parking_lot::Mutex as ParkingMutex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Result of executing a command
#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    /// Command executed successfully, continue running
    Continue,
    /// Exit the application
    Exit,
    /// Clear the chat view
    ClearChat,
    /// Add a message to the chat
    Message(String),
    /// Create a new session (returned by /session new)
    NewSession,
    /// Load a session (returned by /session load)
    LoadSession(String),
    /// Share/export session
    Share(String),
}

/// Context provided to commands for execution
pub struct CommandContext {
    /// Chat view for displaying messages
    pub chat_view: Arc<Mutex<ChatView>>,
    /// Session manager for session operations
    pub session_manager: Arc<Mutex<SessionManager>>,
    /// Current session ID
    pub session_id: String,
    /// Current TUI state
    pub state: Arc<Mutex<TuiState>>,
    /// Conversation messages (LLM format)
    pub messages: Arc<Mutex<Vec<limit_llm::Message>>>,
    /// Total input tokens
    pub total_input_tokens: Arc<Mutex<u64>>,
    /// Total output tokens
    pub total_output_tokens: Arc<Mutex<u64>>,
    /// Clipboard manager (optional)
    pub clipboard: Option<Arc<Mutex<crate::clipboard::ClipboardManager>>>,
    /// Holder for team progress event receiver (shared with main loop)
    pub team_progress_rx: Arc<ParkingMutex<Option<mpsc::UnboundedReceiver<TeamProgressEvent>>>>,
    /// Team progress state for resetting between runs
    pub team_progress: Arc<TeamProgressState>,
}

impl CommandContext {
    /// Create a new command context
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chat_view: Arc<Mutex<ChatView>>,
        session_manager: Arc<Mutex<SessionManager>>,
        session_id: String,
        state: Arc<Mutex<TuiState>>,
        messages: Arc<Mutex<Vec<limit_llm::Message>>>,
        total_input_tokens: Arc<Mutex<u64>>,
        total_output_tokens: Arc<Mutex<u64>>,
        clipboard: Option<Arc<Mutex<crate::clipboard::ClipboardManager>>>,
        team_progress_rx: Arc<ParkingMutex<Option<mpsc::UnboundedReceiver<TeamProgressEvent>>>>,
        team_progress: Arc<TeamProgressState>,
    ) -> Self {
        Self {
            chat_view,
            session_manager,
            session_id,
            state,
            messages,
            total_input_tokens,
            total_output_tokens,
            clipboard,
            team_progress_rx,
            team_progress,
        }
    }

    /// Add a system message to the chat
    pub fn add_system_message(&self, text: String) {
        let msg = Message::system(text);
        self.chat_view.lock().unwrap().add_message(msg);
    }

    /// Add a user message to the chat
    pub fn add_user_message(&self, text: String) {
        let msg = Message::user(text);
        self.chat_view.lock().unwrap().add_message(msg);
    }

    /// Clear the chat view
    pub fn clear_chat(&self) {
        self.chat_view.lock().unwrap().clear();
    }
}

/// Trait for implementing commands
pub trait Command: Send + Sync {
    /// Get the command name (e.g., "help", "session")
    fn name(&self) -> &str;

    /// Get command aliases (e.g., ["?", "h"] for help)
    fn aliases(&self) -> Vec<&str> {
        vec![]
    }

    /// Get command description for help text
    fn description(&self) -> &str;

    /// Get usage examples
    fn usage(&self) -> Vec<&str> {
        vec![]
    }

    /// Execute the command
    fn execute(&self, args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError>;
}

/// Registry for managing commands
pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Register a command
    pub fn register(&mut self, command: Box<dyn Command>) {
        let name = command.name().to_string();
        self.commands.insert(name, command);
    }

    /// Get all registered commands
    #[inline]
    pub fn list_commands(&self) -> Vec<&dyn Command> {
        self.commands.values().map(|c| c.as_ref()).collect()
    }

    /// Parse and execute a command string
    pub fn parse_and_execute(
        &self,
        input: &str,
        ctx: &mut CommandContext,
    ) -> Result<Option<CommandResult>, CliError> {
        let input = input.trim();

        // Must start with /
        if !input.starts_with('/') {
            return Ok(None);
        }

        let input = &input[1..]; // Remove /
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd_name = parts[0].to_lowercase();
        let args = parts.get(1).copied().unwrap_or("");

        // Find command by name or check if command handles it
        for command in self.commands.values() {
            if command.name() == cmd_name || command.aliases().contains(&cmd_name.as_str()) {
                return Ok(Some(command.execute(args, ctx)?));
            }
        }

        // Unknown command
        ctx.add_system_message(format!("Unknown command: /{}", cmd_name));
        ctx.add_system_message("Type /help for available commands".to_string());
        Ok(Some(CommandResult::Continue))
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_registry_creation() {
        let registry = CommandRegistry::new();
        assert_eq!(registry.list_commands().len(), 0);
    }

    #[test]
    fn test_command_registry_default() {
        let registry = CommandRegistry::default();
        assert_eq!(registry.list_commands().len(), 0);
    }
}
