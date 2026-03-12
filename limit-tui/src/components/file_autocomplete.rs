use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

/// File match result from FileFinder
#[derive(Debug, Clone)]
pub struct FileMatchData {
    /// Relative path from working directory
    pub path: String,
    /// Whether it's a directory
    pub is_dir: bool,
}

/// Widget for displaying file autocomplete suggestions
pub struct FileAutocompleteWidget<'a> {
    /// List of matching files
    matches: &'a [FileMatchData],
    /// Currently selected index
    selected_index: usize,
    /// Query string for highlighting
    query: &'a str,
}

impl<'a> FileAutocompleteWidget<'a> {
    /// Create a new file autocomplete widget
    pub fn new(matches: &'a [FileMatchData], selected_index: usize, query: &'a str) -> Self {
        Self {
            matches,
            selected_index,
            query,
        }
    }

    /// Highlight matching characters in the path
    fn highlight_match(&self, path: &str) -> Vec<Span<'static>> {
        let query_lower = self.query.to_lowercase();
        let path_lower = path.to_lowercase();

        if self.query.is_empty() {
            return vec![Span::raw(path.to_string())];
        }

        let mut spans = Vec::new();
        let mut last_end = 0;

        // Find all matches
        let mut pos = 0;
        while pos < path.len() {
            if let Some(start) = path_lower[pos..].find(&query_lower) {
                let abs_start = pos + start;
                let abs_end = abs_start + self.query.len();

                // Add text before match
                if abs_start > last_end {
                    spans.push(Span::raw(path[last_end..abs_start].to_string()));
                }

                // Add matched text with highlight
                spans.push(Span::styled(
                    path[abs_start..abs_end.min(path.len())].to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));

                last_end = abs_end.min(path.len());
                pos = abs_end;
            } else {
                break;
            }
        }

        // Add remaining text
        if last_end < path.len() {
            spans.push(Span::raw(path[last_end..].to_string()));
        }

        spans
    }
}

impl<'a> Widget for FileAutocompleteWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.matches.is_empty() || area.height == 0 {
            return;
        }

        // Draw border
        let border_style = Style::default().fg(Color::Cyan);

        // Top border
        if area.height > 0 {
            let top_line = Line::default().spans(vec![
                Span::styled("┌", border_style),
                Span::styled(
                    "─".repeat(area.width.saturating_sub(2) as usize),
                    border_style,
                ),
                Span::styled("┐", border_style),
            ]);
            top_line.render(area, buf);
        }

        // Draw each match
        let max_items = (area.height.saturating_sub(2)) as usize; // -2 for borders
        let items_to_show = self.matches.len().min(max_items);

        for (i, file_match) in self.matches.iter().take(items_to_show).enumerate() {
            let y = area.y + 1 + i as u16;
            if y >= area.y + area.height - 1 {
                break;
            }

            let is_selected = i == self.selected_index;
            let bg_color = if is_selected {
                Color::DarkGray
            } else {
                Color::Reset
            };

            // Build the line with selection indicator and path
            let mut spans = vec![Span::styled("│", border_style)];

            if is_selected {
                spans.push(Span::styled(
                    "► ",
                    Style::default().fg(Color::Yellow).bg(bg_color),
                ));
            } else {
                spans.push(Span::styled("  ", Style::default().bg(bg_color)));
            }

            // Add highlighted path
            let mut path_spans = self.highlight_match(&file_match.path);
            spans.append(&mut path_spans);

            // Add directory indicator
            if file_match.is_dir {
                spans.push(Span::styled(
                    "/",
                    Style::default().fg(Color::Blue).bg(bg_color),
                ));
            }

            // Calculate remaining space for padding
            let used_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
            let padding = area
                .width
                .saturating_sub(used_width as u16)
                .saturating_sub(1);

            spans.push(Span::styled(
                " ".repeat(padding as usize),
                Style::default().bg(bg_color),
            ));

            spans.push(Span::styled("│", border_style));

            let line = Line::default().spans(spans);
            line.render(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
        }

        // Bottom border
        if area.height > 1 {
            let bottom_y = area.y + area.height - 1;
            let bottom_line = Line::default().spans(vec![
                Span::styled("└", border_style),
                Span::styled(
                    "─".repeat(area.width.saturating_sub(2) as usize),
                    border_style,
                ),
                Span::styled("┘", border_style),
            ]);
            bottom_line.render(
                Rect {
                    x: area.x,
                    y: bottom_y,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
        }
    }
}

/// Calculate popup area for autocomplete widget
pub fn calculate_popup_area(input_area: Rect, match_count: usize) -> Rect {
    let max_height = 10;
    let height = (match_count + 2).min(max_height as usize) as u16; // +2 for borders

    Rect {
        x: input_area.x,
        y: input_area.y.saturating_sub(height),
        width: input_area.width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_match_basic() {
        let matches = vec![FileMatchData {
            path: "Cargo.toml".to_string(),
            is_dir: false,
        }];
        let widget = FileAutocompleteWidget::new(&matches, 0, "Cargo");
        let spans = widget.highlight_match("Cargo.toml");

        assert!(!spans.is_empty());
    }

    #[test]
    fn test_highlight_match_empty_query() {
        let matches = vec![FileMatchData {
            path: "Cargo.toml".to_string(),
            is_dir: false,
        }];
        let widget = FileAutocompleteWidget::new(&matches, 0, "");
        let spans = widget.highlight_match("Cargo.toml");

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "Cargo.toml");
    }

    #[test]
    fn test_calculate_popup_area() {
        let input_area = Rect::new(0, 20, 80, 3);
        let popup = calculate_popup_area(input_area, 5);

        assert_eq!(popup.height, 7); // 5 + 2 borders
        assert_eq!(popup.y, 13); // 20 - 7
    }
}
