//! Built-in tools for limit-agent.
//!
//! This module provides ready-to-use tools that can be registered with
//! a [`ToolRegistry`](crate::ToolRegistry).

pub mod tldr;
pub mod warm_guard;

pub use tldr::{tldr_tool_definition, TldrTool};
pub use warm_guard::WarmGuard;
