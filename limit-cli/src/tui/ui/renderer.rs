//! UI Renderer for TUI components
//!
//! Handles rendering of chat view, status bar, input area, and popups.

use crate::tui::FileAutocompleteState;
use crate::TuiBridge;
use limit_tui::components::{calculate_popup_area, ChatView, FileAutocompleteWidget};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::sync::{Arc, Mutex};

/// UI Renderer for drawing TUI components
pub struct UiRenderer<'a> {
    frame: &'a mut Frame<'a>,
    area: Rect,
}

impl<'a> UiRenderer<'a> {
    /// Create a new UI renderer
    pub fn new(frame: &'a mut Frame<'a>, area: Rect) -> Self {
        Self { frame, area }
    }

    /// Render the complete TUI interface
    pub fn render(
        &mut self,
        chat_view: &Arc<Mutex<ChatView>>,
        input_text: &str,
        cursor_pos: usize,
        status_message: &str,
        status_is_error: bool,
        cursor_blink_state: bool,
        tui_bridge: &TuiBridge,
        file_autocomplete: &Option<FileAutocompleteState>,
    ) {
        // Calculate layout
        let activity_count = tui_bridge.activity_feed().lock().unwrap().len();
        let activity_height = if activity_count > 0 {
            (activity_count as u16).min(3)
        } else {
            0
        };

        // Build constraints
        let mut constraints = vec![Constraint::Percentage(90)];
        if activity_height > 0 {
            constraints.push(Constraint::Length(activity_height));
        }
        constraints.push(Constraint::Length(1)); // Status bar
        constraints.push(Constraint::Length(6)); // Input area

        // Split screen
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints.as_slice())
            .split(self.area);

        let mut chunk_idx = 0;

        // Render chat view
        self.render_chat_view(&chunks[chunk_idx], chat_view, tui_bridge);
        chunk_idx += 1;

        // Render activity feed if present
        if activity_height > 0 {
            self.render_activity_feed(&chunks[chunk_idx], tui_bridge);
            chunk_idx += 1;
        }

        // Render status bar
        self.render_status_bar(&chunks[chunk_idx], status_message, status_is_error);
        chunk_idx += 1;

        // Render input area
        self.render_input_area(
            &chunks[chunk_idx],
            input_text,
            cursor_pos,
            cursor_blink_state,
        );

        // Render autocomplete popup
        if let Some(ref ac) = file_autocomplete {
            if ac.is_active && !ac.matches.is_empty() {
                self.render_autocomplete_popup(&chunks[chunk_idx], ac);
            }
        }
    }

    /// Render chat view with border
    fn render_chat_view(
        &mut self,
        area: &Rect,
        chat_view: &Arc<Mutex<ChatView>>,
        tui_bridge: &TuiBridge,
    ) {
        let chat = chat_view.lock().unwrap();
        let total_input = tui_bridge.total_input_tokens();
        let total_output = tui_bridge.total_output_tokens();
        let title = format!(" Chat (↑{} ↓{}) ", total_input, total_output);

        let chat_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        self.frame
            .render_widget(&*chat, chat_block.inner(*area));
        self.frame.render_widget(chat_block, *area);
    }

    /// Render activity feed
    fn render_activity_feed(&mut self, area: &Rect, tui_bridge: &TuiBridge) {
        let activity_feed = tui_bridge.activity_feed().lock().unwrap();
        let activity_block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::Reset));

        let activity_inner = activity_block.inner(*area);
        self.frame.render_widget(activity_block, *area);
        activity_feed.render(activity_inner, self.frame.buffer_mut());
    }

    /// Render status bar
    fn render_status_bar(
        &mut self,
        area: &Rect,
        status_message: &str,
        status_is_error: bool,
    ) {
        let status_style = if status_is_error {
            Style::default().fg(Color::Red).bg(Color::Reset)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let status = Paragraph::new(Line::from(vec![
            Span::styled(" ● ", Style::default().fg(Color::Green)),
            Span::styled(status_message, status_style),
        ]));

        self.frame.render_widget(status, *area);
    }

    /// Render input area with border
    fn render_input_area(
        &mut self,
        area: &Rect,
        input_text: &str,
        cursor_pos: usize,
        cursor_blink_state: bool,
    ) {
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Input (Esc or /exit to quit) ")
            .title_style(Style::default().fg(Color::Cyan));

        let input_inner = input_block.inner(*area);
        self.frame.render_widget(input_block, *area);

        // Build input line with cursor
        let input_line = if input_text.is_empty() {
            Line::from(vec![Span::styled(
                "Type your message here...",
                Style::default().fg(Color::DarkGray),
            )])
        } else {
            let (before_cursor, at_cursor, after_cursor) =
                Self::split_text_at_cursor(input_text, cursor_pos);

            let cursor_style = if cursor_blink_state {
                Style::default().bg(Color::White).fg(Color::Black)
            } else {
                Style::default().bg(Color::Reset).fg(Color::Reset)
            };

            Line::from(vec![
                Span::raw(before_cursor),
                Span::styled(at_cursor, cursor_style),
                Span::raw(after_cursor),
            ])
        };

        let input_para = Paragraph::new(input_line).wrap(Wrap { trim: false });
        self.frame.render_widget(input_para, input_inner);
    }

    /// Render autocomplete popup
    fn render_autocomplete_popup(
        &mut self,
        input_area: &Rect,
        autocomplete: &FileAutocompleteState,
    ) {
        let popup_area = calculate_popup_area(*input_area, autocomplete.matches.len());

        let widget = FileAutocompleteWidget::new(
            &autocomplete.matches,
            autocomplete.selected_index,
            &autocomplete.query,
        );

        self.frame.render_widget(widget, popup_area);
    }

    /// Split text at cursor position for rendering
    fn split_text_at_cursor(text: &str, cursor_pos: usize) -> (&str, &str, &str) {
        let before_cursor = &text[..cursor_pos];

        let at_cursor = if cursor_pos < text.len() {
            &text[cursor_pos
                ..cursor_pos
                    + text[cursor_pos..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0)]
        } else {
            " "
        };

        let after_cursor = if cursor_pos < text.len() {
            &text[cursor_pos + at_cursor.len()..]
        } else {
            ""
        };

        (before_cursor, at_cursor, after_cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_text_at_cursor() {
        // Empty text
        let (before, at, after) = UiRenderer::split_text_at_cursor("", 0);
        assert_eq!(before, "");
        assert_eq!(at, " ");
        assert_eq!(after, "");

        // Text with cursor at start
        let (before, at, after) = UiRenderer::split_text_at_cursor("hello", 0);
        assert_eq!(before, "");
        assert_eq!(at, "h");
        assert_eq!(after, "ello");

        // Text with cursor in middle
        let (before, at, after) = UiRenderer::split_text_at_cursor("hello", 2);
        assert_eq!(before, "he");
        assert_eq!(at, "l");
        assert_eq!(after, "lo");

        // Text with cursor at end
        let (before, at, after) = UiRenderer::split_text_at_cursor("hello", 5);
        assert_eq!(before, "hello");
        assert_eq!(at, " ");
        assert_eq!(after, "");

        // UTF-8 text - 'é' is 2 bytes (positions 1-2), 'l' starts at position 3
        let text = "héllo";
        let pos = text.char_indices().nth(2).map(|(i, _)| i).unwrap(); // Position of third char
        let (before, at, after) = UiRenderer::split_text_at_cursor(text, pos);
        assert_eq!(before, "hé");
        assert_eq!(at, "l");
        assert_eq!(after, "lo");
    }
}
