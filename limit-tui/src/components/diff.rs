// Diff View component for displaying unified diffs

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::Widget,
    style::{Color, Style},
    text::{Line, Span},
};

/// Type of diff line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffType {
    /// Added line (+)
    Addition,
    /// Removed line (-)
    Deletion,
    /// Context line (no prefix)
    Context,
    /// Header line (@@ ... @@)
    Header,
}

/// A single line in a diff view
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// Type of diff line
    pub line_type: DiffType,
    /// Content of the line (without prefix)
    pub content: String,
    /// Line number in the old file (if applicable)
    pub old_line: Option<usize>,
    /// Line number in the new file (if applicable)
    pub new_line: Option<usize>,
}

impl DiffLine {
    /// Create a new diff line
    pub fn new(line_type: DiffType, content: String) -> Self {
        Self {
            line_type,
            content,
            old_line: None,
            new_line: None,
        }
    }

    /// Set old line number
    pub fn with_old_line(mut self, line: usize) -> Self {
        self.old_line = Some(line);
        self
    }

    /// Set new line number
    pub fn with_new_line(mut self, line: usize) -> Self {
        self.new_line = Some(line);
        self
    }
}

/// Diff view component for displaying unified diffs
#[derive(Clone)]
pub struct DiffView {
    /// Parsed diff lines
    lines: Vec<DiffLine>,
    /// Current scroll offset (line number)
    scroll_offset: usize,
}

impl DiffView {
    /// Create a new empty diff view
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            scroll_offset: 0,
        }
    }

    /// Create a new diff view from a unified diff string
    pub fn from_diff(diff_text: &str) -> Self {
        Self {
            lines: parse_diff(diff_text),
            scroll_offset: 0,
        }
    }

    /// Set the diff content
    pub fn set_diff(&mut self, diff_text: &str) {
        self.lines = parse_diff(diff_text);
        self.scroll_offset = 0;
    }

    /// Get the number of lines in the diff
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if the diff is empty
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Get current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll down by one line
    pub fn scroll_down(&mut self) {
        if self.scroll_offset < self.lines.len().saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }

    /// Scroll up by one page
    pub fn page_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    /// Scroll down by one page
    pub fn page_down(&mut self, page_size: usize) {
        let max_offset = self.lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + page_size).min(max_offset);
    }

    /// Scroll to the top
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll to the bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.lines.len().saturating_sub(1);
    }
}

impl Default for DiffView {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for DiffView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Calculate line numbers column width
        let max_line = self
            .lines
            .iter()
            .filter_map(|l| l.old_line.or(l.new_line))
            .max();
        let line_num_width = max_line.map_or(0, |n| n.to_string().len()) + 1;

        // Only render visible lines (virtualization)
        let visible_count = area.height as usize;
        let start_idx = self.scroll_offset;
        let end_idx = (start_idx + visible_count).min(self.lines.len());

        for (i, line) in self.lines[start_idx..end_idx].iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.bottom() {
                break;
            }

            // Render line numbers
            let old_line_str = line.old_line.map(|n| n.to_string()).unwrap_or_default();
            let new_line_str = line.new_line.map(|n| n.to_string()).unwrap_or_default();

            // Style based on diff type
            let style = match line.line_type {
                DiffType::Addition => Style::default().fg(Color::Green),
                DiffType::Deletion => Style::default().fg(Color::Red),
                DiffType::Context => Style::default().fg(Color::White),
                DiffType::Header => Style::default().fg(Color::Yellow),
            };

            // Build line numbers display
            let line_num_text = if old_line_str.is_empty() && new_line_str.is_empty() {
                format!("{:>line_num_width$} ", "")
            } else if old_line_str.is_empty() {
                format!("{:>line_num_width$} ", new_line_str)
            } else {
                format!("{:>line_num_width$} ", old_line_str)
            };

            // Truncate line content if too long
            let content_max_width = (area.width as usize).saturating_sub(line_num_width + 2);
            let content = if line.content.len() > content_max_width {
                format!(
                    "{}...",
                    &line.content[..content_max_width.saturating_sub(3)]
                )
            } else {
                line.content.clone()
            };

            // Render line numbers with dim style
            let line_num_spans = vec![Span::styled(
                line_num_text,
                Style::default().fg(Color::DarkGray),
            )];
            let line_num_line = Line::from(line_num_spans);
            buf.set_line(area.x, y, &line_num_line, line_num_width as u16);

            // Render diff line
            let content_start_x = area.x + line_num_width as u16;
            let text_spans = vec![Span::styled(content, style)];
            let text_line = Line::from(text_spans);
            buf.set_line(
                content_start_x,
                y,
                &text_line,
                area.width - line_num_width as u16,
            );
        }
    }
}

impl Widget for &DiffView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Delegate to the owned implementation by cloning
        // This allows rendering by reference
        self.clone().render(area, buf);
    }
}

/// Parse unified diff format into DiffLine structs
pub fn parse_diff(diff_text: &str) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    let mut old_line: Option<usize> = None;
    let mut new_line: Option<usize> = None;
    let mut in_hunk = false;

    for line in diff_text.lines() {
        if line.starts_with("---") {
            // Old file header
            lines.push(DiffLine::new(DiffType::Header, line.to_string()));
            in_hunk = false;
        } else if line.starts_with("+++") {
            // New file header
            lines.push(DiffLine::new(DiffType::Header, line.to_string()));
            in_hunk = false;
        } else if line.starts_with("@@") {
            // Hunk header - parse line numbers
            lines.push(DiffLine::new(DiffType::Header, line.to_string()));
            in_hunk = true;

            // Parse hunk header to get starting line numbers
            // Format: @@ -old_start,old_count +new_start,new_count @@
            if let Some(hunk_part) = line.split("@@").nth(1) {
                let parts: Vec<&str> = hunk_part.split_whitespace().collect();
                for part in parts {
                    if part.starts_with('-') {
                        // Old file start line
                        if let Some(line_num) = part.trim_start_matches('-').split(',').next() {
                            old_line = line_num.parse().ok();
                        }
                    } else if part.starts_with('+') {
                        // New file start line
                        if let Some(line_num) = part.trim_start_matches('+').split(',').next() {
                            new_line = line_num.parse().ok();
                        }
                    }
                }
            }
        } else if in_hunk {
            // Hunk content
            if let Some(stripped) = line.strip_prefix('+') {
                // Added line
                let content = stripped.to_string();
                let _line_num = new_line.map(|n| {
                    n + lines
                        .iter()
                        .filter(|l| l.line_type == DiffType::Addition)
                        .count()
                });
                lines.push(
                    DiffLine::new(DiffType::Addition, content).with_new_line(new_line.unwrap_or(0)),
                );
                new_line = new_line.map(|n| n + 1);
            } else if let Some(stripped) = line.strip_prefix('-') {
                // Removed line
                let content = stripped.to_string();
                lines.push(
                    DiffLine::new(DiffType::Deletion, content).with_old_line(old_line.unwrap_or(0)),
                );
                old_line = old_line.map(|n| n + 1);
            } else if let Some(stripped) = line.strip_prefix(' ') {
                // Context line
                let content = stripped.to_string();
                lines.push(
                    DiffLine::new(DiffType::Context, content)
                        .with_old_line(old_line.unwrap_or(0))
                        .with_new_line(new_line.unwrap_or(0)),
                );
                old_line = old_line.map(|n| n + 1);
                new_line = new_line.map(|n| n + 1);
            } else {
                // Other line (treat as context)
                lines.push(DiffLine::new(DiffType::Context, line.to_string()));
            }
        } else {
            // Outside hunk (header lines)
            lines.push(DiffLine::new(DiffType::Header, line.to_string()));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_view_new() {
        let view = DiffView::new();
        assert_eq!(view.len(), 0);
        assert!(view.is_empty());
        assert_eq!(view.scroll_offset(), 0);
    }

    #[test]
    fn test_diff_view_default() {
        let view = DiffView::default();
        assert_eq!(view.len(), 0);
        assert!(view.is_empty());
    }

    #[test]
    fn test_parse_diff() {
        let diff_text = r#"--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,4 @@
 line 1
-deleted line
+added line
 line 2
+new line"#;

        let lines = parse_diff(diff_text);

        assert_eq!(lines.len(), 8);

        // Check first header
        assert_eq!(lines[0].line_type, DiffType::Header);
        assert!(lines[0].content.contains("---"));

        // Check hunk header
        let hunk_idx = lines
            .iter()
            .position(|l| l.line_type == DiffType::Header && l.content.starts_with("@@"));
        assert!(hunk_idx.is_some());

        // Check context line
        let context_idx = lines
            .iter()
            .position(|l| l.line_type == DiffType::Context && l.content == "line 1");
        assert!(context_idx.is_some());

        // Check deletion
        let deletion_idx = lines
            .iter()
            .position(|l| l.line_type == DiffType::Deletion && l.content == "deleted line");
        assert!(deletion_idx.is_some());

        // Check addition
        let addition_idx = lines
            .iter()
            .position(|l| l.line_type == DiffType::Addition && l.content == "added line");
        assert!(addition_idx.is_some());

        // Check new line addition
        let new_addition_idx = lines
            .iter()
            .position(|l| l.line_type == DiffType::Addition && l.content == "new line");
        assert!(new_addition_idx.is_some());
    }

    #[test]
    fn test_diff_view_from_diff() {
        let diff_text = "--- a/file.txt\n+++ b/file.txt\n@@ -1,1 +1,1 @@\n-context\n+new";
        let view = DiffView::from_diff(diff_text);

        assert_eq!(view.len(), 5);
        assert!(!view.is_empty());
    }

    #[test]
    fn test_diff_view_set_diff() {
        let mut view = DiffView::new();
        assert!(view.is_empty());

        let diff_text = "--- a/file.txt\n+++ b/file.txt\n@@ -1,1 +1,1 @@\n-old\n+new";
        view.set_diff(diff_text);

        assert_eq!(view.len(), 5);
        assert!(!view.is_empty());
        assert_eq!(view.scroll_offset(), 0);
    }

    #[test]
    fn test_diff_view_scroll() {
        let mut view = DiffView::from_diff(
            "--- a/file.txt\n+++ b/file.txt\n@@ -1,5 +1,5 @@\n line 1\n line 2\n line 3\n line 4\n line 5",
        );

        assert_eq!(view.scroll_offset(), 0);

        // Scroll down
        view.scroll_down();
        assert_eq!(view.scroll_offset(), 1);

        view.scroll_down();
        assert_eq!(view.scroll_offset(), 2);

        // Scroll up
        view.scroll_up();
        assert_eq!(view.scroll_offset(), 1);

        view.scroll_up();
        assert_eq!(view.scroll_offset(), 0);

        // Can't scroll past top
        view.scroll_up();
        assert_eq!(view.scroll_offset(), 0);
    }

    #[test]
    fn test_diff_view_page_scroll() {
        let mut view = DiffView::from_diff(
            "--- a/file.txt\n+++ b/file.txt\n@@ -1,20 +1,20 @@\n line 1\n line 2\n line 3\n line 4\n line 5\n line 6\n line 7\n line 8\n line 9\n line 10\n line 11\n line 12\n line 13\n line 14\n line 15\n line 16\n line 17\n line 18\n line 19\n line 20",
        );

        assert_eq!(view.scroll_offset(), 0);

        // Page down
        view.page_down(10);
        assert_eq!(view.scroll_offset(), 10);

        view.page_down(10);
        assert_eq!(view.scroll_offset(), 20); // Can.t go past last line

        // Page up
        view.page_up(10);
        assert_eq!(view.scroll_offset(), 10);

        view.page_up(10);
        assert_eq!(view.scroll_offset(), 0);

        // Can't go past top
        view.page_up(10);
        assert_eq!(view.scroll_offset(), 0);
    }

    #[test]
    fn test_diff_view_scroll_to_top_bottom() {
        let mut view = DiffView::from_diff(
            "--- a/file.txt\n+++ b/file.txt\n@@ -1,10 +1,10 @@\n line 1\n line 2\n line 3\n line 4\n line 5\n line 6\n line 7\n line 8\n line 9\n line 10",
        );

        view.scroll_down();
        view.scroll_down();

        assert_eq!(view.scroll_offset(), 2);

        view.scroll_to_top();
        assert_eq!(view.scroll_offset(), 0);

        view.scroll_to_bottom();
        assert_eq!(view.scroll_offset(), 12); // Last line index
    }

    #[test]
    fn test_diff_view_large_file() {
        // Create a large diff with 10K+ lines
        let mut diff_text =
            String::from("--- a/large.txt\n+++ b/large.txt\n@@ -1,10000 +1,10000 @@\n");

        for i in 1..=10000 {
            diff_text.push_str(&format!(" line {}\n", i));
        }

        let view = DiffView::from_diff(&diff_text);

        assert_eq!(view.len(), 10003); // Headers + 10000 context lines
        assert!(!view.is_empty());

        // Test scrolling works with large file
        let mut view = DiffView::from_diff(&diff_text);
        view.page_down(100);
        assert_eq!(view.scroll_offset(), 100);

        view.page_up(50);
        assert_eq!(view.scroll_offset(), 50);

        view.scroll_to_bottom();
        assert_eq!(view.scroll_offset(), 10002);
    }

    #[test]
    fn test_diff_line_new() {
        let line = DiffLine::new(DiffType::Addition, "test content".to_string());
        assert_eq!(line.line_type, DiffType::Addition);
        assert_eq!(line.content, "test content");
        assert_eq!(line.old_line, None);
        assert_eq!(line.new_line, None);
    }

    #[test]
    fn test_diff_line_with_line_numbers() {
        let line = DiffLine::new(DiffType::Addition, "test".to_string())
            .with_old_line(10)
            .with_new_line(15);

        assert_eq!(line.old_line, Some(10));
        assert_eq!(line.new_line, Some(15));
    }

    #[test]
    fn test_parse_empty_diff() {
        let lines = parse_diff("");
        assert!(lines.is_empty());
    }

    #[test]
    fn test_parse_diff_only_headers() {
        let diff_text = "--- a/file.txt\n+++ b/file.txt";
        let lines = parse_diff(diff_text);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].line_type, DiffType::Header);
        assert_eq!(lines[1].line_type, DiffType::Header);
    }

    #[test]
    fn test_diff_view_render() {
        let view = DiffView::from_diff(
            "--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,3 @@\n line 1\n-old\n+new\n line 3",
        );

        // This test just verifies that the render method compiles
        // Actual rendering is tested visually via the example
        assert_eq!(view.len(), 7);
    }
}
