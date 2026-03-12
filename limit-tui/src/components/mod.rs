// Component modules for limit-tui
//
// This module contains reusable UI components built on top of ratatui.

pub mod activity;
pub mod chat;
pub mod diff;
pub mod file_autocomplete;
pub mod progress;
pub mod prompt;

pub use activity::ActivityFeed;
pub use chat::{ChatView, Message, Role};
pub use diff::{parse_diff, DiffLine, DiffType, DiffView};
pub use file_autocomplete::{calculate_popup_area, FileAutocompleteWidget, FileMatchData};
pub use progress::{ProgressBar, Spinner};
pub use prompt::{InputPrompt, InputResult, SelectPrompt, SelectResult};
