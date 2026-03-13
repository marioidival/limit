//! TUI Application module
//!
//! Contains the main TUI application loop (`TuiApp`) and bridge (`TuiBridge`)

mod app_impl;

pub use crate::tui::bridge::TuiBridge;
pub use app_impl::TuiApp;
