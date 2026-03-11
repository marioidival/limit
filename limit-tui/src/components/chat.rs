// Chat view component for displaying conversation messages

use std::cell::{Cell, RefCell};

use crate::syntax::SyntaxHighlighter;
use tracing::debug;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::Widget,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

/// Maximum number of messages to render at once (sliding window)
const RENDER_WINDOW_SIZE: usize = 50;

/// Convert character offset to byte offset for UTF-8 safe slicing
fn char_offset_to_byte(text: &str, char_offset: usize) -> usize {
    text.char_indices()
        .nth(char_offset)
        .map(|(i, _)| i)
        .unwrap_or(text.len())
}

/// Line type for markdown rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineType {
    Normal,
    Header1,
    Header2,
    Header3,
    ListItem,
    CodeBlock,
}

impl LineType {
    fn style(&self) -> Style {
        match self {
            LineType::Header1 => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            LineType::Header2 => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            LineType::Header3 => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            LineType::ListItem => Style::default().fg(Color::White),
            LineType::CodeBlock => Style::default().fg(Color::Gray),
            LineType::Normal => Style::default(),
        }
    }
}

/// Parse inline markdown elements and return styled spans
fn parse_inline_markdown(text: &str, base_style: Style) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current = String::new();
    let mut in_bold = false;
    let mut in_italic = false;
    let mut in_code = false;

    while let Some(c) = chars.next() {
        // Handle code inline: `code`
        if c == '`' && !in_bold && !in_italic {
            if in_code {
                // End of code
                let style = Style::default().fg(Color::Yellow);
                spans.push(Span::styled(current.clone(), style));
                current.clear();
                in_code = false;
            } else {
                // Start of code
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), base_style));
                    current.clear();
                }
                in_code = true;
            }
            continue;
        }

        // Handle bold: **text**
        if c == '*' && chars.peek() == Some(&'*') && !in_code {
            chars.next(); // consume second *
            if in_bold {
                // End of bold
                let style = base_style.add_modifier(Modifier::BOLD);
                spans.push(Span::styled(current.clone(), style));
                current.clear();
                in_bold = false;
            } else {
                // Start of bold
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), base_style));
                    current.clear();
                }
                in_bold = true;
            }
            continue;
        }

        // Handle italic: *text* (single asterisk, not at start/end of word boundary with bold)
        if c == '*' && !in_code && !in_bold {
            if in_italic {
                // End of italic
                let style = base_style.add_modifier(Modifier::ITALIC);
                spans.push(Span::styled(current.clone(), style));
                current.clear();
                in_italic = false;
            } else {
                // Start of italic
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), base_style));
                    current.clear();
                }
                in_italic = true;
            }
            continue;
        }

        current.push(c);
    }

    // Handle remaining text
    if !current.is_empty() {
        let style = if in_code {
            Style::default().fg(Color::Yellow)
        } else if in_bold {
            base_style.add_modifier(Modifier::BOLD)
        } else if in_italic {
            base_style.add_modifier(Modifier::ITALIC)
        } else {
            base_style
        };
        spans.push(Span::styled(current, style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(text, base_style));
    }

    spans
}

/// Detect line type from content
fn detect_line_type(line: &str) -> (LineType, &str) {
    let trimmed = line.trim_start();
    if trimmed.starts_with("### ") {
        (
            LineType::Header3,
            trimmed.strip_prefix("### ").unwrap_or(trimmed),
        )
    } else if trimmed.starts_with("## ") {
        (
            LineType::Header2,
            trimmed.strip_prefix("## ").unwrap_or(trimmed),
        )
    } else if trimmed.starts_with("# ") {
        (
            LineType::Header1,
            trimmed.strip_prefix("# ").unwrap_or(trimmed),
        )
    } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        (LineType::ListItem, line)
    } else {
        (LineType::Normal, line)
    }
}

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

/// Position metadata for mapping screen coordinates to text positions
#[derive(Debug, Clone, Copy)]
pub struct RenderPosition {
    /// Index of the message in the messages vector
    pub message_idx: usize,
    /// Index of the line within the message content
    pub line_idx: usize,
    /// Character offset where this screen line starts
    pub char_start: usize,
    /// Character offset where this screen line ends
    pub char_end: usize,
    /// Absolute screen Y coordinate
    pub screen_row: u16,
}

/// Chat view component for displaying conversation messages
#[derive(Debug, Clone)]
pub struct ChatView {
    messages: Vec<Message>,
    scroll_offset: usize,
    pinned_to_bottom: bool,
    /// Cached max scroll offset from last render (used when leaving pinned state)
    last_max_scroll_offset: Cell<usize>,
    /// Syntax highlighter for code blocks
    highlighter: SyntaxHighlighter,
    /// Cached height for render performance
    cached_height: Cell<usize>,
    /// Cache dirty flag - set to true when content changes
    cache_dirty: Cell<bool>,
    /// Number of hidden messages when using sliding window
    hidden_message_count: Cell<usize>,
    /// Text selection state: (message_idx, char_offset)
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    /// Render position metadata for mouse-to-text mapping
    render_positions: RefCell<Vec<RenderPosition>>,
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
            highlighter: SyntaxHighlighter::new().expect("Failed to initialize syntax highlighter"),
            cache_dirty: Cell::new(true),
            cached_height: Cell::new(0),
            hidden_message_count: Cell::new(0),
            selection_start: None,
            selection_end: None,
            render_positions: RefCell::new(Vec::new()),
        }
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.cache_dirty.set(true); // Invalidate cache when message is added
        // Auto-scroll to bottom on new message
        self.scroll_to_bottom();
    }

    /// Append content to the last assistant message, or create a new one if none exists
    pub fn append_to_last_assistant(&mut self, content: &str) {
        if let Some(last) = self.messages.last_mut() {
            if matches!(last.role, Role::Assistant) {
                last.content.push_str(content);
                self.cache_dirty.set(true); // Invalidate cache on content change
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
        // If pinned to bottom, sync scroll_offset before scrolling up
        if self.pinned_to_bottom {
            self.scroll_offset = self.last_max_scroll_offset.get();
            self.pinned_to_bottom = false;
        }
        self.scroll_offset = self.scroll_offset.saturating_sub(SCROLL_LINES);
        // Invalidate cache since window size changed
        self.cache_dirty.set(true);
    }

    /// Scroll down by multiple lines
    pub fn scroll_down(&mut self) {
        const SCROLL_LINES: usize = 5;
        // If pinned to bottom, trying to scroll down does nothing (already at bottom)
        if self.pinned_to_bottom {
            return;
        }
        self.scroll_offset = self.scroll_offset.saturating_add(SCROLL_LINES);
    }

    /// Scroll up by one page (viewport height)
    pub fn scroll_page_up(&mut self, viewport_height: u16) {
        // If pinned to bottom, sync scroll_offset before scrolling up
        if self.pinned_to_bottom {
            self.scroll_offset = self.last_max_scroll_offset.get();
            self.pinned_to_bottom = false;
        }
        let page_size = viewport_height as usize;
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    /// Scroll down by one page
    pub fn scroll_page_down(&mut self, viewport_height: u16) {
        // If pinned to bottom, trying to scroll down does nothing (already at bottom)
        if self.pinned_to_bottom {
            return;
        }
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

    /// Start text selection at position
    pub fn start_selection(&mut self, message_idx: usize, byte_offset: usize) {
        self.selection_start = Some((message_idx, byte_offset));
        self.selection_end = Some((message_idx, byte_offset));
    }

    /// Extend selection to position
    pub fn extend_selection(&mut self, message_idx: usize, byte_offset: usize) {
        if self.selection_start.is_some() {
            self.selection_end = Some((message_idx, byte_offset));
        }
    }

    /// Clear text selection
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }

    /// Check if there is an active selection
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }

    /// Map screen coordinates to text position for mouse selection
    /// Returns (message_idx, char_offset) if a valid position is found
    pub fn screen_to_text_pos(&self, col: u16, row: u16) -> Option<(usize, usize)> {
        for pos in self.render_positions.borrow().iter() {
            if pos.screen_row == row {
                // Calculate character offset within the line based on column
                // (assumes monospace font - accurate for terminal)
                let line_len = pos.char_end.saturating_sub(pos.char_start);
                let char_in_line = (col as usize).min(line_len);
                return Some((pos.message_idx, pos.char_start + char_in_line));
            }
        }
        None
    }

    /// Check if a byte position is within the current selection
    pub fn is_selected(&self, message_idx: usize, char_offset: usize) -> bool {
        let Some((start_msg, start_offset)) = self.selection_start else {
            return false;
        };
        let Some((end_msg, end_offset)) = self.selection_end else {
            return false;
        };

        // Normalize order
        let (min_msg, min_offset, max_msg, max_offset) =
            if start_msg < end_msg || (start_msg == end_msg && start_offset <= end_offset) {
                (start_msg, start_offset, end_msg, end_offset)
            } else {
                (end_msg, end_offset, start_msg, start_offset)
            };

        // Check if position is in selection range
        if message_idx < min_msg || message_idx > max_msg {
            return false;
        }

        if message_idx == min_msg && message_idx == max_msg {
            // Same message: check offset range
            char_offset >= min_offset && char_offset < max_offset
        } else if message_idx == min_msg {
            // First message: offset >= min_offset
            char_offset >= min_offset
        } else if message_idx == max_msg {
            // Last message: offset < max_offset
            char_offset < max_offset
        } else {
            // Middle message: fully selected
            true
        }
    }

    /// Apply selection highlighting to text spans
    /// Takes a line of text and returns styled spans with selection highlighted
    fn apply_selection_highlight<'a>(
        &self,
        text: &'a str,
        message_idx: usize,
        line_char_start: usize,
        base_style: Style,
    ) -> Vec<Span<'a>> {
        let selection_style = Style::default().bg(Color::Blue).fg(Color::White);

        // If no selection, just return styled text
        if !self.has_selection() {
            return vec![Span::styled(text, base_style)];
        }

        let mut spans = Vec::new();
        let mut current_start = 0;
        let mut in_selection = false;
        let char_positions: Vec<(usize, char)> = text.char_indices().collect();

        for (i, (byte_idx, _)) in char_positions.iter().enumerate() {
            let global_char = line_char_start + i;
            let is_sel = self.is_selected(message_idx, global_char);

            if is_sel != in_selection {
                // Transition point - push current segment
                if i > current_start {
                    let segment_byte_start = char_positions[current_start].0;
                    let segment_byte_end = *byte_idx;
                    let segment = &text[segment_byte_start..segment_byte_end];
                    let style = if in_selection {
                        selection_style
                    } else {
                        base_style
                    };
                    spans.push(Span::styled(segment, style));
                }
                current_start = i;
                in_selection = is_sel;
            }
        }

        // Push final segment
        if current_start < char_positions.len() {
            let segment_byte_start = char_positions[current_start].0;
            let segment = &text[segment_byte_start..];
            let style = if in_selection {
                selection_style
            } else {
                base_style
            };
            spans.push(Span::styled(segment, style));
        }

        if spans.is_empty() {
            vec![Span::styled(text, base_style)]
        } else {
            spans
        }
    }

    /// Get selected text (character-precise)
    pub fn get_selected_text(&self) -> Option<String> {
        let (start_msg, start_offset) = self.selection_start?;
        let (end_msg, end_offset) = self.selection_end?;

        // Normalize order
        let (min_msg, min_offset, max_msg, max_offset) =
            if start_msg < end_msg || (start_msg == end_msg && start_offset <= end_offset) {
                (start_msg, start_offset, end_msg, end_offset)
            } else {
                (end_msg, end_offset, start_msg, start_offset)
            };

        if min_msg == max_msg {
            // Single message: extract substring using character indices
            let msg = self.messages.get(min_msg)?;
            let content = &msg.content;
            let start_byte = char_offset_to_byte(content, min_offset);
            let end_byte = char_offset_to_byte(content, max_offset);
            if start_byte < content.len() && end_byte <= content.len() {
                Some(content[start_byte..end_byte].to_string())
            } else {
                None
            }
        } else {
            // Multiple messages: collect parts
            let mut result = String::new();

            // First message: from offset to end
            if let Some(msg) = self.messages.get(min_msg) {
                let start_byte = char_offset_to_byte(&msg.content, min_offset);
                if start_byte < msg.content.len() {
                    result.push_str(&msg.content[start_byte..]);
                }
            }

            // Middle messages: full content
            for idx in (min_msg + 1)..max_msg {
                if let Some(msg) = self.messages.get(idx) {
                    result.push('\n');
                    result.push_str(&msg.content);
                }
            }

            // Last message: from start to offset
            if let Some(msg) = self.messages.get(max_msg) {
                result.push('\n');
                let end_byte = char_offset_to_byte(&msg.content, max_offset);
                if end_byte > 0 && end_byte <= msg.content.len() {
                    result.push_str(&msg.content[..end_byte]);
                }
            }

            Some(result)
        }
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.pinned_to_bottom = true;
        self.cache_dirty.set(true);
        self.hidden_message_count.set(0);
    }

    /// Get the render window (sliding window for large sessions)
    /// Returns a slice of messages to render and the count of hidden messages
    fn get_render_window(&self) -> (&[Message], usize) {
        let total_count = self.messages.len();

        // Use sliding window only when pinned to bottom and there are more than RENDER_WINDOW_SIZE messages
        if self.pinned_to_bottom && total_count > RENDER_WINDOW_SIZE {
            let hidden_count = total_count.saturating_sub(RENDER_WINDOW_SIZE);
            let window = &self.messages[hidden_count..];
            self.hidden_message_count.set(hidden_count);
            (window, hidden_count)
        } else {
            self.hidden_message_count.set(0);
            (&self.messages, 0)
        }
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

    /// Process code blocks with syntax highlighting
    /// Returns a vector of (line, line_type, is_code_block, lang)
    fn process_code_blocks(&self, content: &str) -> Vec<(String, LineType, bool, Option<String>)> {
        let mut result = Vec::new();
        let lines = content.lines().peekable();
        let mut in_code_block = false;
        let mut current_lang: Option<String> = None;

        for line in lines {
            if line.starts_with("```") {
                if in_code_block {
                    // End of code block
                    in_code_block = false;
                    current_lang = None;
                } else {
                    // Start of code block
                    in_code_block = true;
                    current_lang = line
                        .strip_prefix("```")
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());
                }
            } else if in_code_block {
                result.push((
                    line.to_string(),
                    LineType::CodeBlock,
                    true,
                    current_lang.clone(),
                ));
            } else {
                let (line_type, _) = detect_line_type(line);
                result.push((line.to_string(), line_type, false, None));
            }
        }

        result
    }

    /// Calculate total height needed to display all messages
    fn calculate_total_height(&self, width: u16) -> usize {
        // Check cache first - return cached value if not dirty
        if !self.cache_dirty.get() {
            return self.cached_height.get();
        }

        let mut total_height = 0;

        // IMPORTANT: Calculate height for ALL messages, not just render window
        // This is needed for correct scroll offset calculation
        for message in &self.messages {
            // Role badge line: "[USER] HH:MM"
            total_height += 1;

            // Message content lines (with wrapping)
            let processed = self.process_code_blocks(&message.content);

            for (line, _line_type, _is_code, _lang) in processed {
                // Code blocks render line-by-line with height 1
                // Regular text wraps to estimated height
                let line_height = if _is_code {
                    1 // Code blocks: one row per line, no wrapping
                } else {
                    Self::estimate_line_count(&line, width as usize)
                };
                total_height += line_height;
            }

            // Empty line between messages
            total_height += 1;
        }

        // Cache result and mark as clean
        self.cached_height.set(total_height);
        self.cache_dirty.set(false);

        total_height
    }

    /// Render visible messages based on scroll offset
    /// Render visible messages based on scroll offset
    fn render_to_buffer(&self, area: Rect, buf: &mut Buffer) {
        // Clear render position metadata for this frame
        self.render_positions.borrow_mut().clear();

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

        // Content should always start at area.y - pinned_to_bottom only affects scroll_offset
        let (initial_y_offset, skip_until, max_y) =
            (area.y, scroll_offset, scroll_offset + viewport_height);

        let mut y_offset = initial_y_offset;
        let mut global_y: usize = 0;

        // Use sliding window for large sessions when pinned to bottom
        let (messages_to_render, hidden_count) = self.get_render_window();

        // When using sliding window, we need to account for hidden messages in global_y
        // This ensures scroll offset calculations work correctly
        if hidden_count > 0 {
            // Calculate approximate height of hidden messages
            // This allows scroll to work correctly even with sliding window
            for message in &self.messages[..hidden_count] {
                let role_height = 1;
                let processed = self.process_code_blocks(&message.content);
                let content_height: usize = processed
                    .iter()
                    .map(|(line, _, _, _)| Self::estimate_line_count(line, area.width as usize))
                    .sum();
                let separator_height = 1;
                global_y += role_height + content_height + separator_height;
            }
        }
        for (local_msg_idx, message) in messages_to_render.iter().enumerate() {
            let message_idx = hidden_count + local_msg_idx;

            // Skip if this message is above the viewport
            let role_height = 1;
            let processed = self.process_code_blocks(&message.content);
            let content_height: usize = processed
                .iter()
                .map(|(line, _, _, _)| Self::estimate_line_count(line, area.width as usize))
                .sum();
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

            // Render message content with markdown and code highlighting
            // Track character offset within the message for selection mapping
            let mut char_offset: usize = 0;
            for (line_idx, (line, line_type, is_code_block, lang)) in processed.iter().enumerate() {
                let line_height = Self::estimate_line_count(line, area.width as usize);
                let line_char_count = line.chars().count();

                // Track this line's render position for mouse selection
                if global_y >= skip_until && y_offset < area.y + area.height {
                    self.render_positions.borrow_mut().push(RenderPosition {
                        message_idx,
                        line_idx,
                        char_start: char_offset,
                        char_end: char_offset + line_char_count,
                        screen_row: y_offset,
                    });
                }

                if *is_code_block && global_y >= skip_until {
                    // Code block with syntax highlighting
                    if let Some(ref lang_str) = lang {
                        if let Ok(highlighted_spans) = self
                            .highlighter
                            .highlight_to_spans(&format!("{}\n", line), lang_str)
                        {
                            // Render highlighted lines
                            for highlighted_line in highlighted_spans {
                                if y_offset < area.y + area.height && global_y < max_y {
                                    let text = Text::from(Line::from(highlighted_line));
                                    Paragraph::new(text)
                                        .wrap(Wrap { trim: false })
                                        .render(Rect::new(area.x, y_offset, area.width, 1), buf);
                                    y_offset += 1;
                                }
                                global_y += 1;

                                if global_y >= max_y {
                                    break;
                                }
                            }
                            continue;
                        }
                    }
                }

                // Regular text with markdown styling and selection highlighting
                let base_style = line_type.style();
                let spans = if self.has_selection() {
                    // Apply selection highlighting on top of markdown styling
                    self.apply_selection_highlight(line, message_idx, char_offset, base_style)
                } else {
                    parse_inline_markdown(line, base_style)
                };
                let text_line = Line::from(spans);

                // Render the line
                if global_y >= skip_until && y_offset < area.y + area.height {
                    // Clamp height to remaining viewport space
                    let render_height =
                        line_height.min((area.y + area.height - y_offset) as usize) as u16;
                    Paragraph::new(text_line)
                        .wrap(Wrap { trim: false })
                        .render(Rect::new(area.x, y_offset, area.width, render_height), buf);
                    y_offset += line_height as u16;
                }
                global_y += line_height;

                // Update character offset for next line
                char_offset += line_char_count + 1; // +1 for newline

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

        // Scroll down when pinned to bottom should do nothing (already at bottom)
        chat.scroll_down();
        assert!(chat.pinned_to_bottom); // Still pinned
        assert_eq!(chat.scroll_offset, 0); // Offset unchanged

        // Scroll up first to unpin
        chat.scroll_up();
        assert!(!chat.pinned_to_bottom);

        // Now scroll down should work
        chat.scroll_down();
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

    #[test]
    fn test_short_content_pinned_to_bottom_should_start_at_top() {
        // Bug: When content is short and pinned to bottom, it incorrectly anchors to bottom
        // causing content to scroll up visually when new content is added
        let mut chat = ChatView::new();

        chat.add_message(Message::user("Hello".to_string()));

        let area = Rect::new(0, 0, 50, 20);
        let mut buffer = Buffer::empty(area);

        // Render the chat
        chat.render(area, &mut buffer);

        // Check that content starts at the top of the area (y=0 relative to inner area)
        // The first line should be the role badge, which should be at y=0 (after border)
        let cell = buffer.cell((0, 0)).unwrap();
        // Should not be empty - should have content
        assert!(
            !cell.symbol().is_empty(),
            "Content should start at top, not be pushed down"
        );
    }

    #[test]
    fn test_streaming_content_stays_pinned() {
        // Bug: When content grows during streaming, it can scroll up unexpectedly
        let mut chat = ChatView::new();

        // Start with short content
        chat.add_message(Message::assistant("Start".to_string()));

        let area = Rect::new(0, 0, 50, 20);
        let mut buffer1 = Buffer::empty(area);
        chat.render(area, &mut buffer1);

        // Add more content (simulating streaming)
        chat.append_to_last_assistant(" and continue with more text that is longer");

        let mut buffer2 = Buffer::empty(area);
        chat.render(area, &mut buffer2);

        // The last line should be visible (near bottom of viewport)
        // Check that content is still visible and not scrolled off-screen
        // Should have some content (not empty)
        let has_content_near_bottom = (0u16..20).any(|y| {
            let c = buffer2.cell((0, y)).unwrap();
            !c.symbol().is_empty() && c.symbol() != "│" && c.symbol() != " "
        });

        assert!(
            has_content_near_bottom,
            "Content should remain visible near bottom when pinned"
        );
    }

    #[test]
    fn test_content_shorter_than_viewport_no_excess_padding() {
        // Bug: When total_height < viewport_height, bottom_padding pushes content down
        let mut chat = ChatView::new();

        chat.add_message(Message::user("Short message".to_string()));

        let total_height = chat.calculate_total_height(50);
        let viewport_height: u16 = 20;

        // Content should fit without needing padding
        assert!(
            total_height < viewport_height as usize,
            "Content should be shorter than viewport"
        );

        let area = Rect::new(0, 0, 50, viewport_height);
        let mut buffer = Buffer::empty(area);

        chat.render(area, &mut buffer);

        // Content should start at y=0 (relative to inner area after border)
        // Find the first non-empty, non-border cell
        let mut first_content_y: Option<u16> = None;
        for y in 0..viewport_height {
            let cell = buffer.cell((0, y)).unwrap();
            let is_border = matches!(
                cell.symbol(),
                "─" | "│" | "┌" | "┐" | "└" | "┘" | "├" | "┤" | "┬" | "┴"
            );
            if !is_border && !cell.symbol().is_empty() {
                first_content_y = Some(y);
                break;
            }
        }

        let first_content_y = first_content_y.expect("Should find content somewhere");

        assert_eq!(
            first_content_y, 0,
            "Content should start at y=0, not be pushed down by padding"
        );
    }

    #[test]
    fn test_pinned_state_after_scrolling() {
        let mut chat = ChatView::new();

        // Add enough messages to fill more than viewport
        for i in 0..10 {
            chat.add_message(Message::user(format!("Message {}", i)));
        }

        // Should be pinned initially
        assert!(chat.pinned_to_bottom);

        // Scroll up
        chat.scroll_up();
        assert!(!chat.pinned_to_bottom);

        // Scroll back down
        chat.scroll_to_bottom();
        assert!(chat.pinned_to_bottom);
    }

    #[test]
    fn test_message_growth_maintains_correct_position() {
        // Simulate scenario where a message grows (streaming response)
        let mut chat = ChatView::new();

        // Add initial message
        chat.add_message(Message::assistant("Initial".to_string()));

        let area = Rect::new(0, 0, 60, 10);
        let mut buffer = Buffer::empty(area);
        chat.render(area, &mut buffer);

        // Grow the message
        chat.append_to_last_assistant(" content that gets added");

        let mut buffer2 = Buffer::empty(area);
        chat.render(area, &mut buffer2);

        // Should still be pinned
        assert!(
            chat.pinned_to_bottom,
            "Should remain pinned after content growth"
        );
    }
}
