//! # limit-tui
//!
//! [![Crates.io](https://img.shields.io/crates/v/limit-tui.svg)](https://crates.io/crates/limit-tui)
//! [![Docs.rs](https://docs.rs/limit-tui/badge.svg)](https://docs.rs/limit-tui)
//! [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
//!
//! **Terminal UI components with Virtual DOM rendering for Rust applications.**
//!
//! Build rich terminal interfaces with a component model, powered by
//! [Ratatui](https://ratatui.rs/). Features declarative rendering, diff-based
//! updates, and built-in syntax highlighting.
//!
//! ## Features
//!
//! - **Virtual DOM**: Component model with diff-based rendering
//! - **Pre-built components**: Chat views, inputs, spinners, progress bars, select menus
//! - **Syntax highlighting**: Built-in support for 150+ languages via Syntect
//! - **Event handling**: Keyboard, mouse, and resize events
//!
//! ## Virtual DOM
//!
//! Build UIs with `VNode`:
//!
//! ```rust
//! use limit_tui::vdom::VNode;
//! use std::collections::HashMap;
//!
//! let ui = VNode::Element {
//!     tag: "div".to_string(),
//!     attrs: HashMap::new(),
//!     children: vec![
//!         VNode::Text("Hello, World!".to_string()),
//!     ],
//! };
//! ```
//!
//! ### Diff-Based Updates
//!
//! The Virtual DOM computes minimal changes:
//!
//! ```rust
//! use limit_tui::vdom::{diff, apply, VNode};
//!
//! let old_tree = VNode::Text("Hello".to_string());
//! let new_tree = VNode::Text("Hello, World!".to_string());
//!
//! let patches = diff(&old_tree, &new_tree);
//! // patches contains minimal changes to transform old → new
//! ```
//!
//! ## Components
//!
//! | Component | Description |
//! |-----------|-------------|
//! | [`ChatView`] | Conversation display with role-based styling |
//! | [`InputPrompt`] | Interactive text input with placeholder |
//! | [`SelectPrompt`] | Single/multi-select menus |
//! | [`Spinner`] | Loading spinner with label |
//! | [`ProgressBar`] | Progress indicator |
//!
//! ## Syntax Highlighting
//!
//! ```rust,no_run
//! use limit_tui::SyntaxHighlighter;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let highlighter = SyntaxHighlighter::new()?;
//!
//! let code = highlighter.highlight_to_spans(r#"
//! fn main() {
//!     println!("Hello, World!");
//! }
//! "#, "rust")?;
//!
//! // Returns styled spans with syntax colors
//! # Ok(())
//! # }
//! ```

pub mod accessibility;
pub mod backend;
pub mod components;
pub mod layout;
pub mod syntax;
pub mod vdom;

pub use accessibility::{Accessible, LiveRegion};

pub use backend::{render_vdom_to_ratatui, run_event_loop, RatatuiBackend};
pub use components::{
    ChatView, InputPrompt, InputResult, Message, ProgressBar, Role, SelectPrompt, SelectResult,
    Spinner,
};
pub use layout::{AlignItems, FlexDirection, FlexStyle, FlexboxLayout, JustifyContent};
pub use syntax::SyntaxHighlighter;
pub use vdom::{apply, diff, render, Patch, VNode};
