//! TUI state types and constants
//!
//! This module contains the core state types used by the TUI system.

/// Maximum paste size to prevent memory issues (100KB)
pub const MAX_PASTE_SIZE: usize = 100 * 1024;

/// TUI state for displaying agent events
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TuiState {
    #[default]
    Idle,
    Thinking,
}

/// State for file autocomplete popup
#[derive(Debug, Clone)]
pub struct FileAutocompleteState {
    /// Whether autocomplete popup is visible
    pub is_active: bool,
    /// Query typed after @ (e.g., "Cargo" in "@Cargo")
    pub query: String,
    /// Start position of @ in input_text
    pub trigger_pos: usize,
    /// List of matching files
    pub matches: Vec<limit_tui::components::FileMatchData>,
    /// Currently selected index in matches
    pub selected_index: usize,
}

impl Default for FileAutocompleteState {
    fn default() -> Self {
        Self {
            is_active: false,
            query: String::new(),
            trigger_pos: 0,
            matches: Vec::new(),
            selected_index: 0,
        }
    }
}

/// Debug log to file (bypasses tracing)
/// 
/// **DEPRECATED**: This function will be removed in favor of proper tracing.
/// Use `tracing::debug!` instead.
#[deprecated(note = "Use tracing::debug! instead")]
pub fn debug_log(msg: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_state_default() {
        let state = TuiState::default();
        assert_eq!(state, TuiState::Idle);
    }

    #[test]
    fn test_file_autocomplete_default() {
        let state = FileAutocompleteState::default();
        assert_eq!(state.is_active, false);
        assert_eq!(state.query, "");
        assert_eq!(state.trigger_pos, 0);
        assert_eq!(state.matches.len(), 0);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_max_paste_size() {
        assert_eq!(MAX_PASTE_SIZE, 100 * 1024);
    }
}
