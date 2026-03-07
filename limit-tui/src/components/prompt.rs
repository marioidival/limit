// Interactive prompt components for terminal UI

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

/// Result from InputPrompt interaction
#[derive(Debug, Clone, PartialEq)]
pub enum InputResult {
    /// User submitted the input
    Submitted(String),
    /// User cancelled the input
    Cancelled,
    /// No action taken
    None,
}

/// Result from SelectPrompt interaction
#[derive(Debug, Clone, PartialEq)]
pub enum SelectResult {
    /// User selected an option (index)
    Selected(usize),
    /// User cancelled the selection
    Cancelled,
    /// No action taken
    None,
}

/// Text input prompt with cursor, placeholder, and validation
#[derive(Debug, Clone)]
pub struct InputPrompt {
    text: String,
    cursor_pos: usize,
    placeholder: String,
    error: Option<String>,
}

impl InputPrompt {
    /// Create a new input prompt with placeholder text
    pub fn new(placeholder: &str) -> Self {
        Self {
            text: String::new(),
            cursor_pos: 0,
            placeholder: placeholder.to_string(),
            error: None,
        }
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: KeyEvent) -> InputResult {
        match key.code {
            // Character input
            KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE => {
                self.insert_char(c);
                InputResult::None
            }

            // Backspace - delete character before cursor
            KeyCode::Backspace => {
                self.delete_char_before_cursor();
                InputResult::None
            }

            // Delete - delete character at cursor
            KeyCode::Delete => {
                self.delete_char_at_cursor();
                InputResult::None
            }

            // Left arrow - move cursor left
            KeyCode::Left => {
                self.move_cursor_left();
                InputResult::None
            }

            // Right arrow - move cursor right
            KeyCode::Right => {
                self.move_cursor_right();
                InputResult::None
            }

            // Home - move cursor to start
            KeyCode::Home => {
                self.cursor_pos = 0;
                InputResult::None
            }

            // End - move cursor to end
            KeyCode::End => {
                self.cursor_pos = self.text.len();
                InputResult::None
            }

            // Enter - submit input
            KeyCode::Enter => {
                if self.validate() {
                    InputResult::Submitted(self.text.clone())
                } else {
                    InputResult::None
                }
            }

            // Escape - cancel
            KeyCode::Esc => InputResult::Cancelled,

            _ => InputResult::None,
        }
    }

    /// Insert a character at cursor position
    fn insert_char(&mut self, c: char) {
        if self.cursor_pos <= self.text.len() {
            self.text.insert(self.cursor_pos, c);
            self.cursor_pos += 1;
            self.clear_error();
        }
    }

    /// Delete character before cursor
    fn delete_char_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.text.remove(self.cursor_pos);
            self.clear_error();
        }
    }

    /// Delete character at cursor position
    fn delete_char_at_cursor(&mut self) {
        if self.cursor_pos < self.text.len() {
            self.text.remove(self.cursor_pos);
            self.clear_error();
        }
    }

    /// Move cursor left
    fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor right
    fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.text.len() {
            self.cursor_pos += 1;
        }
    }

    /// Validate input - override for custom validation
    fn validate(&self) -> bool {
        self.error.is_none()
    }

    /// Set validation error message
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    /// Clear error
    fn clear_error(&mut self) {
        self.error = None;
    }

    /// Get current text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get current cursor position
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Render the input prompt
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Determine display text (placeholder or actual text)
        let display_text = if self.text.is_empty() {
            Text::from(vec![Line::from(vec![Span::styled(
                &self.placeholder,
                Style::default().fg(Color::DarkGray),
            )])])
        } else {
            // Split text into before cursor, at cursor, after cursor
            let before_cursor = &self.text[..self.cursor_pos];
            let after_cursor = &self.text[self.cursor_pos..];

            Text::from(vec![Line::from(vec![
                Span::raw(before_cursor),
                Span::styled(
                    if after_cursor.chars().next().is_some() {
                        after_cursor.chars().next().unwrap().to_string()
                    } else {
                        " ".to_string()
                    },
                    Style::default().add_modifier(Modifier::REVERSED),
                ),
                Span::raw(&after_cursor[after_cursor.chars().next().map_or(0, |c| c.len_utf8())..]),
            ])])
        };

        // Create paragraph with text
        let paragraph = Paragraph::new(display_text);

        // Render the paragraph
        paragraph.render(area, buf);

        // Render error message below input if present
        if let Some(ref error_msg) = self.error {
            let error_area = Rect {
                x: area.x,
                y: area.y.saturating_add(1),
                width: area.width,
                height: 1,
            };
            let error_text = Paragraph::new(Text::from(vec![Line::from(vec![Span::styled(
                error_msg,
                Style::default().fg(Color::Red),
            )])]));
            error_text.render(error_area, buf);
        }
    }
}

impl Default for InputPrompt {
    fn default() -> Self {
        Self::new("")
    }
}

/// Selection prompt for choosing from a list of options
/// Selection prompt for choosing from a list of options
#[derive(Debug, Clone)]
pub struct SelectPrompt {
    options: Vec<String>,
    selected: usize,
    title: String,
}

impl SelectPrompt {
    /// Create a new select prompt with title and options
    pub fn new(title: &str, options: Vec<String>) -> Self {
        Self {
            options,
            selected: 0,
            title: title.to_string(),
        }
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: KeyEvent) -> SelectResult {
        match key.code {
            // Up arrow - move selection up
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                SelectResult::None
            }

            // Down arrow - move selection down
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.options.len() {
                    self.selected += 1;
                }
                SelectResult::None
            }

            // Page Up - move up 5 items
            KeyCode::PageUp => {
                self.selected = self.selected.saturating_sub(5);
                SelectResult::None
            }

            // Page Down - move down 5 items
            KeyCode::PageDown => {
                self.selected = (self.selected + 5).min(self.options.len() - 1);
                SelectResult::None
            }

            // Home - select first item
            KeyCode::Home => {
                self.selected = 0;
                SelectResult::None
            }

            // End - select last item
            KeyCode::End => {
                self.selected = self.options.len().saturating_sub(1);
                SelectResult::None
            }

            // Enter - confirm selection
            KeyCode::Enter => SelectResult::Selected(self.selected),

            // Escape - cancel
            KeyCode::Esc => SelectResult::Cancelled,

            _ => SelectResult::None,
        }
    }

    /// Get currently selected option
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Get selected option text
    pub fn selected_text(&self) -> Option<&str> {
        self.options.get(self.selected).map(|s| s.as_str())
    }

    /// Get all options
    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// Get title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Render the select prompt
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Create block with title
        let block = Block::bordered().title(self.title.as_str());

        // Calculate content area (inside borders)
        let content_area = block.inner(area);

        // Render the block border
        block.render(area, buf);

        // Render each option
        for (i, option) in self.options.iter().enumerate() {
            if i >= content_area.height as usize {
                break;
            }

            // Determine style based on selection
            let is_selected = i == self.selected;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Build line with arrow indicator for selected item
            let line = Line::from(vec![
                Span::styled(
                    if is_selected { ">" } else { " " },
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" "),
                Span::styled(option, style),
            ]);

            // Render line
            Paragraph::new(Text::from(line)).style(style).render(
                Rect {
                    x: content_area.x,
                    y: content_area.y.saturating_add(i as u16),
                    width: content_area.width,
                    height: 1,
                },
                buf,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

    fn create_key_event(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    // InputPrompt tests

    #[test]
    fn test_input_prompt_new() {
        let prompt = InputPrompt::new("Enter text:");
        assert_eq!(prompt.text(), "");
        assert_eq!(prompt.cursor_pos(), 0);
        assert_eq!(prompt.placeholder, "Enter text:");
        assert!(prompt.error.is_none());
    }

    #[test]
    fn test_input_prompt_default() {
        let prompt = InputPrompt::default();
        assert_eq!(prompt.text(), "");
        assert_eq!(prompt.cursor_pos(), 0);
        assert_eq!(prompt.placeholder, "");
    }

    #[test]
    fn test_input_prompt_type() {
        let mut prompt = InputPrompt::new("Enter text:");

        // Type characters
        prompt.handle_key(create_key_event(KeyCode::Char('H')));
        assert_eq!(prompt.text(), "H");
        assert_eq!(prompt.cursor_pos(), 1);

        prompt.handle_key(create_key_event(KeyCode::Char('i')));
        assert_eq!(prompt.text(), "Hi");
        assert_eq!(prompt.cursor_pos(), 2);

        prompt.handle_key(create_key_event(KeyCode::Char('!')));
        assert_eq!(prompt.text(), "Hi!");
        assert_eq!(prompt.cursor_pos(), 3);
    }

    #[test]
    fn test_input_prompt_backspace() {
        let mut prompt = InputPrompt::new("Enter text:");

        // Type some text
        prompt.handle_key(create_key_event(KeyCode::Char('H')));
        prompt.handle_key(create_key_event(KeyCode::Char('i')));

        assert_eq!(prompt.text(), "Hi");
        assert_eq!(prompt.cursor_pos(), 2);

        // Backspace
        prompt.handle_key(create_key_event(KeyCode::Backspace));
        assert_eq!(prompt.text(), "H");
        assert_eq!(prompt.cursor_pos(), 1);

        // Another backspace
        prompt.handle_key(create_key_event(KeyCode::Backspace));
        assert_eq!(prompt.text(), "");
        assert_eq!(prompt.cursor_pos(), 0);
    }

    #[test]
    fn test_input_prompt_backspace_in_middle() {
        let mut prompt = InputPrompt::new("Enter text:");

        prompt.text = String::from("Hello");
        prompt.cursor_pos = 3;

        prompt.handle_key(create_key_event(KeyCode::Backspace));
        assert_eq!(prompt.text(), "Helo");
        assert_eq!(prompt.cursor_pos(), 2);
    }

    #[test]
    fn test_input_prompt_delete() {
        let mut prompt = InputPrompt::new("Enter text:");

        prompt.text = String::from("Hello");
        prompt.cursor_pos = 2;

        // Delete character at cursor
        prompt.handle_key(create_key_event(KeyCode::Delete));
        assert_eq!(prompt.text(), "Helo");
        assert_eq!(prompt.cursor_pos(), 2);
    }

    #[test]
    fn test_input_prompt_cursor_move() {
        let mut prompt = InputPrompt::new("Enter text:");

        // Type text
        prompt.text = String::from("Hello");
        prompt.cursor_pos = 5;

        // Move left
        prompt.handle_key(create_key_event(KeyCode::Left));
        assert_eq!(prompt.cursor_pos(), 4);

        prompt.handle_key(create_key_event(KeyCode::Left));
        assert_eq!(prompt.cursor_pos(), 3);

        // Move right
        prompt.handle_key(create_key_event(KeyCode::Right));
        assert_eq!(prompt.cursor_pos(), 4);

        // Can't move past end
        prompt.handle_key(create_key_event(KeyCode::Right));
        prompt.handle_key(create_key_event(KeyCode::Right));
        assert_eq!(prompt.cursor_pos(), 5);
    }

    #[test]
    fn test_input_prompt_home_end() {
        let mut prompt = InputPrompt::new("Enter text:");

        prompt.text = String::from("Hello");
        prompt.cursor_pos = 2;

        // Home
        prompt.handle_key(create_key_event(KeyCode::Home));
        assert_eq!(prompt.cursor_pos(), 0);

        // End
        prompt.handle_key(create_key_event(KeyCode::End));
        assert_eq!(prompt.cursor_pos(), 5);
    }

    #[test]
    fn test_input_prompt_submit() {
        let mut prompt = InputPrompt::new("Enter text:");

        prompt.text = String::from("Test");

        // Submit with Enter
        let result = prompt.handle_key(create_key_event(KeyCode::Enter));
        assert_eq!(result, InputResult::Submitted("Test".to_string()));
    }

    #[test]
    fn test_input_prompt_cancel() {
        let mut prompt = InputPrompt::new("Enter text:");

        prompt.text = String::from("Test");

        // Cancel with Escape
        let result = prompt.handle_key(create_key_event(KeyCode::Esc));
        assert_eq!(result, InputResult::Cancelled);
        assert_eq!(prompt.text(), "Test"); // Text should remain
    }

    #[test]
    fn test_input_prompt_set_error() {
        let mut prompt = InputPrompt::new("Enter text:");

        prompt.set_error("Invalid input".to_string());
        assert_eq!(prompt.error, Some("Invalid input".to_string()));

        // Validation should fail with error
        assert!(!prompt.validate());
    }

    #[test]
    fn test_input_prompt_clear_error() {
        let mut prompt = InputPrompt::new("Enter text:");

        prompt.set_error("Invalid input".to_string());

        // Typing should clear error
        prompt.handle_key(create_key_event(KeyCode::Char('H')));
        assert!(prompt.error.is_none());
    }

    // SelectPrompt tests

    #[test]
    fn test_select_prompt_new() {
        let options = vec![
            "Option 1".to_string(),
            "Option 2".to_string(),
            "Option 3".to_string(),
        ];

        let prompt = SelectPrompt::new("Choose:", options);
        assert_eq!(prompt.title(), "Choose:");
        assert_eq!(prompt.selected(), 0);
        assert_eq!(prompt.options().len(), 3);
        assert_eq!(prompt.selected_text(), Some("Option 1"));
    }

    #[test]
    fn test_select_prompt_navigate_down() {
        let options = vec![
            "Option 1".to_string(),
            "Option 2".to_string(),
            "Option 3".to_string(),
        ];

        let mut prompt = SelectPrompt::new("Choose:", options);

        // Navigate down
        prompt.handle_key(create_key_event(KeyCode::Down));
        assert_eq!(prompt.selected(), 1);
        assert_eq!(prompt.selected_text(), Some("Option 2"));

        prompt.handle_key(create_key_event(KeyCode::Down));
        assert_eq!(prompt.selected(), 2);
        assert_eq!(prompt.selected_text(), Some("Option 3"));

        // Can't go past last option
        prompt.handle_key(create_key_event(KeyCode::Down));
        assert_eq!(prompt.selected(), 2);
    }

    #[test]
    fn test_select_prompt_navigate_up() {
        let options = vec![
            "Option 1".to_string(),
            "Option 2".to_string(),
            "Option 3".to_string(),
        ];

        let mut prompt = SelectPrompt::new("Choose:", options);
        prompt.selected = 2;

        // Navigate up
        prompt.handle_key(create_key_event(KeyCode::Up));
        assert_eq!(prompt.selected(), 1);

        prompt.handle_key(create_key_event(KeyCode::Up));
        assert_eq!(prompt.selected(), 0);

        // Can't go before first option
        prompt.handle_key(create_key_event(KeyCode::Up));
        assert_eq!(prompt.selected(), 0);
    }

    #[test]
    fn test_select_prompt_vim_keys() {
        let options = vec![
            "Option 1".to_string(),
            "Option 2".to_string(),
            "Option 3".to_string(),
        ];

        let mut prompt = SelectPrompt::new("Choose:", options);

        // 'j' for down
        prompt.handle_key(create_key_event(KeyCode::Char('j')));
        assert_eq!(prompt.selected(), 1);

        // 'k' for up
        prompt.handle_key(create_key_event(KeyCode::Char('k')));
        assert_eq!(prompt.selected(), 0);
    }

    #[test]
    fn test_select_prompt_select() {
        let options = vec![
            "Option 1".to_string(),
            "Option 2".to_string(),
            "Option 3".to_string(),
        ];

        let mut prompt = SelectPrompt::new("Choose:", options);
        prompt.selected = 1;

        // Select with Enter
        let result = prompt.handle_key(create_key_event(KeyCode::Enter));
        assert_eq!(result, SelectResult::Selected(1));
    }

    #[test]
    fn test_select_prompt_cancel() {
        let options = vec![
            "Option 1".to_string(),
            "Option 2".to_string(),
            "Option 3".to_string(),
        ];

        let mut prompt = SelectPrompt::new("Choose:", options);

        // Cancel with Escape
        let result = prompt.handle_key(create_key_event(KeyCode::Esc));
        assert_eq!(result, SelectResult::Cancelled);
    }

    #[test]
    fn test_select_prompt_page_navigation() {
        let options = (0..20).map(|i| format!("Option {}", i)).collect();

        let mut prompt = SelectPrompt::new("Choose:", options);
        assert_eq!(prompt.selected(), 0);

        // Page Down
        prompt.handle_key(create_key_event(KeyCode::PageDown));
        assert_eq!(prompt.selected(), 5);

        prompt.handle_key(create_key_event(KeyCode::PageDown));
        assert_eq!(prompt.selected(), 10);

        // Page Up
        prompt.handle_key(create_key_event(KeyCode::PageUp));
        assert_eq!(prompt.selected(), 5);

        // Page Up at start
        prompt.selected = 2;
        prompt.handle_key(create_key_event(KeyCode::PageUp));
        assert_eq!(prompt.selected(), 0);
    }

    #[test]
    fn test_select_prompt_home_end() {
        let options = vec![
            "Option 1".to_string(),
            "Option 2".to_string(),
            "Option 3".to_string(),
            "Option 4".to_string(),
            "Option 5".to_string(),
        ];

        let mut prompt = SelectPrompt::new("Choose:", options);
        prompt.selected = 3;

        // Home
        prompt.handle_key(create_key_event(KeyCode::Home));
        assert_eq!(prompt.selected(), 0);

        // End
        prompt.handle_key(create_key_event(KeyCode::End));
        assert_eq!(prompt.selected(), 4);
    }

    #[test]
    fn test_select_prompt_empty_options() {
        let options: Vec<String> = vec![];
        let prompt = SelectPrompt::new("Choose:", options);

        assert_eq!(prompt.selected(), 0);
        assert!(prompt.selected_text().is_none());
        assert_eq!(prompt.options().len(), 0);
    }

    #[test]
    fn test_input_prompt_no_result() {
        let mut prompt = InputPrompt::new("Enter text:");

        // Unknown key should return None
        let result = prompt.handle_key(create_key_event(KeyCode::F(1)));
        assert_eq!(result, InputResult::None);
    }

    #[test]
    fn test_select_prompt_no_result() {
        let options = vec!["Option 1".to_string()];
        let mut prompt = SelectPrompt::new("Choose:", options);

        // Unknown key should return None
        let result = prompt.handle_key(create_key_event(KeyCode::F(1)));
        assert_eq!(result, SelectResult::None);
    }
}
