# limit-tui

[![Crates.io](https://img.shields.io/crates/v/limit-tui.svg)](https://crates.io/crates/limit-tui)
[![Docs.rs](https://docs.rs/limit-tui/badge.svg)](https://docs.rs/limit-tui)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Terminal UI components with Virtual DOM rendering for Rust applications.**

Build rich terminal interfaces with a React-like component model, powered by [Ratatui](https://ratatui.rs/). Features declarative rendering, diff-based updates, and built-in syntax highlighting.

Part of the [Limit](https://github.com/marioidival/limit) ecosystem.

## Why This Exists

Building terminal UIs in Rust shouldn't require imperative rendering code. `limit-tui` brings a React-like Virtual DOM to the terminal, enabling declarative UI components with efficient diff-based updates.

## Features

- **Virtual DOM**: React-like component model with diff-based rendering
- **Pre-built components**: Chat views, inputs, spinners, progress bars, select menus
- **Flexbox layout**: CSS Flexbox-inspired layout system
- **Syntax highlighting**: Built-in support for 150+ languages via Syntect
- **Event handling**: Keyboard, mouse, and resize events
- **Zero-cost abstractions**: Efficient rendering with minimal allocations

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
limit-tui = "0.0.27"
```

**Requirements**: Rust 1.70+, Unix-like OS (Linux, macOS)

## Quick Start

### Basic Application

```rust,no_run
use limit_tui::backend::RatatuiBackend;
use limit_tui::components::{ChatView, InputPrompt};
use limit_tui::vdom::{VNode, Component};
use limit_tui::layout::{FlexDirection, AlignItems};
use crossterm::event::{KeyEvent, KeyCode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = RatatuiBackend::new()?;
    let app = App::new();
    limit_tui::run_event_loop(backend, app)
}

struct App {
    messages: Vec<Message>,
    input: String,
}

impl Component for App {
    fn render(&self) -> VNode {
        VNode::fragment(vec![
            ChatView::new(self.messages.clone()).into(),
            InputPrompt::new(&self.input, "Type a message...").into(),
        ])
    }
    
    fn on_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => self.input.push(c),
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    self.messages.push(Message {
                        role: Role::User,
                        content: self.input.clone(),
                    });
                    self.input.clear();
                }
            }
            KeyCode::Backspace => { self.input.pop(); }
            _ => {}
        }
    }
}
```

## Virtual DOM

Build UIs declaratively with `VNode`:

```rust
use limit_tui::vdom::VNode;
use ratatui::style::{Style, Color};

let ui = VNode::div()
    .style(Style::default().bg(Color::Blue))
    .children(vec![
        VNode::text("Hello, World!"),
        VNode::div()
            .flex(FlexDirection::Column)
            .align_items(AlignItems::Center)
            .gap(2)
            .children(vec![
                VNode::text("Line 1"),
                VNode::text("Line 2"),
            ]),
    ]);
```

### Diff-Based Updates

The Virtual DOM efficiently computes minimal changes:

```rust
use limit_tui::vdom::{diff, apply, VNode};

let old_tree = VNode::text("Hello");
let new_tree = VNode::text("Hello, World!");

let patches = diff(&old_tree, &new_tree);
// patches = [Patch::SetText(0, "Hello, World!")]

apply(&mut terminal, patches)?;
```

## Components

### ChatView

Display conversation messages with role-based styling:

```rust
use limit_tui::components::{ChatView, Message, Role};

let chat = ChatView::new(vec![
    Message { role: Role::User, content: "What is Rust?".into() },
    Message { role: Role::Assistant, content: "Rust is a systems programming...".into() },
    Message { role: Role::System, content: "Session started".into() },
]);
```

### InputPrompt

Interactive text input with placeholder:

```rust
use limit_tui::components::InputPrompt;

let input = InputPrompt::new(&buffer, "Enter command...")
    .prefix(">")
    .show_cursor(true);
```

### SelectPrompt

Single/multi-select menus:

```rust
use limit_tui::components::{SelectPrompt, SelectResult};

let menu = SelectPrompt::new(&["Option A", "Option B", "Option C"])
    .title("Choose an option");

match menu.result() {
    SelectResult::Selected(index) => println!("Selected: {}", index),
    SelectResult::Cancelled => println!("Cancelled"),
    SelectResult::None => {},
}
```

### Spinner & ProgressBar

Loading indicators:

```rust
use limit_tui::components::{Spinner, ProgressBar};

let spinner = Spinner::new().label("Loading...");
let progress = ProgressBar::new(75, 100).label("Downloading");
```

## Layout System

Flexbox-inspired layout:

```rust
use limit_tui::layout::{FlexboxLayout, FlexDirection, JustifyContent, AlignItems};

let layout = FlexboxLayout::new()
    .direction(FlexDirection::Row)
    .justify_content(JustifyContent::SpaceBetween)
    .align_items(AlignItems::Center)
    .gap(2);
```

### Layout Properties

| Property | Values | Description |
|----------|--------|-------------|
| `direction` | `Row`, `Column` | Main axis direction |
| `justify_content` | `Start`, `Center`, `End`, `SpaceBetween`, `SpaceAround` | Main axis alignment |
| `align_items` | `Start`, `Center`, `End`, `Stretch` | Cross axis alignment |
| `gap` | `u16` | Spacing between children |
| `wrap` | `bool` | Enable flex wrapping |

## Syntax Highlighting

```rust
use limit_tui::syntax::SyntaxHighlighter;

let highlighter = SyntaxHighlighter::new()?;

let code = highlighter.highlight(r#"
fn main() {
    println!("Hello, World!");
}
"#, "rust");

// Returns styled text with syntax colors
```

### Supported Languages

Built-in support for 150+ languages including:
- Rust, Go, Python, JavaScript, TypeScript
- Java, C, C++, C#
- HTML, CSS, SCSS
- JSON, YAML, TOML, XML
- Markdown, SQL, Shell

## API Reference

| Module | Description |
|--------|-------------|
| `vdom` | Virtual DOM types and diff algorithm |
| `components` | Pre-built UI components |
| `layout` | Flexbox layout system |
| `syntax` | Syntax highlighting |
| `backend` | Ratatui integration |

## Examples

```bash
# Run examples
cargo run --example basic
cargo run --example chat
cargo run --example flexbox
```

## License

MIT © [Mário Idival](https://github.com/marioidival)
