use termimad::MadSkin;

pub struct MarkdownRenderer {
    skin: MadSkin,
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

        Self {
            skin: MadSkin::default(),
            width,
        }
    }

    pub fn render(&self, markdown: &str) -> String {
        let skin = &self.skin;
        let text = skin.inline(markdown);
        text.to_string()
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
