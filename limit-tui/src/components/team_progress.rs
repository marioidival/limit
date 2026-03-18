//! Team progress panel widget.
//!
//! Displays workflow phase, task list, and elapsed time during team execution.

use crate::components::progress::SPINNER_FRAMES;
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

    // Finish summary line (when workflow is finished)
    if snapshot.finished {
        let color = if snapshot.success {
            Color::Green
        } else {
            Color::Yellow
        };
        lines.push(Line::from(Span::styled(
            format!(" {}", snapshot.finish_summary),
            Style::default().fg(color),
        )));
    }

    // Status text lines (dimmed, truncated agent output — char-safe, multi-line support)
    if !snapshot.status_text.is_empty() {
        let lines_to_render: Vec<&str> = snapshot.status_text.lines().take(2).collect();
        for msg in lines_to_render {
            let mut msg_str = msg.to_string();
            let max_msg_len = width.saturating_sub(2);
            if msg_str.chars().count() > max_msg_len {
                let mut truncated = String::with_capacity(max_msg_len);
                for (i, ch) in msg_str.chars().enumerate() {
                    if i >= max_msg_len.saturating_sub(1) {
                        break;
                    }
                    truncated.push(ch);
                }
                truncated.push('…');
                msg_str = truncated;
            }
            lines.push(Line::from(Span::styled(
                format!(" {}", msg_str),
                Style::default().fg(Color::DarkGray),
            )));
        }
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
    let finish_summary_line = if snapshot.finished { 1 } else { 0 };
    // Multi-line status support: render up to 2 lines
    let status_lines = if snapshot.status_text.is_empty() {
        0
    } else {
        snapshot.status_text.lines().take(2).count() as u16
    };
    let content = if snapshot.tasks.is_empty() {
        1 + finish_summary_line + status_lines // phase bar only
    } else {
        let task_lines = (snapshot.tasks.len().min(6) as u16) + 1; // +1 for "Tasks:" header
        let more_line = if snapshot.tasks.len() > 6 { 1 } else { 0 };
        1 + finish_summary_line + status_lines + task_lines + more_line // phase bar + status + tasks
    };
    (border + content).clamp(3, 11)
}

fn build_phase_bar(snapshot: &TaskProgressSnapshot, width: usize) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    // Phase name with spinner
    let phase_name = snapshot
        .current_phase
        .map(|p| format!("{}", p))
        .unwrap_or_else(|| "Starting".to_string());
    let spinner = SPINNER_FRAMES[snapshot.spinner_frame % SPINNER_FRAMES.len()];
    spans.push(Span::styled(
        format!(" {} ", format!("{} {}", spinner, phase_name)),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ));

    // Progress bar with color-coded phases: [███░░░░░░░░░░] 3/6
    // Phase colors: PM=Magenta, TL=Blue, Jr=Yellow, Validation/Delivery=Green
    let completed = snapshot.phases_completed;
    let bar_width = 12usize;
    let filled = ((completed * bar_width) / PHASE_COUNT).min(bar_width);
    let empty = bar_width - filled;

    spans.push(Span::raw("["));
    for i in 0..filled {
        // Map bar position to phase index (0-5) and assign color
        let phase_index = (i * PHASE_COUNT as usize) / bar_width;
        let color = match phase_index {
            0 => Color::Magenta, // PM Analysis
            1 => Color::Blue,    // TL Plan
            2 => Color::Blue,    // TL Breakdown
            3 => Color::Yellow,  // Jr Execution
            4 => Color::Green,   // TL Validation
            5 => Color::Green,   // PM Delivery
            _ => Color::Green,
        };
        spans.push(Span::styled("█", Style::default().fg(color)));
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
