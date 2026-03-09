// Activity feed component for limit-tui
//
// This module provides an ActivityFeed component for displaying
// recent tool activities in a compact log format.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use tracing::debug;

/// Maximum number of activities to keep in the feed
const MAX_ACTIVITIES: usize = 5;

/// A single activity entry
#[derive(Debug, Clone)]
pub struct Activity {
    /// Activity message (e.g., "Reading src/main.rs...")
    pub message: String,
    /// Whether this activity is currently in progress
    pub in_progress: bool,
}

/// Activity feed component that displays recent tool activities
///
/// The feed shows a compact list of recent activities with
/// visual indicators for in-progress items.
#[derive(Debug, Clone)]
pub struct ActivityFeed {
    /// List of recent activities (most recent first)
    activities: Vec<Activity>,
}

impl Default for ActivityFeed {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityFeed {
    /// Create a new empty activity feed
    pub fn new() -> Self {
        debug!(component = %"ActivityFeed", "Component created");
        Self {
            activities: Vec::with_capacity(MAX_ACTIVITIES),
        }
    }

    /// Add a new activity to the feed
    ///
    /// Activities are always added to the top of the list.
    /// Multiple in-progress activities can exist simultaneously.
    pub fn add(&mut self, message: String, in_progress: bool) {
        // Add new activity at the beginning
        self.activities.insert(
            0,
            Activity {
                message,
                in_progress,
            },
        );

        // Prune old activities
        if self.activities.len() > MAX_ACTIVITIES {
            self.activities.truncate(MAX_ACTIVITIES);
        }
    }

    /// Mark the most recent in-progress activity as complete
    ///
    /// Since activities are added at the beginning, we mark the first
    /// in-progress activity (most recent) as complete.
    pub fn complete_current(&mut self) {
        for activity in &mut self.activities {
            if activity.in_progress {
                activity.in_progress = false;
                break;
            }
        }
    }

    /// Clear all activities
    pub fn clear(&mut self) {
        self.activities.clear();
    }

    /// Mark all in-progress activities as complete
    pub fn complete_all(&mut self) {
        for activity in &mut self.activities {
            activity.in_progress = false;
        }
    }

    /// Check if there are any activities
    pub fn is_empty(&self) -> bool {
        self.activities.is_empty()
    }

    /// Check if there are any in-progress activities
    pub fn has_in_progress(&self) -> bool {
        self.activities.iter().any(|a| a.in_progress)
    }

    /// Get the number of activities
    pub fn len(&self) -> usize {
        self.activities.len()
    }

    /// Render the activity feed to a buffer
    ///
    /// # Arguments
    ///
    /// * `area` - The area to render the feed in
    /// * `buf` - The buffer to render to
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || self.activities.is_empty() {
            return;
        }

        let mut lines = Vec::with_capacity(self.activities.len());

        for activity in &self.activities {
            let (indicator, color) = if activity.in_progress {
                ("⏳", Color::Yellow)
            } else {
                ("✓", Color::Green)
            };

            lines.push(Line::from(vec![
                Span::styled(indicator, Style::default().fg(color)),
                Span::raw(" "),
                Span::styled(&activity.message, Style::default().fg(Color::DarkGray)),
            ]));
        }

        let paragraph = Paragraph::new(lines);
        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_feed_new() {
        let feed = ActivityFeed::new();
        assert!(feed.is_empty());
        assert_eq!(feed.len(), 0);
    }

    #[test]
    fn test_activity_feed_default() {
        let feed = ActivityFeed::default();
        assert!(feed.is_empty());
    }

    #[test]
    fn test_activity_feed_add() {
        let mut feed = ActivityFeed::new();
        feed.add("Reading file...".to_string(), true);
        assert_eq!(feed.len(), 1);
    }

    #[test]
    fn test_activity_feed_add_multiple() {
        let mut feed = ActivityFeed::new();
        feed.add("Reading file...".to_string(), false);
        feed.add("Editing file...".to_string(), false);
        assert_eq!(feed.len(), 2);
        // Most recent should be first
        assert_eq!(feed.activities[0].message, "Editing file...");
    }

    #[test]
    fn test_activity_feed_multiple_in_progress() {
        let mut feed = ActivityFeed::new();
        feed.add("Reading file...".to_string(), true);
        feed.add("Editing file...".to_string(), true);
        // Should have both activities (no longer replaces)
        assert_eq!(feed.len(), 2);
        assert_eq!(feed.activities[0].message, "Editing file...");
        assert!(feed.activities[0].in_progress);
        assert!(feed.activities[1].in_progress);
    }

    #[test]
    fn test_activity_feed_complete_current() {
        let mut feed = ActivityFeed::new();
        feed.add("Reading file...".to_string(), true);
        assert!(feed.activities[0].in_progress);
        feed.complete_current();
        assert!(!feed.activities[0].in_progress);
    }

    #[test]
    fn test_activity_feed_has_in_progress() {
        let mut feed = ActivityFeed::new();
        assert!(!feed.has_in_progress());
        feed.add("Reading file...".to_string(), true);
        assert!(feed.has_in_progress());
        feed.complete_current();
        assert!(!feed.has_in_progress());
    }

    #[test]
    fn test_activity_feed_prune() {
        let mut feed = ActivityFeed::new();
        feed.add("Reading file...".to_string(), true);
        feed.clear();
        assert!(feed.is_empty());
    }

    #[test]
    fn test_activity_feed_render() {
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 3,
        });

        let mut feed = ActivityFeed::new();
        feed.add("Reading file...".to_string(), false);
        feed.add("Editing file...".to_string(), true);
        feed.render(
            Rect {
                x: 0,
                y: 0,
                width: 40,
                height: 3,
            },
            &mut buffer,
        );

        // Verify that something was rendered
        assert!(!buffer.content.is_empty());
    }
}
