//! TUI (Terminal User Interface) module for limit-cli
//!
//! This module provides a rich terminal interface for interacting with the Limit AI agent.
//!
//! # Architecture
//!
//! The TUI system is organized into several components:
//!
//! - **State**: Core state types (`TuiState`, `FileAutocompleteState`)
//! - **Bridge**: Connection between agent and UI (`TuiBridge`)
//! - **App**: Main application loop (`TuiApp`)
//! - **Input**: Input handling (`InputHandler`, `InputEditor`, `ClipboardHandler`)
//! - **Activity**: Activity message formatting
//! - **Autocomplete**: File autocomplete management
//!
//! # Example
//!
//! ```no_run
//! use limit_cli::tui::app::{TuiBridge, TuiApp};
//! use limit_cli::agent_bridge::{AgentBridge, AgentEvent};
//! use tokio::sync::mpsc;
//!
//! // Create agent bridge and event channel
//! let (tx, rx) = mpsc::unbounded_channel();
//! let bridge = AgentBridge::new(config)?;
//!
//! // Create TUI bridge
//! let tui_bridge = TuiBridge::new(bridge, rx)?;
//!
//! // Run TUI app
//! let app = TuiApp::new(tui_bridge)?;
//! app.run()?;
//! ```

mod state;

pub mod activity;
pub mod app;
pub mod autocomplete;
pub mod bridge;
pub mod commands;
pub mod input;
pub mod ui;

// Re-export public API
pub use input::InputHandler;
pub use state::{FileAutocompleteState, TuiState, MAX_PASTE_SIZE};
