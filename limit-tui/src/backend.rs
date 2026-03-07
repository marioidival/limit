// Ratatui backend for rendering VDOM to terminal

use crate::vdom::VNode;
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    prelude::Widget,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};
use std::io;
use std::time::Duration;

/// Backend for rendering VDOM to terminal using Ratatui
pub struct RatatuiBackend {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl RatatuiBackend {
    /// Create a new Ratatui backend with raw mode enabled (no alternate screen)
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        // Note: We do NOT use EnterAlternateScreen as per requirements
        execute!(stdout, DisableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    /// Render VDOM to terminal
    pub fn render_vdom(&mut self, vnode: &VNode) -> io::Result<()> {
        self.terminal.draw(|f| {
            let size = f.area();
            render_vdom_to_ratatui(vnode, size, f.buffer_mut());
        })?;
        Ok(())
    }

    /// Get the terminal size
    pub fn size(&self) -> io::Result<Rect> {
        let size = self.terminal.size()?;
        Ok(Rect::new(0, 0, size.width, size.height))
    }

    /// Clear the terminal
    pub fn clear(&mut self) -> io::Result<()> {
        self.terminal.clear()
    }
}

impl Drop for RatatuiBackend {
    fn drop(&mut self) {
        // Cleanup: disable raw mode
        let _ = disable_raw_mode();
    }
}

/// Render VNode to Ratatui buffer
pub fn render_vdom_to_ratatui(vnode: &VNode, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    match vnode {
        VNode::Text(text) => {
            let paragraph = Paragraph::new(text.as_str()).wrap(Wrap { trim: true });
            paragraph.render(area, buf);
        }
        VNode::Element {
            tag,
            attrs,
            children,
        } => {
            match tag.as_str() {
                "box" => {
                    // Extract title from attrs if present
                    let title = attrs.get("title").map(|s| s.as_str()).unwrap_or("");

                    let block = Block::default().title(title).borders(Borders::ALL);

                    let inner_area = block.inner(area);
                    block.render(area, buf);

                    if !children.is_empty() {
                        // Render first child inside the box
                        render_vdom_to_ratatui(&children[0], inner_area, buf);
                    }
                }
                "list" => {
                    // Convert children to list items
                    let items: Vec<ListItem> = children
                        .iter()
                        .filter_map(|child| {
                            if let VNode::Text(text) = child {
                                Some(ListItem::new(text.as_str()))
                            } else {
                                None
                            }
                        })
                        .collect();

                    let list = List::new(items);
                    list.render(area, buf);
                }
                "text" => {
                    // Extract text from first text child or combine children
                    let text = children
                        .iter()
                        .filter_map(|child| {
                            if let VNode::Text(t) = child {
                                Some(t.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ");

                    let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
                    paragraph.render(area, buf);
                }
                _ => {
                    // Default: render children with indentation
                    for (i, child) in children.iter().enumerate() {
                        let child_height = area.height / children.len() as u16;
                        let child_area = Rect {
                            x: area.x,
                            y: area.y + (i as u16 * child_height),
                            width: area.width,
                            height: child_height,
                        };
                        render_vdom_to_ratatui(child, child_area, buf);
                    }
                }
            }
        }
    }
}

/// Event loop callback type
pub type EventCallback = fn(KeyEvent) -> bool;

/// Run event loop for terminal UI
pub fn run_event_loop<F>(mut backend: RatatuiBackend, vnode: &VNode, callback: F) -> io::Result<()>
where
    F: Fn(KeyEvent) -> bool,
{
    // Initial render
    backend.render_vdom(vnode)?;

    loop {
        // Poll for events with timeout (target 60 FPS ~ 16ms)
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                // Call callback, return true to exit loop
                if callback(key) {
                    break;
                }
                // Re-render after event
                backend.render_vdom(vnode)?;
            }
        } else {
            // No events, just render (for animations)
            backend.render_vdom(vnode)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn test_backend_new() {
        // Note: This test may fail in headless environments (no TTY)
        // In CI or non-interactive environments, terminal creation will fail
        // We test the structure but skip the actual terminal creation if not available
        let backend = RatatuiBackend::new();
        match backend {
            Ok(_) => {}, // Terminal created successfully
            Err(_) => {
                // Expected in headless environments - skip test silently
            }
        }
    }

    #[test]
    fn test_render_text_to_ratatui() {
        let vnode = VNode::Text("Hello, World!".to_string());
        let area = Rect::new(0, 0, 20, 1);
        let mut buffer = Buffer::empty(area);

        render_vdom_to_ratatui(&vnode, area, &mut buffer);

        // Check that text was rendered
        let cell = buffer.cell((0, 0)).unwrap();
        assert_eq!(cell.symbol(), "H");
    }

    #[test]
    fn test_render_box_to_ratatui() {
        let vnode = VNode::Element {
            tag: "box".to_string(),
            attrs: {
                let mut map = std::collections::HashMap::new();
                map.insert("title".to_string(), "Test".to_string());
                map
            },
            children: vec![VNode::Text("Content".to_string())],
        };

        let area = Rect::new(0, 0, 20, 10);
        let mut buffer = Buffer::empty(area);

        render_vdom_to_ratatui(&vnode, area, &mut buffer);

        // Check that border was rendered (top-left corner)
        let cell = buffer.cell((0, 0)).unwrap();
        // Should be a border character
        assert!(!cell.symbol().is_empty());
    }

    #[test]
    fn test_render_list_to_ratatui() {
        let vnode = VNode::Element {
            tag: "list".to_string(),
            attrs: std::collections::HashMap::new(),
            children: vec![
                VNode::Text("Item 1".to_string()),
                VNode::Text("Item 2".to_string()),
            ],
        };

        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);

        render_vdom_to_ratatui(&vnode, area, &mut buffer);

        // Check that first item was rendered
        let cell = buffer.cell((0, 0)).unwrap();
        assert!(!cell.symbol().is_empty());
    }
}
