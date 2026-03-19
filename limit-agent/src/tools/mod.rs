//! Built-in tools for limit-agent.
//!
//! This module provides ready-to-use tools that can be registered with
//! a [`ToolRegistry`](crate::ToolRegistry).

pub mod tldr;

pub use tldr::{tldr_tool_definition, TldrTool};
