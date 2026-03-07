// Ratatui Demo Example
//
// Run with: cargo run --package limit-tui --example ratatui_demo
//
// Press 'q' to exit

use crossterm::event::KeyCode;
use limit_tui::backend::{run_event_loop, RatatuiBackend};
use limit_tui::vdom::VNode;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Ratatui Demo...");
    println!("Press 'q' to exit\n");

    // Create a VDOM tree with a text node and a box element
    let vnode = VNode::Element {
        tag: "box".to_string(),
        attrs: {
            let mut map = HashMap::new();
            map.insert("title".to_string(), "Ratatui Demo".to_string());
            map
        },
        children: vec![
            VNode::Text("Welcome to limit-tui with Ratatui!".to_string()),
            VNode::Text("This is a simple demo showing VDOM rendering.".to_string()),
            VNode::Element {
                tag: "list".to_string(),
                attrs: HashMap::new(),
                children: vec![
                    VNode::Text("✓ Text node rendering".to_string()),
                    VNode::Text("✓ Box with borders".to_string()),
                    VNode::Text("✓ List widget".to_string()),
                    VNode::Text("✓ Press 'q' to quit".to_string()),
                ],
            },
        ],
    };

    // Create backend
    let backend = RatatuiBackend::new()?;

    // Event loop callback
    let callback = |key: crossterm::event::KeyEvent| -> bool {
        match key.code {
            KeyCode::Char('q') => {
                println!("\nExiting...");
                true // Exit loop
            }
            _ => false,
        }
    };

    // Run event loop
    run_event_loop(backend, &vnode, callback)?;

    Ok(())
}
