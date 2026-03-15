//! Handler modules for browser tool
//!
//! Each module contains handlers for a specific category of browser actions.
//! These handlers extract arguments and call the appropriate client methods.

pub mod dialog;
pub mod interaction;
pub mod navigation;
pub mod query;
pub mod settings;
pub mod state;
pub mod storage;
pub mod tabs;
pub mod wait;
