//! Team history tracking for logging and debugging.
//!
//! Provides a timeline of events across all agents in a team.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A single event in the team timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamEvent {
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Which role produced this event (PM, TL, Jr).
    pub role: String,
    /// What kind of action was performed.
    pub action: String,
    /// The content / output of the action.
    pub content: String,
}

impl fmt::Display for TeamEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.role, self.action, self.content)
    }
}

/// Append-only log of team events.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamHistory {
    events: Vec<TeamEvent>,
}

impl TeamHistory {
    /// Create an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an event.
    pub fn add_event(&mut self, event: TeamEvent) {
        self.events.push(event);
    }

    /// Read-only access to the event list.
    pub fn events(&self) -> &[TeamEvent] {
        &self.events
    }

    /// Number of recorded events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Remove all events.
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_history_empty() {
        let history = TeamHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_team_history_add_event() {
        let mut history = TeamHistory::new();
        history.add_event(TeamEvent {
            timestamp: Utc::now(),
            role: "PM".to_string(),
            action: "analysis".to_string(),
            content: "Test analysis".to_string(),
        });
        assert_eq!(history.len(), 1);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_team_history_clear() {
        let mut history = TeamHistory::new();
        history.add_event(TeamEvent {
            timestamp: Utc::now(),
            role: "PM".to_string(),
            action: "test".to_string(),
            content: "data".to_string(),
        });
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn test_team_event_display() {
        let event = TeamEvent {
            timestamp: Utc::now(),
            role: "TL".to_string(),
            action: "plan".to_string(),
            content: "do stuff".to_string(),
        };
        let display = format!("{}", event);
        assert!(display.contains("[TL]"));
        assert!(display.contains("plan"));
        assert!(display.contains("do stuff"));
    }
}
