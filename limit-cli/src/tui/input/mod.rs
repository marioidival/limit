//! Input handling module for TUI
//!
//! This module provides keyboard, mouse, and clipboard input handling.

mod clipboard;
mod handler;

pub use clipboard::ClipboardHandler;
pub use handler::{InputAction, InputHandler};
