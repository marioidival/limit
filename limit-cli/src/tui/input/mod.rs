//! Input handling module for TUI
//!
//! This module provides keyboard, mouse, and clipboard input handling.

mod clipboard;
mod editor;
mod handler;

pub use clipboard::ClipboardHandler;
pub use editor::InputEditor;
pub use handler::{InputAction, InputHandler};
