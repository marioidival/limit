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
    /// Reusable buffer for file matches (avoids allocations)
    matches_buffer: Vec<FileMatchData>,
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
            matches_buffer: Vec::with_capacity(64),
        }
    }

    /// Check if autocomplete is currently active
    #[inline]
    pub fn is_active(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.is_active)
    }

    /// Get current autocomplete state
    #[inline]
    pub fn state(&self) -> Option<&AutocompleteState> {
        self.state.as_ref()
    }

    /// Get mutable autocomplete state
    #[inline]
    pub fn state_mut(&mut self) -> Option<&mut AutocompleteState> {
        self.state.as_mut()
    }

    /// Activate autocomplete at the given trigger position
    pub fn activate(&mut self, trigger_pos: usize) {
        let matches = self.get_matches("");

        self.state = Some(AutocompleteState {
            is_active: true,
            query: String::with_capacity(64),
            trigger_pos,
            matches,
            selected_index: 0,
        });

        tracing::debug!("Activated autocomplete at pos {}", trigger_pos);
    }

    /// Deactivate autocomplete
    #[inline]
    pub fn deactivate(&mut self) {
        self.state = None;
    }

    /// Update the query and refresh matches
    pub fn update_query(&mut self, query: &str) {
        if let Some(ref mut state) = self.state {
            // First update query
            state.query.clear();
            state.query.push_str(query);
            
            // Get matches separately to avoid borrow conflicts
            let matches = self.get_matches(query);
            
            // Update state
            if let Some(ref mut state) = self.state {
                state.matches = matches;
                state.selected_index = 0;
            }
        }
    }

    /// Add a character to the query (avoids String allocation)
    pub fn append_char(&mut self, c: char) {
        if let Some(ref mut state) = self.state {
            state.query.push(c);
            let query = state.query.clone();
            
            let matches = self.get_matches(&query);
            
            if let Some(ref mut state) = self.state {
                state.matches = matches;
                state.selected_index = 0;
            }
        }
    }

    /// Remove last character from query
    pub fn backspace(&mut self) -> bool {
        let should_close = self.state.as_ref().map(|s| s.query.is_empty()).unwrap_or(false);

        if should_close {
            return true;
        }

        if let Some(ref mut state) = self.state {
            state.query.pop();
            let query = state.query.clone();
            
            let matches = self.get_matches(&query);
            
            if let Some(ref mut state) = self.state {
                state.matches = matches;
                state.selected_index = 0;
            }
        }
        false
    }

    /// Navigate up in matches
    #[inline]
    pub fn navigate_up(&mut self) {
        if let Some(ref mut state) = self.state {
            state.selected_index = state.selected_index.saturating_sub(1);
        }
    }

    /// Navigate down in matches
    #[inline]
    pub fn navigate_down(&mut self) {
        if let Some(ref mut state) = self.state {
            let max_idx = state.matches.len().saturating_sub(1);
            state.selected_index = state.selected_index.min(max_idx).saturating_add(1).min(max_idx);
        }
    }

    /// Get selected match
    #[inline]
    pub fn selected_match(&self) -> Option<&FileMatchData> {
        self.state.as_ref().and_then(|s| s.matches.get(s.selected_index))
    }

    /// Get trigger position
    #[inline]
    pub fn trigger_pos(&self) -> Option<usize> {
        self.state.as_ref().map(|s| s.trigger_pos)
    }

    /// Accept selected completion and return the text to insert
    pub fn accept_completion(&mut self) -> Option<String> {
        let selected = self.selected_match()?;

        // Pre-allocate with space for path + space
        let mut result = String::with_capacity(selected.path.len() + 1);
        result.push_str(&selected.path);
        result.push(' ');
        
        self.state = None;
        Some(result)
    }

    /// Get file matches for query (reuses internal buffer)
    fn get_matches(&mut self, query: &str) -> Vec<FileMatchData> {
        // Clear buffer for reuse
        self.matches_buffer.clear();
        
        // Scan files and clone to avoid holding borrow
        let files = self.file_finder.scan_files().clone();
        
        // Filter files (now we don't hold the borrow)
        let matches = self.file_finder.filter_files(&files, query);

        self.matches_buffer.extend(matches.into_iter().map(|m| FileMatchData {
            path: m.path.to_string_lossy().to_string(),
            is_dir: m.is_dir,
        }));

        // Clone the buffer to return (matches_buffer is reused next call)
        self.matches_buffer.clone()
    }

    /// Convert to legacy FileAutocompleteState for rendering
    pub fn to_legacy_state(&self) -> Option<crate::tui::FileAutocompleteState> {
        self.state.as_ref().map(|s| crate::tui::FileAutocompleteState {
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

        manager.navigate_up();
        manager.navigate_down();
    }

    #[test]
    fn test_navigation_with_empty_matches() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut manager = FileAutocompleteManager::new(dir);

        manager.activate(0);
        manager.update_query("zzzzzzz_nonexistent_file_xyz");

        manager.navigate_up();
        manager.navigate_down();

        let has_selection = manager.selected_match().is_some();
        let match_count = manager.state().map(|s| s.matches.len()).unwrap_or(0);
        if match_count == 0 {
            assert!(!has_selection, "Should have no selection when no matches");
        }
    }

    #[test]
    fn test_accept_completion() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut manager = FileAutocompleteManager::new(dir);

        manager.activate(0);

        if manager.selected_match().is_some() {
            let result = manager.accept_completion();
            assert!(result.is_some());
            assert!(!manager.is_active(), "Should deactivate after accepting");
        }
    }

    #[test]
    fn test_accept_completion_empty() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut manager = FileAutocompleteManager::new(dir);

        manager.activate(0);
        manager.update_query("zzzzzzz_nonexistent_file_xyz");

        let result = manager.accept_completion();
        assert!(result.is_none() || !manager.is_active());
    }

    #[test]
    fn test_backspace_states() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut manager = FileAutocompleteManager::new(dir);

        let should_close = manager.backspace();
        assert!(!should_close);

        manager.activate(0);

        let should_close = manager.backspace();
        assert!(should_close, "Should close when query is empty");

        manager.append_char('C');
        assert!(!manager.backspace(), "Should not close when query has content");

        let state = manager.state().unwrap();
        assert_eq!(state.query, "");
    }

    #[test]
    fn test_to_legacy_state() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let manager = FileAutocompleteManager::new(dir);

        assert!(manager.to_legacy_state().is_none());

        let mut manager = manager;
        manager.activate(5);

        let legacy = manager.to_legacy_state();
        assert!(legacy.is_some());

        let legacy = legacy.unwrap();
        assert!(legacy.is_active);
        assert_eq!(legacy.query, "");
        assert_eq!(legacy.trigger_pos, 5);
        assert_eq!(legacy.selected_index, 0);
    }

    #[test]
    fn test_update_query() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut manager = FileAutocompleteManager::new(dir);

        manager.activate(0);
        manager.update_query("Cargo");

        let state = manager.state().unwrap();
        assert_eq!(state.query, "Cargo");
        assert_eq!(state.selected_index, 0, "Should reset selection on query update");
    }

    #[test]
    fn test_trigger_pos() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut manager = FileAutocompleteManager::new(dir);

        assert_eq!(manager.trigger_pos(), None);

        manager.activate(10);
        assert_eq!(manager.trigger_pos(), Some(10));

        manager.deactivate();
        assert_eq!(manager.trigger_pos(), None);
    }

    #[test]
    fn test_navigation_bounds() {
        let dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut manager = FileAutocompleteManager::new(dir);

        manager.activate(0);

        let match_count = manager.state().map(|s| s.matches.len()).unwrap_or(0);

        if match_count > 0 {
            manager.navigate_up();
            assert_eq!(manager.state().unwrap().selected_index, 0);

            for _ in 0..match_count {
                manager.navigate_down();
            }

            let final_index = manager.state().unwrap().selected_index;
            assert!(final_index < match_count || match_count == 0);
        }
    }
}
