pub mod agent_bridge;
pub mod clipboard;
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
