//! Tool trait and built-in tools for agent execution.
//!
//! This module provides the core [`Tool`] trait for defining executable tools
//! that can be registered with a [`ToolRegistry`](crate::ToolRegistry) and called by AI agents.
//!
//! # Defining Custom Tools
//!
//! Implement the [`Tool`] trait to create custom tools:
//!
//! ```
//! use async_trait::async_trait;
//! use limit_agent::{Tool, AgentError};
//! use serde_json::{json, Value};
//!
//! struct CalculatorTool;
//!
//! #[async_trait]
//! impl Tool for CalculatorTool {
//!     fn name(&self) -> &str {
//!         "calculate"
//!     }
//!     
//!     async fn execute(&self, args: Value) -> Result<Value, AgentError> {
//!         let a = args["a"].as_f64().unwrap_or(0.0);
//!         let b = args["b"].as_f64().unwrap_or(0.0);
//!         let op = args["op"].as_str().unwrap_or("+");
//!         
//!         let result = match op {
//!             "+" => a + b,
//!             "-" => a - b,
//!             "*" => a * b,
//!             "/" => a / b,
//!             _ => return Err(AgentError::ToolError("Unknown operator".into())),
//!         };
//!         
//!         Ok(json!({ "result": result }))
//!     }
//! }
//! ```
//!
//! # Built-in Tools
//!
//! - [`EchoTool`] — Simple echo tool for testing, returns its input unchanged

use crate::error::AgentError;
use async_trait::async_trait;
use serde_json::Value;

/// Trait for defining executable tools.
///
/// Tools are the primary way for AI agents to interact with the world.
/// Each tool has a name and an async execute method that takes JSON
/// arguments and returns a JSON result.
///
/// # Thread Safety
///
/// All tools must be `Send + Sync` because they may be executed concurrently
/// across multiple threads.
pub trait Tool: Send + Sync {
    /// Returns the unique name of this tool.
    ///
    /// Tool names should be descriptive and follow a consistent naming
    /// convention (e.g., `snake_case`).
    fn name(&self) -> &str;

    /// Executes the tool with the given arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - JSON value containing the tool parameters. The structure
    ///   depends on the tool's parameter schema.
    ///
    /// # Returns
    ///
    /// A JSON value containing the tool's output, or an error if execution failed.
    ///
    /// # Errors
    ///
    /// Returns [`AgentError::ToolError`] if the tool execution fails.
    fn execute<'life0, 'async_trait>(
        &'life0 self,
        args: Value,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<Value, AgentError>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait;
}

/// A simple echo tool that returns its input unchanged.
///
/// Primarily useful for testing the tool registration and execution pipeline.
pub struct EchoTool;

impl EchoTool {
    /// Creates a new EchoTool instance.
    pub fn new() -> Self {
        EchoTool
    }
}

impl Default for EchoTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        // Echo back the input arguments unchanged
        Ok(args)
    }
}
