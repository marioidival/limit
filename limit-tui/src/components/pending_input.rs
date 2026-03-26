// Widget for displaying pending input messages
// Based on Codex's pending_input_preview.rs

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use std::borrow::Cow;

/// Maximum lines to show per message preview
const MAX_PREVIEW_LINES: usize = 3;

/// Widget that displays pending input messages above the composer
pub struct PendingInputPreview {
    /// Pending steer messages (submitted to core but not committed)
    pub pending_steers: Vec<String>,
    /// Queued messages (not yet submitted)
    pub queued_messages: Vec<String>,
}

impl Default for PendingInputPreview {
    fn default() -> Self {
        Self::new()
    }
}

impl PendingInputPreview {
    /// Create a new pending input preview widget
    pub fn new() -> Self {
        Self {
            pending_steers: Vec::new(),
            queued_messages: Vec::new(),
        }
    }

    /// Check if there are any messages to display
    pub fn has_messages(&self) -> bool {
        !self.pending_steers.is_empty() || !self.queued_messages.is_empty()
    }

    /// Truncate text to a maximum number of lines
    fn truncate_lines(text: &str, max_lines: usize) -> Vec<&str> {
        let lines: Vec<&str> = text.lines().take(max_lines).collect();
        lines
    }

    /// Render pending steers section
    fn render_steers<'a>(&self, lines: &mut Vec<Line<'a>>, _width: u16) {
        if self.pending_steers.is_empty() {
            return;
        }

        // Header
        lines.push(Line::from(vec![
            Span::styled(
                "Messages to be submitted after next tool call",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (press ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to interrupt and send immediately)",
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        // Render each steer
        for steer in &self.pending_steers {
            let preview_lines = Self::truncate_lines(steer, MAX_PREVIEW_LINES);
            for line in preview_lines {
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(*line, Style::default().fg(Color::Gray)),
                ]));
            }
            if steer.lines().count() > MAX_PREVIEW_LINES {
                lines.push(Line::from(vec![Span::styled(
                    "  ...",
                    Style::default().fg(Color::DarkGray),
                )]));
            }
        }
    }

    /// Render queued messages section
    fn render_queued<'a>(&self, lines: &mut Vec<Line<'a>>, _width: u16) {
        if self.queued_messages.is_empty() {
            return;
        }

        // Header
        lines.push(Line::from(vec![Span::styled(
            "Queued follow-up messages",
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )]));

        // Render each queued message
        for msg in &self.queued_messages {
            let preview_lines = Self::truncate_lines(msg, MAX_PREVIEW_LINES);
            for line in preview_lines {
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(*line, Style::default().fg(Color::Gray)),
                ]));
            }
            if msg.lines().count() > MAX_PREVIEW_LINES {
                lines.push(Line::from(vec![Span::styled(
                    "  ...",
                    Style::default().fg(Color::DarkGray),
                )]));
            }
        }

        // Hint for editing
        lines.push(Line::from(vec![
            Span::styled("    Alt+↑ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "edit last queued message",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }
}

impl Widget for PendingInputPreview {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.has_messages() || area.width < 4 {
            return;
        }

        let mut lines = Vec::new();

        self.render_steers(&mut lines, area.width);
        self.render_queued(&mut lines, area.width);

        // Render lines to buffer
        for (y, line) in lines.iter().enumerate() {
            if y >= area.height as usize {
                break;
            }
            let y = area.y + y as u16;
            if y >= buf.area.height {
                break;
            }

            // Render line
            let x = area.x;
            let max_x = (area.x + area.width).min(buf.area.width);

            for span in line.spans.iter() {
                let span_x = x + span.width() as u16;
                if span_x >= max_x {
                    break;
                }

                for (char_idx, ch) in span.content.chars().enumerate() {
                    let char_x = x + char_idx as u16;
                    if char_x >= max_x {
                        break;
                    }

                    if let Some(cell) = buf.cell_mut((char_x, y)) {
                        cell.set_char(ch);
                        cell.set_style(span.style);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let widget = PendingInputPreview::new();
        assert!(!widget.has_messages());
    }

    #[test]
    fn test_has_messages() {
        let mut widget = PendingInputPreview::new();
        assert!(!widget.has_messages());

        widget.pending_steers.push("Test".to_string());
        assert!(widget.has_messages());

        widget.pending_steers.clear();
        widget.queued_messages.push("Test".to_string());
        assert!(widget.has_messages());
    }

    #[test]
    fn test_truncate_lines() {
        let text = "Line1\nLine2\nLine3\nLine4\nLine5";
        let lines = PendingInputPreview::truncate_lines(text, 3);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Line1");
        assert_eq!(lines[1], "Line2");
        assert_eq!(lines[2], "Line3");
    }

    #[test]
    fn test_truncate_lines_short() {
        let text = "Line1\nLine2";
        let lines = PendingInputPreview::truncate_lines(text, 5);
        assert_eq!(lines.len(), 2);
    }
}
