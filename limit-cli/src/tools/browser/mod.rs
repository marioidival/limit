//! Browser automation tools
//!
//! This module provides browser automation capabilities through the agent-browser CLI.
//!
//! # Architecture
//!
//! - [`config`] - Configuration types for browser settings
//! - [`executor`] - Abstraction layer for executing browser commands
//! - [`types`] - Data types for browser operations
//! - [`client`] - High-level API for browser operations
//! - [`tool`] - LLM agent tool implementation
//!
//! # Usage
//!
//! ## As an LLM Tool
//!
//! ```ignore
//! use limit_cli::tools::browser::BrowserTool;
//! use limit_agent::Tool;
//!
//! let tool = BrowserTool::new();
//! let result = tool.execute(serde_json::json!({
//!     "action": "open",
//!     "url": "https://example.com"
//! })).await;
//! ```
//!
//! ## As a TUI Command
//!
//! ```ignore
//! /browser open https://example.com
//! /browser snapshot
//! /browser click "button.submit"
//! /browser close
//! ```

pub mod action;
pub mod args;
pub mod client;
pub mod client_ext;
pub mod config;
pub mod executor;
pub mod response;
pub mod tool;
pub mod types;

pub use action::BrowserAction;
pub use args::ArgsExt;
pub use client::BrowserClient;
pub use config::{BrowserConfig, BrowserEngine};
pub use executor::{BrowserError, BrowserExecutor, BrowserOutput, CliExecutor};
pub use response::{ok, ok_msg, Response};
pub use tool::BrowserTool;
pub use types::{BoundingBox, Cookie, Request, SnapshotResult, TabInfo};
