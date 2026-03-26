//! # limit-cli
//!
//! AI-powered terminal coding assistant with TUI interface.
//!
//! This crate provides the main entry point for the Limit AI coding assistant.
//! It includes session management, tool execution, and TUI interface.
//!
//! ## Features
//!
//! - **Multi-provider LLM**: Anthropic Claude, OpenAI GPT, z.ai GLM, and local models
//! - **18 built-in tools**: File I/O, bash execution, git operations, code analysis
//! - **Session persistence**: Auto-save and restore conversations
//! - **File autocomplete**: Type `@` to quickly reference files
//!
//! ## Usage
//!
//! ```bash
//! lim
//! ```
//!
//! ## Modules
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`agent_bridge`] | Bridge between LLM agent and CLI |
//! | [`session`] | Session management and persistence |
//! | [`tools`] | Built-in tool implementations |
//! | [`tui`] | Terminal UI components |
//! | [`syntax`] | Syntax highlighting |
//! | [`file_finder`] | File autocomplete with fuzzy matching |
//! | [`session_share`] | Export sessions to various formats |

pub mod agent_bridge;
pub mod clipboard;
pub mod clipboard_paste;
pub mod clipboard_text;
pub mod session_share;
pub mod session_tree;
pub mod system_prompt;

pub use session_tree::{
    generate_entry_id, SerializableMessage, SessionEntry, SessionEntryType, SessionTree,
};

pub mod error;
pub mod file_finder;
pub mod logging;
pub mod session;
pub mod syntax;
pub mod tools;
pub mod tui;
pub mod tui_bridge;

pub use agent_bridge::{AgentBridge, AgentEvent};
pub use error::CliError;
pub use file_finder::{FileFinder, FileMatch};
pub use logging::init_logging;
pub use session::SessionManager;
pub use session_share::{ExportFormat, SessionExport, SessionShare};
pub use syntax::SyntaxHighlighter;
pub use tui::{FileAutocompleteState, TuiState};
pub use tui_bridge::{TuiApp, TuiBridge};
