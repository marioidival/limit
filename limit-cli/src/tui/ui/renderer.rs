//! UI Renderer for TUI components
//!
//! Handles rendering of chat view, status bar, input area, and popups.

use crate::tui::bridge::TuiBridge;
use crate::tui::FileAutocompleteState;
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
pub struct UiRenderer;

impl UiRenderer {
    /// Render the complete TUI interface
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        chat_view: &Arc<Mutex<ChatView>>,
        input_text: &str,
        cursor_pos: usize,
        status_message: &str,
        status_is_error: bool,
        cursor_blink_state: bool,
        tui_bridge: &TuiBridge,
        file_autocomplete: &Option<FileAutocompleteState>,
    ) {
        // Calculate activity height
        let activity_count = tui_bridge.activity_feed().lock().unwrap().len();
        let activity_height = (activity_count as u16).min(3);

        // Build constraints (pre-allocated array on stack)
        let constraints = if activity_height == 0 {
            [
                Constraint::Percentage(90),
                Constraint::Length(1),
                Constraint::Length(6),
                Constraint::Length(0),
            ]
        } else {
            [
                Constraint::Percentage(90),
                Constraint::Length(activity_height),
                Constraint::Length(1),
                Constraint::Length(6),
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut chunk_idx = 0;

        // Render chat view
        Self::render_chat_view(frame, &chunks[chunk_idx], chat_view, tui_bridge);
        chunk_idx += 1;

        // Render activity feed if present
        if activity_height > 0 {
            Self::render_activity_feed(frame, &chunks[chunk_idx], tui_bridge);
            chunk_idx += 1;
        }

        // Render status bar
        Self::render_status_bar(frame, &chunks[chunk_idx], status_message, status_is_error);
        chunk_idx += 1;

        // Render input area
        Self::render_input_area(
            frame,
            &chunks[chunk_idx],
            input_text,
            cursor_pos,
            cursor_blink_state,
        );

        // Render autocomplete popup
        if let Some(ref ac) = file_autocomplete {
            if ac.is_active && !ac.matches.is_empty() {
                Self::render_autocomplete_popup(frame, &chunks[chunk_idx], ac);
            }
        }
    }

    /// Render chat view with border
    fn render_chat_view(
        frame: &mut Frame,
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

        frame.render_widget(&*chat, chat_block.inner(*area));
        frame.render_widget(chat_block, *area);
    }

    /// Render activity feed
    fn render_activity_feed(frame: &mut Frame, area: &Rect, tui_bridge: &TuiBridge) {
        let activity_feed = tui_bridge.activity_feed().lock().unwrap();
        let activity_block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::Reset));

        let activity_inner = activity_block.inner(*area);
        frame.render_widget(activity_block, *area);
        activity_feed.render(activity_inner, frame.buffer_mut());
    }

    /// Render status bar
    fn render_status_bar(
        frame: &mut Frame,
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

        frame.render_widget(status, *area);
    }

    /// Render input area with border
    fn render_input_area(
        frame: &mut Frame,
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
        frame.render_widget(input_block, *area);

        let input_line = if input_text.is_empty() {
            Line::from(vec![Span::styled(
                "Type your message here...",
                Style::default().fg(Color::DarkGray),
            )])
        } else {
            let (before_cursor, at_cursor, after_cursor) =
                split_text_at_cursor(input_text, cursor_pos);

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

        frame.render_widget(
            Paragraph::new(input_line).wrap(Wrap { trim: false }),
            input_inner,
        );
    }

    /// Render autocomplete popup
    fn render_autocomplete_popup(
        frame: &mut Frame,
        input_area: &Rect,
        autocomplete: &FileAutocompleteState,
    ) {
        let popup_area = calculate_popup_area(*input_area, autocomplete.matches.len());

        let widget = FileAutocompleteWidget::new(
            &autocomplete.matches,
            autocomplete.selected_index,
            &autocomplete.query,
        );

        frame.render_widget(widget, popup_area);
    }
}

/// Split text at cursor position for rendering (freestanding function for reuse)
#[inline]
fn split_text_at_cursor(text: &str, cursor_pos: usize) -> (&str, &str, &str) {
    if text.is_empty() {
        return ("", " ", "");
    }

    let pos = cursor_pos.min(text.len());
    let before_cursor = &text[..pos];

    // Get char at cursor (or space if at end) - single-pass
    text[pos..]
        .chars()
        .next()
        .map(|c| {
            let end = c.len_utf8();
            (before_cursor, &text[pos..pos + end], &text[pos + end..])
        })
        .unwrap_or((before_cursor, " ", ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_text_at_cursor() {
        let (before, at, after) = split_text_at_cursor("", 0);
        assert_eq!(before, "");
        assert_eq!(at, " ");
        assert_eq!(after, "");

        let (before, at, after) = split_text_at_cursor("hello", 0);
        assert_eq!(before, "");
        assert_eq!(at, "h");
        assert_eq!(after, "ello");

        let (before, at, after) = split_text_at_cursor("hello", 2);
        assert_eq!(before, "he");
        assert_eq!(at, "l");
        assert_eq!(after, "lo");

        let (before, at, after) = split_text_at_cursor("hello", 5);
        assert_eq!(before, "hello");
        assert_eq!(at, " ");
        assert_eq!(after, "");

        let text = "héllo";
        let pos = text.char_indices().nth(2).map(|(i, _)| i).unwrap();
        let (before, at, after) = split_text_at_cursor(text, pos);
        assert_eq!(before, "hé");
        assert_eq!(at, "l");
        assert_eq!(after, "lo");
    }
}
