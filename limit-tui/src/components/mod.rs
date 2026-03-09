// Component modules for limit-tui
//
// This module contains reusable UI components built on top of ratatui.

pub mod activity;
pub mod chat;
pub mod diff;
pub mod progress;
pub mod prompt;

pub use activity::ActivityFeed;
pub use chat::{ChatView, Message, Role};
pub use diff::{parse_diff, DiffLine, DiffType, DiffView};
pub use progress::{ProgressBar, Spinner};
pub use prompt::{InputPrompt, InputResult, SelectPrompt, SelectResult};
