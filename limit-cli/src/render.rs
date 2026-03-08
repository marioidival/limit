use crate::syntax::SyntaxHighlighter;
use std::io::Write;
use termimad::MadSkin;

pub struct MarkdownRenderer {
    skin: MadSkin,
    highlighter: SyntaxHighlighter,
    #[allow(dead_code)]
    width: usize,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        let width = crossterm::terminal::size()
            .map(|(cols, _)| cols as usize)
            .unwrap_or(80);

        let highlighter =
            SyntaxHighlighter::new().expect("Failed to initialize syntax highlighter");

        Self {
            skin: MadSkin::default(),
            highlighter,
            width,
        }
    }

    pub fn render(&self, markdown: &str) -> String {
        // Pre-process code blocks with syntax highlighting
        let processed = self.process_code_blocks(markdown);
        self.skin.inline(&processed).to_string()
    }

    /// Process code blocks and apply syntax highlighting
    fn process_code_blocks(&self, markdown: &str) -> String {
        let mut result = String::new();
        let mut lines = markdown.lines().peekable();

        while let Some(line) = lines.next() {
            if line.starts_with("```") {
                // Start of code block
                let lang = line.strip_prefix("```").unwrap_or("").trim();

                // Read code content
                let mut code_content = String::new();
                while let Some(next_line) = lines.peek() {
                    if next_line.starts_with("```") {
                        lines.next(); // Consume the closing ```
                        break;
                    }
                    code_content.push_str(next_line);
                    code_content.push('\n');
                    lines.next();
                }

                // Apply syntax highlighting
                match self.highlighter.highlight_to_ansi(&code_content, lang) {
                    Ok(highlighted) => {
                        result.push_str(&highlighted);
                    }
                    Err(_) => {
                        // Fallback to code without highlighting
                        result.push_str(&code_content);
                    }
                }
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        result
    }

    /// Render and print markdown directly to stdout
    pub fn render_and_print(&self, markdown: &str) -> Result<(), std::io::Error> {
        let text = self.render(markdown);
        println!("{}", text);
        std::io::stdout().flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_rendering() {
        let renderer = MarkdownRenderer::new();
        let markdown = "# Hello\n**bold** and *italic*";
        let rendered = renderer.render(markdown);
        assert!(!rendered.is_empty());
    }

    #[test]
    fn test_code_block() {
        let renderer = MarkdownRenderer::new();
        let markdown = "```rust\nfn main() {}\n```";
        let rendered = renderer.render(markdown);
        assert!(!rendered.is_empty());
    }

    #[test]
    fn test_list() {
        let renderer = MarkdownRenderer::new();
        let markdown = "- item 1\n- item 2";
        let rendered = renderer.render(markdown);
        assert!(!rendered.is_empty());
    }

    #[test]
    fn test_link() {
        let renderer = MarkdownRenderer::new();
        let markdown = "[link](https://example.com)";
        let rendered = renderer.render(markdown);
        assert!(!rendered.is_empty());
    }
}
