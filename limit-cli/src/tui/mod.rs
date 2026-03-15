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
//! use limit_cli::agent_bridge::AgentBridge;
//! use limit_llm::{Config, ProviderConfig};
//! use tokio::sync::mpsc;
//! use std::collections::HashMap;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create config
//! let mut providers = HashMap::new();
//! providers.insert("anthropic".to_string(), ProviderConfig {
//!     api_key: Some("test-key".to_string()),
//!     model: "claude-3-5-sonnet-20241022".to_string(),
//!     base_url: None,
//!     max_tokens: 4096,
//!     timeout: 60,
//!     max_iterations: 100,
//!     thinking_enabled: false,
//!     clear_thinking: true,
//! });
//! let config = Config { provider: "anthropic".to_string(), providers, browser: limit_llm::BrowserConfigSection::default() };
//!
//! // Create agent bridge and event channel
//! let (tx, rx) = mpsc::unbounded_channel();
//! let bridge = AgentBridge::new(config)?;
//!
//! // Create TUI bridge
//! let tui_bridge = TuiBridge::new(bridge, rx)?;
//!
//! // Run TUI app
//! let mut app = TuiApp::new(tui_bridge)?;
//! app.run()?;
//! # Ok(())
//! # }
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
