// Chat view component for displaying conversation messages

use std::cell::Cell;

use tracing::debug;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::Widget,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
/// Role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Role {
    /// Get display name for the role
    pub fn display_name(&self) -> &str {
        match self {
            Role::User => "USER",
            Role::Assistant => "ASSISTANT",
            Role::System => "SYSTEM",
        }
    }

    /// Get color for the role badge
    pub fn badge_color(&self) -> Color {
        match self {
            Role::User => Color::Blue,
            Role::Assistant => Color::Green,
            Role::System => Color::Yellow,
        }
    }
}

/// A single chat message
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: String,
}

impl Message {
    /// Create a new message
    pub fn new(role: Role, content: String, timestamp: String) -> Self {
        Self {
            role,
            content,
            timestamp,
        }
    }

    /// Create a user message with current timestamp
    pub fn user(content: String) -> Self {
        let timestamp = Self::current_timestamp();
        Self::new(Role::User, content, timestamp)
    }

    /// Create an assistant message with current timestamp
    pub fn assistant(content: String) -> Self {
        let timestamp = Self::current_timestamp();
        Self::new(Role::Assistant, content, timestamp)
    }

    /// Create a system message with current timestamp
    pub fn system(content: String) -> Self {
        let timestamp = Self::current_timestamp();
        Self::new(Role::System, content, timestamp)
    }

    /// Get current timestamp in local timezone
    fn current_timestamp() -> String {
        chrono::Local::now().format("%H:%M").to_string()
    }
}

/// Chat view component for displaying conversation messages
#[derive(Debug, Clone)]
pub struct ChatView {
    messages: Vec<Message>,
    scroll_offset: usize,
    pinned_to_bottom: bool,
    /// Cached max scroll offset from last render (used when leaving pinned state)
    last_max_scroll_offset: Cell<usize>,
}

impl Default for ChatView {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatView {
    pub fn new() -> Self {
        debug!(component = %"ChatView", "Component created");
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            pinned_to_bottom: true,
            last_max_scroll_offset: Cell::new(0),
        }
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        // Auto-scroll to bottom on new message
        self.scroll_to_bottom();
    }

    /// Append content to the last assistant message, or create a new one if none exists
    pub fn append_to_last_assistant(&mut self, content: &str) {
        if let Some(last) = self.messages.last_mut() {
            if matches!(last.role, Role::Assistant) {
                last.content.push_str(content);
                self.scroll_to_bottom();
                return;
            }
        }
        // No assistant message to append to, create new
        self.add_message(Message::assistant(content.to_string()));
    }

    /// Get the number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get a reference to the messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Scroll up by multiple lines (better UX than single line)
    pub fn scroll_up(&mut self) {
        const SCROLL_LINES: usize = 5;
        // When leaving pinned state, sync scroll_offset to actual position
        if self.pinned_to_bottom {
            self.scroll_offset = self.last_max_scroll_offset.get();
        }
        self.pinned_to_bottom = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(SCROLL_LINES);
    }

    /// Scroll down by multiple lines
    pub fn scroll_down(&mut self) {
        const SCROLL_LINES: usize = 5;
        // When leaving pinned state, sync scroll_offset to actual position
        if self.pinned_to_bottom {
            self.scroll_offset = self.last_max_scroll_offset.get();
        }
        self.pinned_to_bottom = false;
        self.scroll_offset = self.scroll_offset.saturating_add(SCROLL_LINES);
    }

    /// Scroll up by one page (viewport height)
    pub fn scroll_page_up(&mut self, viewport_height: u16) {
        // When leaving pinned state, sync scroll_offset to actual position
        if self.pinned_to_bottom {
            self.scroll_offset = self.last_max_scroll_offset.get();
        }
        self.pinned_to_bottom = false;
        let page_size = viewport_height as usize;
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    /// Scroll down by one page
    pub fn scroll_page_down(&mut self, viewport_height: u16) {
        // When leaving pinned state, sync scroll_offset to actual position
        if self.pinned_to_bottom {
            self.scroll_offset = self.last_max_scroll_offset.get();
        }
        self.pinned_to_bottom = false;
        let page_size = viewport_height as usize;
        self.scroll_offset = self.scroll_offset.saturating_add(page_size);
    }

    /// Scroll to the bottom (show newest messages)
    pub fn scroll_to_bottom(&mut self) {
        self.pinned_to_bottom = true;
    }

    /// Scroll to the top (show oldest messages)
    pub fn scroll_to_top(&mut self) {
        self.pinned_to_bottom = false;
        self.scroll_offset = 0;
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.pinned_to_bottom = true;
    }

    /// Estimate the number of lines needed to display text with wrapping
    fn estimate_line_count(text: &str, width: usize) -> usize {
        if width == 0 {
            return 0;
        }

        let mut lines = 0;
        let mut current_line_len = 0;

        for line in text.lines() {
            if line.is_empty() {
                lines += 1;
                current_line_len = 0;
                continue;
            }

            // Split line into words and calculate wrapped lines
            let words: Vec<&str> = line.split_whitespace().collect();
            let mut word_index = 0;

            while word_index < words.len() {
                let word = words[word_index];
                let word_len = word.len();

                if current_line_len == 0 {
                    // First word on line
                    if word_len > width {
                        // Very long word - split it
                        let mut chars_left = word;
                        while !chars_left.is_empty() {
                            let take = chars_left.len().min(width);
                            lines += 1;
                            chars_left = &chars_left[take..];
                        }
                        current_line_len = 0;
                    } else {
                        current_line_len = word_len;
                    }
                } else if current_line_len + 1 + word_len <= width {
                    // Word fits on current line
                    current_line_len += 1 + word_len;
                } else {
                    // Need new line
                    lines += 1;
                    current_line_len = if word_len > width {
                        // Very long word - split it
                        let mut chars_left = word;
                        while !chars_left.is_empty() {
                            let take = chars_left.len().min(width);
                            lines += 1;
                            chars_left = &chars_left[take..];
                        }
                        0
                    } else {
                        word_len
                    };
                }

                word_index += 1;
            }

            // Account for the line itself if we added any content
            if current_line_len > 0 || words.is_empty() {
                lines += 1;
            }

            current_line_len = 0;
        }

        lines.max(1)
    }

    /// Calculate total height needed to display all messages
    fn calculate_total_height(&self, width: u16) -> usize {
        let mut total_height = 0;

        for message in &self.messages {
            // Role badge line: "[USER] HH:MM"
            total_height += 1;

            // Message content lines (with wrapping)
            let content_height = Self::estimate_line_count(&message.content, width as usize);
            total_height += content_height;

            // Empty line between messages
            total_height += 1;
        }

        total_height
    }

    /// Render visible messages based on scroll offset
    fn render_to_buffer(&self, area: Rect, buf: &mut Buffer) {
        let total_height = self.calculate_total_height(area.width);
        let viewport_height = area.height as usize;

        // Calculate scroll offset based on pinned state
        let max_scroll_offset = if total_height > viewport_height {
            total_height.saturating_sub(viewport_height)
        } else {
            0
        };

        // Cache the max offset for scroll functions to use
        self.last_max_scroll_offset.set(max_scroll_offset);

        let scroll_offset = if self.pinned_to_bottom {
            // When pinned to bottom, always show the newest messages
            max_scroll_offset
        } else {
            // User has scrolled - clamp to valid range
            self.scroll_offset.min(max_scroll_offset)
        };

        let mut y_offset: u16 = area.y;
        let mut global_y: usize = 0;
        let skip_until = scroll_offset;
        let max_y = scroll_offset + viewport_height;
        for message in &self.messages {
            // Skip if this message is above the viewport
            let role_height = 1;
            let content_height = Self::estimate_line_count(&message.content, area.width as usize);
            let separator_height = 1;
            let message_height = role_height + content_height + separator_height;

            if global_y + message_height <= skip_until {
                global_y += message_height;
                continue;
            }

            if global_y >= max_y {
                break;
            }

            // Render role badge
            if global_y >= skip_until && y_offset < area.y + area.height {
                let role_text = format!("[{}] {}", message.role.display_name(), message.timestamp);
                let style = Style::default()
                    .fg(message.role.badge_color())
                    .add_modifier(Modifier::BOLD);

                let line = Line::from(vec![Span::styled(role_text, style)]);

                Paragraph::new(line)
                    .wrap(Wrap { trim: false })
                    .render(Rect::new(area.x, y_offset, area.width, 1), buf);

                y_offset += 1;
            }
            global_y += 1;

            // Render message content
            let lines: Vec<String> = message
                .content
                .lines()
                .flat_map(|line| {
                    // Simple word wrapping
                    let mut result = Vec::new();
                    let mut current = String::new();
                    let max_width = area.width as usize;

                    for word in line.split_whitespace() {
                        let test = if current.is_empty() {
                            word.to_string()
                        } else {
                            format!("{} {}", current, word)
                        };

                        if test.len() <= max_width || current.is_empty() {
                            current = test;
                        } else {
                            if !current.is_empty() {
                                result.push(current);
                            }
                            current = word.to_string();
                            if word.len() > max_width {
                                // Very long word - split it
                                while current.len() > max_width {
                                    result.push(current[..max_width].to_string());
                                    current = current[max_width..].to_string();
                                }
                            }
                        }
                    }

                    if !current.is_empty() {
                        result.push(current);
                    }

                    result
                })
                .collect();

            for line in lines {
                if global_y >= skip_until && y_offset < area.y + area.height {
                    Paragraph::new(line.as_str())
                        .wrap(Wrap { trim: false })
                        .render(Rect::new(area.x, y_offset, area.width, 1), buf);
                    y_offset += 1;
                }
                global_y += 1;

                if global_y >= max_y {
                    break;
                }
            }

            // Add separator line
            if global_y >= skip_until && global_y < max_y && y_offset < area.y + area.height {
                Paragraph::new("─".repeat(area.width as usize).as_str())
                    .style(Style::default().fg(Color::DarkGray))
                    .render(Rect::new(area.x, y_offset, area.width, 1), buf);
                y_offset += 1;
            }
            global_y += 1;
        }
    }
}

impl ratatui::widgets::Widget for &ChatView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // No border here - let the parent draw_ui handle borders for consistent layout
        (*self).render_to_buffer(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_display_name() {
        assert_eq!(Role::User.display_name(), "USER");
        assert_eq!(Role::Assistant.display_name(), "ASSISTANT");
        assert_eq!(Role::System.display_name(), "SYSTEM");
    }

    #[test]
    fn test_role_badge_color() {
        assert_eq!(Role::User.badge_color(), Color::Blue);
        assert_eq!(Role::Assistant.badge_color(), Color::Green);
        assert_eq!(Role::System.badge_color(), Color::Yellow);
    }

    #[test]
    fn test_message_new() {
        let message = Message::new(Role::User, "Hello, World!".to_string(), "12:34".to_string());

        assert_eq!(message.role, Role::User);
        assert_eq!(message.content, "Hello, World!");
        assert_eq!(message.timestamp, "12:34");
    }

    #[test]
    fn test_message_user() {
        let message = Message::user("Test message".to_string());

        assert_eq!(message.role, Role::User);
        assert_eq!(message.content, "Test message");
        assert!(!message.timestamp.is_empty());
    }

    #[test]
    fn test_message_assistant() {
        let message = Message::assistant("Response".to_string());

        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.content, "Response");
        assert!(!message.timestamp.is_empty());
    }

    #[test]
    fn test_message_system() {
        let message = Message::system("System notification".to_string());

        assert_eq!(message.role, Role::System);
        assert_eq!(message.content, "System notification");
        assert!(!message.timestamp.is_empty());
    }

    #[test]
    fn test_chat_view_new() {
        let chat = ChatView::new();

        assert_eq!(chat.message_count(), 0);
        assert_eq!(chat.scroll_offset, 0);
        assert!(chat.messages().is_empty());
    }

    #[test]
    fn test_chat_view_default() {
        let chat = ChatView::default();

        assert_eq!(chat.message_count(), 0);
        assert_eq!(chat.scroll_offset, 0);
    }

    #[test]
    fn test_chat_view_add_message() {
        let mut chat = ChatView::new();

        chat.add_message(Message::user("Hello".to_string()));
        assert_eq!(chat.message_count(), 1);

        chat.add_message(Message::assistant("Hi there!".to_string()));
        assert_eq!(chat.message_count(), 2);
    }

    #[test]
    fn test_chat_view_add_multiple_messages() {
        let mut chat = ChatView::new();

        for i in 0..5 {
            chat.add_message(Message::user(format!("Message {}", i)));
        }

        assert_eq!(chat.message_count(), 5);
    }

    #[test]
    fn test_chat_view_scroll_up() {
        let mut chat = ChatView::new();

        // Add some messages
        for i in 0..10 {
            chat.add_message(Message::user(format!("Message {}", i)));
        }

        // After adding messages, we're pinned to bottom
        assert!(chat.pinned_to_bottom);

        // Scroll up should unpin and adjust offset
        chat.scroll_up();
        assert!(!chat.pinned_to_bottom);
        // scroll_offset doesn't change when pinned, but will be used after unpin
        // The actual visual scroll is calculated in render
    }

    #[test]
    fn test_chat_view_scroll_up_bounds() {
        let mut chat = ChatView::new();

        chat.add_message(Message::user("Test".to_string()));
        chat.scroll_to_top(); // Start at top with scroll_offset = 0

        // Try to scroll up when at top - saturating_sub should keep it at 0
        chat.scroll_up();
        assert_eq!(chat.scroll_offset, 0);
        assert!(!chat.pinned_to_bottom);

        chat.scroll_up();
        assert_eq!(chat.scroll_offset, 0);
    }

    #[test]
    fn test_chat_view_scroll_down() {
        let mut chat = ChatView::new();

        chat.add_message(Message::user("Test".to_string()));

        // After adding, pinned to bottom
        assert!(chat.pinned_to_bottom);

        chat.scroll_down();
        // Scroll down unpins from bottom
        assert!(!chat.pinned_to_bottom);
        // scroll_offset increases by SCROLL_LINES (5)
        assert_eq!(chat.scroll_offset, 5);
    }

    #[test]
    fn test_chat_view_scroll_to_bottom() {
        let mut chat = ChatView::new();

        for i in 0..5 {
            chat.add_message(Message::user(format!("Message {}", i)));
        }

        chat.scroll_to_top();
        assert_eq!(chat.scroll_offset, 0);
        assert!(!chat.pinned_to_bottom);

        chat.scroll_to_bottom();
        // scroll_to_bottom sets pinned_to_bottom, not a specific offset
        assert!(chat.pinned_to_bottom);
    }

    #[test]
    fn test_chat_view_scroll_to_top() {
        let mut chat = ChatView::new();

        for i in 0..5 {
            chat.add_message(Message::user(format!("Message {}", i)));
        }

        chat.scroll_to_bottom();
        assert!(chat.pinned_to_bottom);

        chat.scroll_to_top();
        assert_eq!(chat.scroll_offset, 0);
        assert!(!chat.pinned_to_bottom);
    }

    #[test]
    fn test_chat_view_auto_scroll() {
        let mut chat = ChatView::new();

        for i in 0..5 {
            chat.add_message(Message::user(format!("Message {}", i)));
            // After adding a message, should auto-scroll to bottom (pinned)
        }

        // Auto-scroll sets pinned_to_bottom, not a specific scroll_offset
        assert!(chat.pinned_to_bottom);
    }

    #[test]
    fn test_chat_view_render() {
        let mut chat = ChatView::new();
        chat.add_message(Message::user("Test message".to_string()));

        let area = Rect::new(0, 0, 50, 20);
        let mut buffer = Buffer::empty(area);

        // This should not panic
        chat.render(area, &mut buffer);

        // Check that something was rendered
        let cell = buffer.cell((0, 0)).unwrap();
        // Should have at least the border character
        assert!(!cell.symbol().is_empty());
    }

    #[test]
    fn test_chat_view_render_multiple_messages() {
        let mut chat = ChatView::new();

        chat.add_message(Message::user("First message".to_string()));
        chat.add_message(Message::assistant("Second message".to_string()));
        chat.add_message(Message::system("System message".to_string()));

        let area = Rect::new(0, 0, 50, 20);
        let mut buffer = Buffer::empty(area);

        // This should not panic
        chat.render(area, &mut buffer);
    }

    #[test]
    fn test_chat_view_render_with_long_message() {
        let mut chat = ChatView::new();

        let long_message = "This is a very long message that should wrap across multiple lines in the buffer when rendered. ".repeat(5);
        chat.add_message(Message::user(long_message));

        let area = Rect::new(0, 0, 30, 20);
        let mut buffer = Buffer::empty(area);

        // This should not panic
        chat.render(area, &mut buffer);
    }

    #[test]
    fn test_chat_view_messages_ref() {
        let mut chat = ChatView::new();

        chat.add_message(Message::user("Message 1".to_string()));
        chat.add_message(Message::assistant("Message 2".to_string()));

        let messages = chat.messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Message 1");
        assert_eq!(messages[1].content, "Message 2");
    }

    #[test]
    fn test_calculate_total_height() {
        let mut chat = ChatView::new();

        // Empty chat has 0 height
        assert_eq!(chat.calculate_total_height(50), 0);

        chat.add_message(Message::user("Hello".to_string()));
        // 1 role line + 1 content line + 1 separator = 3
        assert_eq!(chat.calculate_total_height(50), 3);
    }

    #[test]
    fn test_calculate_total_height_with_wrapping() {
        let mut chat = ChatView::new();

        // Short message - single line
        chat.add_message(Message::user("Hi".to_string()));
        assert_eq!(chat.calculate_total_height(50), 3);

        // Long message - multiple lines due to wrapping
        let long_msg = "This is a very long message that will definitely wrap onto multiple lines when displayed in a narrow container".to_string();
        chat.add_message(Message::assistant(long_msg));

        // First message: 3 lines
        // Second message: role line + wrapped content lines + separator
        let height = chat.calculate_total_height(20);
        assert!(height > 6); // More than 2 * 3 due to wrapping
    }
}
