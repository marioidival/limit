//! Team progress panel widget.
//!
//! Displays workflow phase, task list, and elapsed time during team execution.

use crate::components::team_progress_types::{
    TaskProgressSnapshot, TaskProgressStatus, PHASE_COUNT,
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render the team progress panel. Returns the height used (0 if hidden).
pub fn render_team_progress(frame: &mut Frame, area: Rect, snapshot: &TaskProgressSnapshot) {
    if !snapshot.is_active {
        return;
    }

    let width = area.width.saturating_sub(2) as usize; // inner width

    // Build phase bar
    let phase_bar = build_phase_bar(snapshot, width);
    let mut lines: Vec<Line> = vec![phase_bar];

    // Status text line (dimmed, truncated agent output — char-safe)
    if !snapshot.status_text.is_empty() {
        let mut msg = snapshot.status_text.clone();
        let max_msg_len = width.saturating_sub(2);
        if msg.chars().count() > max_msg_len {
            let mut truncated = String::with_capacity(max_msg_len);
            for (i, ch) in msg.chars().enumerate() {
                if i >= max_msg_len.saturating_sub(1) {
                    break;
                }
                truncated.push(ch);
            }
            truncated.push('…');
            msg = truncated;
        }
        lines.push(Line::from(Span::styled(
            format!(" {}", msg),
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Add task list (max 6 visible tasks)
    let max_task_lines = 6usize;
    let visible_tasks = snapshot.tasks.len().min(max_task_lines);
    if visible_tasks > 0 {
        lines.push(Line::from(Span::styled(
            "Tasks:",
            Style::default().fg(Color::Gray),
        )));
        for task in &snapshot.tasks[..visible_tasks] {
            lines.push(build_task_line(task, width));
        }
        if snapshot.tasks.len() > max_task_lines {
            let remaining = snapshot.tasks.len() - max_task_lines;
            lines.push(Line::from(Span::styled(
                format!("  ... and {} more", remaining),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let total_needed = lines.len() + 2; // +2 for Borders::ALL
    let panel_height = total_needed.min(area.height as usize);
    if panel_height == 0 {
        return;
    }

    let panel_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: panel_height as u16,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Team Progress ")
        .title_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(panel_area);
    frame.render_widget(block, panel_area);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// Calculate the total height the panel needs (0 if inactive).
/// Includes 2-row border overhead from `Borders::ALL`.
pub fn panel_height(snapshot: &TaskProgressSnapshot) -> u16 {
    if !snapshot.is_active {
        return 0;
    }
    let border = 2u16; // Borders::ALL: top + bottom
    let status_line = if snapshot.status_text.is_empty() {
        0
    } else {
        1
    };
    let content = if snapshot.tasks.is_empty() {
        1 + status_line // phase bar only
    } else {
        let task_lines = (snapshot.tasks.len().min(6) as u16) + 1; // +1 for "Tasks:" header
        let more_line = if snapshot.tasks.len() > 6 { 1 } else { 0 };
        1 + status_line + task_lines + more_line // phase bar + status + tasks
    };
    (border + content).clamp(3, 11)
}

fn build_phase_bar(snapshot: &TaskProgressSnapshot, width: usize) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    // Phase name
    let phase_name = snapshot
        .current_phase
        .map(|p| format!("{}", p))
        .unwrap_or_else(|| "Starting".to_string());
    spans.push(Span::styled(
        format!(" {} ", phase_name),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ));

    // Progress bar: [███░░░░░░░░░░] 3/6
    let completed = snapshot.phases_completed;
    let bar_width = 12usize;
    let filled = ((completed * bar_width) / PHASE_COUNT).min(bar_width);
    let empty = bar_width - filled;

    spans.push(Span::raw(" ["));
    if filled > 0 {
        spans.push(Span::styled(
            "█".repeat(filled),
            Style::default().fg(Color::Green),
        ));
    }
    if empty > 0 {
        spans.push(Span::styled(
            "░".repeat(empty),
            Style::default().fg(Color::DarkGray),
        ));
    }
    spans.push(Span::raw("]"));

    spans.push(Span::styled(
        format!(" {}/{}", completed, PHASE_COUNT),
        Style::default().fg(Color::Yellow),
    ));

    // Elapsed timer
    if let Some(started_at) = snapshot.started_at {
        let elapsed = started_at.elapsed().as_secs();
        if elapsed < 60 {
            spans.push(Span::styled(
                format!("  Elapsed: {}s", elapsed),
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            let mins = elapsed / 60;
            let secs = elapsed % 60;
            spans.push(Span::styled(
                format!("  Elapsed: {}m{}s", mins, secs),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    // Pad to width to avoid rendering artifacts
    let current_len: usize = spans.iter().map(|s| s.width()).sum();
    if current_len < width {
        spans.push(Span::raw(" ".repeat(width - current_len)));
    }

    Line::from(spans)
}

fn build_task_line(
    task: &crate::components::team_progress_types::TaskProgressInfo,
    width: usize,
) -> Line<'static> {
    let (icon, color) = match task.status {
        TaskProgressStatus::Pending => ("○", Color::DarkGray),
        TaskProgressStatus::InProgress => ("⏳", Color::Yellow),
        TaskProgressStatus::Completed => ("✅", Color::Green),
        TaskProgressStatus::Failed => ("❌", Color::Red),
    };

    // Truncate description to fit (char-safe for multi-byte UTF-8)
    let max_desc_len = width.saturating_sub(10); // "  icon " + agent label
    let mut desc = task.description.clone();
    let desc_char_len = desc.chars().count();
    if desc_char_len > max_desc_len {
        let mut truncated = String::with_capacity(max_desc_len);
        for (i, ch) in desc.chars().enumerate() {
            if i >= max_desc_len.saturating_sub(1) {
                break;
            }
            truncated.push(ch);
        }
        truncated.push('…');
        desc = truncated;
    }

    let mut spans: Vec<Span<'static>> = vec![
        Span::raw("  "),
        Span::styled(icon.to_string(), Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(desc, Style::default().fg(Color::White)),
    ];

    // Agent label
    if let Some(idx) = task.agent_index {
        spans.push(Span::styled(
            format!(" Jr[{}]", idx),
            Style::default().fg(Color::Cyan),
        ));
    }

    Line::from(spans)
}
