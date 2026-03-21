//! # limit-cli
//!
//! AI-powered terminal coding assistant with REPL and TUI interfaces.
//!
//! This crate provides the main entry point for the Limit AI coding assistant.
//! It includes session management, tool execution, markdown rendering, and
//! both TUI and REPL interfaces.
//!
//! ## Features
//!
//! - **Multi-provider LLM**: Anthropic Claude, OpenAI GPT, z.ai GLM, and local models
//! - **18 built-in tools**: File I/O, bash execution, git operations, code analysis
//! - **Session persistence**: Auto-save and restore conversations
//! - **Markdown rendering**: Rich formatting with syntax highlighting
//! - **File autocomplete**: Type `@` to quickly reference files
//!
//! ## Usage
//!
//! ```bash
//! # TUI mode (default)
//! lim
//!
//! # REPL mode
//! lim --no-tui
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
//! | [`render`] | Markdown rendering |
//! | [`syntax`] | Syntax highlighting |
//! | [`file_finder`] | File autocomplete with fuzzy matching |
//! | [`session_share`] | Export sessions to various formats |

pub mod agent_bridge;
pub mod clipboard;
pub mod project_settings;
pub mod session_share;
pub mod system_prompt;

pub mod error;
pub mod file_finder;
pub mod logging;
pub mod render;
pub mod session;
pub mod syntax;
pub mod tools;
pub mod tui;
pub mod tui_bridge;

pub use agent_bridge::{AgentBridge, AgentEvent};
pub use error::CliError;
pub use file_finder::{FileFinder, FileMatch};
pub use logging::init_logging;
pub use render::MarkdownRenderer;
pub use session::SessionManager;
pub use session_share::{ExportFormat, SessionExport, SessionShare};
pub use syntax::SyntaxHighlighter;
pub use tui::{FileAutocompleteState, TuiState};
// Legacy re-exports from tui_bridge module (deprecated, will be removed)
pub use tui_bridge::{TuiApp, TuiBridge};
