// Component modules for limit-tui
//
// This module contains reusable UI components built on top of ratatui.

pub mod chat;
pub mod diff;
pub mod progress;
pub mod prompt;

pub use chat::{ChatView, Message, Role};
pub use diff::{DiffLine, DiffType, DiffView, parse_diff};
pub use progress::{ProgressBar, Spinner};
pub use prompt::{InputPrompt, InputResult, SelectPrompt, SelectResult};
