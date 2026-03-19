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
    /// Severity level for this event.
    #[serde(default)]
    pub level: EventLevel,
}

/// Severity of a team event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EventLevel {
    /// Informational event (default).
    #[default]
    Info,
    /// Warning (non-critical issue).
    Warn,
    /// Error (failure that was recovered or not).
    Error,
}

impl fmt::Display for EventLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warn => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
        }
    }
}

impl fmt::Display for TeamEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}][{}] {}: {}",
            self.level, self.role, self.action, self.content
        )
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

    /// Append an event (defaults to [`EventLevel::Info`]).
    pub fn add_event(&mut self, event: TeamEvent) {
        self.events.push(event);
    }

    /// Append a warning event.
    pub fn add_warn(&mut self, role: &str, action: &str, content: &str) {
        self.add_event(TeamEvent {
            timestamp: Utc::now(),
            role: role.to_string(),
            action: action.to_string(),
            content: content.to_string(),
            level: EventLevel::Warn,
        });
    }

    /// Append an error event.
    pub fn add_error(&mut self, role: &str, action: &str, content: &str) {
        self.add_event(TeamEvent {
            timestamp: Utc::now(),
            role: role.to_string(),
            action: action.to_string(),
            content: content.to_string(),
            level: EventLevel::Error,
        });
    }

    /// Number of error-level events.
    pub fn error_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.level == EventLevel::Error)
            .count()
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
            level: EventLevel::Info,
        });
        assert_eq!(history.len(), 1);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_team_history_add_warn() {
        let mut history = TeamHistory::new();
        history.add_warn("Jr", "execution", "timeout on task 3");
        assert_eq!(history.len(), 1);
        assert_eq!(history.events()[0].level, EventLevel::Warn);
    }

    #[test]
    fn test_team_history_add_error() {
        let mut history = TeamHistory::new();
        history.add_error("Jr", "execution", "tool not found");
        assert_eq!(history.len(), 1);
        assert_eq!(history.error_count(), 1);
    }

    #[test]
    fn test_team_history_error_count() {
        let mut history = TeamHistory::new();
        history.add_event(TeamEvent {
            timestamp: Utc::now(),
            role: "PM".to_string(),
            action: "analysis".to_string(),
            content: "ok".to_string(),
            level: EventLevel::Info,
        });
        history.add_warn("TL", "plan", "risky approach");
        history.add_error("Jr", "execution", "failed");
        assert_eq!(history.len(), 3);
        assert_eq!(history.error_count(), 1);
    }

    #[test]
    fn test_team_history_clear() {
        let mut history = TeamHistory::new();
        history.add_event(TeamEvent {
            timestamp: Utc::now(),
            role: "PM".to_string(),
            action: "test".to_string(),
            content: "data".to_string(),
            level: EventLevel::default(),
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
            level: EventLevel::Info,
        };
        let display = format!("{}", event);
        assert!(display.contains("[TL]"));
        assert!(display.contains("plan"));
        assert!(display.contains("do stuff"));
        assert!(display.contains("[INFO]"));
    }

    #[test]
    fn test_event_level_display() {
        assert_eq!(format!("{}", EventLevel::Info), "INFO");
        assert_eq!(format!("{}", EventLevel::Warn), "WARN");
        assert_eq!(format!("{}", EventLevel::Error), "ERROR");
    }

    #[test]
    fn test_event_level_default() {
        assert_eq!(EventLevel::default(), EventLevel::Info);
    }
}
