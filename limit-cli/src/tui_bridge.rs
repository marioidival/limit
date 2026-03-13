//! Legacy TUI module - Re-exports from modular structure
//!
//! This module exists for backwards compatibility.
//! New code should use `limit_cli::tui::app::TuiApp` and `limit_cli::tui::bridge::TuiBridge` directly.
//!
//! # Deprecation
//!
//! This module is deprecated and will be removed in a future version.
//! Use the `tui` module instead.

// Re-export from the modular structure for backwards compatibility
#[deprecated(
    since = "0.0.25",
    note = "Use `limit_cli::tui::app::TuiApp` or `limit_cli::tui::bridge::TuiBridge` instead"
)]
pub use crate::tui::app::TuiApp;
#[deprecated(
    since = "0.0.25",
    note = "Use `limit_cli::tui::bridge::TuiBridge` instead"
)]
pub use crate::tui::bridge::TuiBridge;
