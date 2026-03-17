//! # limit-agent
//!
//! [![Crates.io](https://img.shields.io/crates/v/limit-agent.svg)](https://crates.io/crates/limit-agent)
//! [![Docs.rs](https://docs.rs/limit-agent/badge.svg)](https://docs.rs/limit-agent)
//! [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
//!
//! **Agent runtime for AI applications with tool registry and Docker sandbox.**
//!
//! Build autonomous AI agents that can execute tools, run code in isolated
//! containers, and maintain state across conversations.
//!
//! ## Features
//!
//! - **Tool Registry**: Define, register, and execute tools dynamically
//! - **Docker Sandbox**: Isolated execution environment for untrusted code
//! - **Event-driven**: Subscribe to agent lifecycle events
//! - **State Management**: Persist and restore agent state
//! - **LLM Integration**: Works seamlessly with `limit-llm`
//!
//! ## Quick Start
//!
//! ### Define a Custom Tool
//!
//! ```rust
//! use async_trait::async_trait;
//! use limit_agent::{Tool, AgentError};
//! use serde_json::{json, Value};
//!
//! struct WeatherTool;
//!
//! #[async_trait]
//! impl Tool for WeatherTool {
//!     fn name(&self) -> &str {
//!         "get_weather"
//!     }
//!     
//!     async fn execute(&self, args: Value) -> Result<Value, AgentError> {
//!         let location = args["location"].as_str().unwrap_or("Unknown");
//!         // In a real implementation, call a weather API
//!         Ok(json!({
//!             "location": location,
//!             "temp": 22,
//!             "condition": "sunny"
//!         }))
//!     }
//! }
//! ```
//!
//! ### Register and Execute Tools
//!
//! ```rust
//! use limit_agent::ToolRegistry;
//! # use async_trait::async_trait;
//! # use limit_agent::{Tool, AgentError};
//! # use serde_json::{json, Value};
//! # struct WeatherTool;
//! # #[async_trait]
//! # impl Tool for WeatherTool {
//! #     fn name(&self) -> &str { "get_weather" }
//! #     async fn execute(&self, args: Value) -> Result<Value, AgentError> {
//! #         Ok(json!({"temp": 22}))
//! #     }
//! # }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut registry = ToolRegistry::new();
//!     
//!     // Register tools
//!     registry.register(WeatherTool)?;
//!     
//!     // Execute a tool by name
//!     let result = registry
//!         .execute("get_weather", json!({ "location": "Tokyo" }))
//!         .await?;
//!     
//!     println!("Weather: {:?}", result);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Docker Sandbox
//!
//! Run untrusted code in isolated Docker containers:
//!
//! ```rust,no_run
//! use limit_agent::sandbox::DockerSandbox;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Check if Docker is available
//!     if !DockerSandbox::check_docker_available().await {
//!         return Err("Docker is not available".into());
//!     }
//!     
//!     let sandbox = DockerSandbox::new().await?;
//!     
//!     // Create a container
//!     let container = sandbox.create_container("alpine:latest").await?;
//!     
//!     // Start and execute
//!     sandbox.start_container(&container).await?;
//!     let output = sandbox.execute_in_container(&container, &["echo".into(), "hello".into()]).await?;
//!     println!("{}", output);
//!     
//!     // Cleanup
//!     sandbox.cleanup_container(&container).await;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Event System
//!
//! Subscribe to agent lifecycle events for logging, monitoring, or debugging:
//!
//! ```rust
//! use limit_agent::events::{EventBus, Event};
//!
//! let events = EventBus::new();
//!
//! events.subscribe(|event| {
//!     match event {
//!         Event::ToolCall { name, args, version: _ } => {
//!             println!("Tool {} started with {:?}", name, args);
//!         }
//!         Event::ToolResult { output, version: _ } => {
//!             println!("Tool completed: {}", output);
//!         }
//!         Event::Error { message, version: _ } => {
//!             eprintln!("Error: {}", message);
//!         }
//!         _ => {}
//!     }
//! });
//! ```
//!
//! ## Core Types
//!
//! | Type | Description |
//! |------|-------------|
//! | [`Tool`] | Trait for defining executable tools |
//! | [`ToolRegistry`] | Registry for managing and executing tools |
//! | [`DockerSandbox`] | Isolated execution environment |
//! | [`StateManager`] | Persist/restore agent state |
//! | [`EventBus`] | Event subscription system |

pub mod error;
pub mod events;
pub mod executor;
pub mod registry;
pub mod sandbox;
pub mod state;
pub mod team;
pub mod tool;

pub use error::AgentError;
pub use events::EventBus;
pub use registry::ToolRegistry;
pub use team::{
    EventLevel, Role, RoleConfig, TaskProgressInfo, TaskProgressStatus, Team, TeamAgent,
    TeamConfig, TeamEvent, TeamHistory, TeamProgressEvent, TeamResult, TeamSection, TeamSnapshot,
    TeamStore, WorkflowPhase, PHASE_COUNT,
};
pub use tool::{EchoTool, Tool};
