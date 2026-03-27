// Widget for displaying pending input messages
// Two-tier queue preview: pending steers + queued messages

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

const MAX_PREVIEW_LINES: usize = 3;
const MAX_PREVIEW_WIDTH: usize = 60;

#[derive(Debug, Clone)]
pub struct PreviewConfig {
    pub edit_binding: String,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            edit_binding: "Alt+↑".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PendingInputPreview {
    pub pending_steers: Vec<String>,
    pub queued_messages: Vec<String>,
    pub interrupt_in_progress: bool,
    config: PreviewConfig,
}

impl PendingInputPreview {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: PreviewConfig) -> Self {
        Self {
            pending_steers: Vec::new(),
            queued_messages: Vec::new(),
            interrupt_in_progress: false,
            config,
        }
    }

    pub fn edit_binding_hint(&self) -> &str {
        &self.config.edit_binding
    }

    pub fn has_messages(&self) -> bool {
        !self.pending_steers.is_empty() || !self.queued_messages.is_empty()
    }

    pub fn set_data(
        &mut self,
        queued_messages: Vec<String>,
        pending_steers: Vec<String>,
        interrupt_in_progress: bool,
    ) {
        self.queued_messages = queued_messages;
        self.pending_steers = pending_steers;
        self.interrupt_in_progress = interrupt_in_progress;
    }

    fn truncate_preview(text: &str) -> String {
        let lines: Vec<&str> = text.lines().take(MAX_PREVIEW_LINES).collect();
        let truncated = lines.join("\n");
        if text.lines().count() > MAX_PREVIEW_LINES {
            format!("{}…", truncated)
        } else if truncated.chars().count() > MAX_PREVIEW_WIDTH {
            format!(
                "{}…",
                truncated
                    .chars()
                    .take(MAX_PREVIEW_WIDTH)
                    .collect::<String>()
            )
        } else {
            truncated
        }
    }

    fn render_steers(&self, lines: &mut Vec<Line>) {
        if self.pending_steers.is_empty() {
            return;
        }

        let count = self.pending_steers.len();
        let header = if self.interrupt_in_progress {
            format!("⏳ Interrupting... ({} pending)", count)
        } else {
            format!("⏳ Pending ({}) [ESC: send now]", count)
        };

        lines.push(Line::from(Span::styled(
            header,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        for steer in &self.pending_steers {
            let preview = Self::truncate_preview(steer);
            let icon = if self.interrupt_in_progress {
                "⚡"
            } else {
                "◯"
            };
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(icon, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::styled(preview, Style::default().fg(Color::Gray)),
            ]));
        }
    }

    fn render_queued(&self, lines: &mut Vec<Line>) {
        if self.queued_messages.is_empty() {
            return;
        }

        if !self.pending_steers.is_empty() {
            lines.push(Line::raw(""));
        }

        let count = self.queued_messages.len();
        lines.push(Line::from(Span::styled(
            format!("📝 Queued ({}) [{}: edit]", count, self.config.edit_binding),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        for msg in &self.queued_messages {
            let preview = Self::truncate_preview(msg);
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("→", Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(preview, Style::default().fg(Color::Gray)),
            ]));
        }
    }
}

impl Widget for &PendingInputPreview {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.has_messages() || area.width < 4 {
            return;
        }

        let mut lines = Vec::new();
        self.render_steers(&mut lines);
        self.render_queued(&mut lines);

        for (y, line) in lines.iter().enumerate() {
            if y >= area.height as usize {
                break;
            }
            let y = area.y + y as u16;
            if y >= buf.area.height {
                break;
            }

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

impl Widget for PendingInputPreview {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf)
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
    fn test_truncate_preview() {
        let text = "Line1\nLine2\nLine3\nLine4\nLine5";
        let truncated = PendingInputPreview::truncate_preview(text);
        assert!(truncated.contains("Line1"));
        assert!(truncated.contains("Line2"));
        assert!(truncated.contains("Line3"));
        assert!(truncated.contains("…"));
    }

    #[test]
    fn test_set_data() {
        let mut widget = PendingInputPreview::new();
        widget.set_data(
            vec!["queued".to_string()],
            vec!["pending".to_string()],
            true,
        );
        assert_eq!(widget.queued_messages.len(), 1);
        assert_eq!(widget.pending_steers.len(), 1);
        assert!(widget.interrupt_in_progress);
    }
}
