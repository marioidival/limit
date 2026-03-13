//! File autocomplete manager for TUI
//!
//! Manages file path autocomplete with @ prefix.

use crate::file_finder::FileFinder;
use limit_tui::components::FileMatchData;
use std::path::PathBuf;

/// Manages file autocomplete state and operations
pub struct FileAutocompleteManager {
    /// File finder instance
    file_finder: FileFinder,
    /// Current autocomplete state
    state: Option<AutocompleteState>,
}

/// State for active autocomplete session
#[derive(Debug, Clone, Default)]
pub struct AutocompleteState {
    /// Whether autocomplete is active
    pub is_active: bool,
    /// Query typed after @
    pub query: String,
    /// Position of @ trigger in input
    pub trigger_pos: usize,
    /// Matching files
    pub matches: Vec<FileMatchData>,
    /// Currently selected index
    pub selected_index: usize,
}

impl FileAutocompleteManager {
    /// Create a new autocomplete manager
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            file_finder: FileFinder::new(working_dir),
            state: None,
        }
    }

    /// Check if autocomplete is currently active
    pub fn is_active(&self) -> bool {
        self.state.as_ref().map(|s| s.is_active).unwrap_or(false)
    }

    /// Get current autocomplete state
    pub fn state(&self) -> Option<&AutocompleteState> {
        self.state.as_ref()
    }

    /// Get mutable autocomplete state
    pub fn state_mut(&mut self) -> Option<&mut AutocompleteState> {
        self.state.as_mut()
    }

    /// Activate autocomplete at the given trigger position
    pub fn activate(&mut self, trigger_pos: usize) {
        let matches = self.get_matches("");

        self.state = Some(AutocompleteState {
            is_active: true,
            query: String::new(),
            trigger_pos,
            matches,
            selected_index: 0,
        });

        tracing::debug!("Activated autocomplete at pos {}", trigger_pos);
    }

    /// Deactivate autocomplete
    pub fn deactivate(&mut self) {
        self.state = None;
    }

    /// Update the query and refresh matches
    pub fn update_query(&mut self, query: &str) {
        let matches = self.get_matches(query);
        if let Some(ref mut state) = self.state {
            state.query = query.to_string();
            state.matches = matches;
            state.selected_index = 0;
        }
    }

    /// Add a character to the query
    pub fn append_char(&mut self, c: char) {
        // First get the new query
        let new_query = self.state.as_ref().map(|s| {
            let mut q = s.query.clone();
            q.push(c);
            q
        });

        if let Some(query) = new_query {
            let matches = self.get_matches(&query);
            if let Some(ref mut state) = self.state {
                state.query = query;
                state.matches = matches;
                state.selected_index = 0;
            }
        }
    }

    /// Remove last character from query
    pub fn backspace(&mut self) -> bool {
        // First check if we should close
        let should_close = self
            .state
            .as_ref()
            .map(|s| s.query.is_empty())
            .unwrap_or(false);

        if should_close {
            return true;
        }

        // Get the new query
        let new_query = self.state.as_ref().map(|s| {
            let mut q = s.query.clone();
            q.pop();
            q
        });

        if let Some(query) = new_query {
            let matches = self.get_matches(&query);
            if let Some(ref mut state) = self.state {
                state.query = query;
                state.matches = matches;
                state.selected_index = 0;
            }
        }
        false
    }

    /// Navigate up in matches
    pub fn navigate_up(&mut self) {
        if let Some(ref mut state) = self.state {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
        }
    }

    /// Navigate down in matches
    pub fn navigate_down(&mut self) {
        if let Some(ref mut state) = self.state {
            if state.selected_index + 1 < state.matches.len() {
                state.selected_index += 1;
            }
        }
    }

    /// Get selected match
    pub fn selected_match(&self) -> Option<&FileMatchData> {
        self.state
            .as_ref()
            .and_then(|s| s.matches.get(s.selected_index))
    }

    /// Accept selected completion and return the text to insert
    pub fn accept_completion(&mut self) -> Option<String> {
        let selected = self.selected_match().cloned();

        if let Some(selected) = selected {
            // Return path with trailing space
            let result = format!("{} ", selected.path);
            self.state = None;
            Some(result)
        } else {
            None
        }
    }

    /// Get file matches for query
    fn get_matches(&mut self, query: &str) -> Vec<FileMatchData> {
        let files = self.file_finder.scan_files().clone();
        let matches = self.file_finder.filter_files(&files, query);

        matches
            .into_iter()
            .map(|m| FileMatchData {
                path: m.path.to_string_lossy().to_string(),
                is_dir: m.is_dir,
            })
            .collect()
    }

    /// Get trigger position
    pub fn trigger_pos(&self) -> Option<usize> {
        self.state.as_ref().map(|s| s.trigger_pos)
    }

    /// Convert to legacy FileAutocompleteState for rendering
    pub fn to_legacy_state(&self) -> Option<crate::tui::FileAutocompleteState> {
        self.state
            .as_ref()
            .map(|s| crate::tui::FileAutocompleteState {
                is_active: s.is_active,
                query: s.query.clone(),
                trigger_pos: s.trigger_pos,
                matches: s.matches.clone(),
                selected_index: s.selected_index,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_autocomplete_manager_creation() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let manager = FileAutocompleteManager::new(dir);
        assert!(!manager.is_active());
    }

    #[test]
    fn test_activate_deactivate() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut manager = FileAutocompleteManager::new(dir);

        assert!(!manager.is_active());

        manager.activate(0);
        assert!(manager.is_active());

        manager.deactivate();
        assert!(!manager.is_active());
    }

    #[test]
    fn test_navigation() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut manager = FileAutocompleteManager::new(dir);

        manager.activate(0);

        // Should not crash if no matches
        manager.navigate_up();
        manager.navigate_down();
    }
}
