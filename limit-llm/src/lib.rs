//! # limit-llm
//!
//! [![Crates.io](https://img.shields.io/crates/v/limit-llm.svg)](https://crates.io/crates/limit-llm)
//! [![Docs.rs](https://docs.rs/limit-llm/badge.svg)](https://docs.rs/limit-llm)
//! [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
//!
//! **Multi-provider LLM client for Rust with streaming support.**
//!
//! Unified API for Anthropic Claude, OpenAI, z.ai, and local LLMs with built-in
//! token tracking, state persistence, and automatic model handoff.
//!
//! ## Features
//!
//! - **Multi-provider support**: Anthropic Claude, OpenAI GPT, z.ai GLM, and local LLMs
//! - **Streaming responses**: Async streaming with `futures::Stream`
//! - **Token tracking**: SQLite-based usage tracking and cost estimation
//! - **State persistence**: Serialize/restore conversation state with bincode
//! - **Model handoff**: Automatic fallback between providers on failure
//! - **Tool calling**: Full function/tool support for all compatible providers
//! - **Thinking mode**: Extended reasoning support (Claude, z.ai)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use limit_llm::{AnthropicClient, Message, Role, LlmProvider};
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client from environment variable ANTHROPIC_API_KEY
//!     let client = AnthropicClient::new(
//!         std::env::var("ANTHROPIC_API_KEY")?,
//!         None,  // default base URL
//!         60,    // timeout in seconds
//!         "claude-sonnet-4-6-20260217",
//!         4096,  // max tokens
//!     );
//!
//!     let messages = vec![
//!         Message {
//!             role: Role::User,
//!             content: Some("Hello, Claude!".to_string()),
//!             tool_calls: None,
//!             tool_call_id: None,
//!         }
//!     ];
//!
//!     // Stream the response
//!     let mut stream = client.send(messages, vec![]).await;
//!     
//!     while let Some(chunk) = stream.next().await {
//!         match chunk {
//!             Ok(limit_llm::ProviderResponseChunk::ContentDelta(text)) => {
//!                 print!("{}", text);
//!             }
//!             Ok(limit_llm::ProviderResponseChunk::Done(usage)) => {
//!                 println!("\n\nTokens: {} in, {} out",
//!                     usage.input_tokens, usage.output_tokens);
//!             }
//!             Err(e) => eprintln!("Error: {}", e),
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Providers
//!
//! | Provider | Client | Streaming | Tools | Thinking |
//! |----------|--------|-----------|-------|----------|
//! | Anthropic Claude | [`AnthropicClient`] | ✓ | ✓ | ✓ |
//! | OpenAI | [`OpenAiProvider`] | ✓ | ✓ | — |
//! | z.ai GLM | [`ZaiProvider`] | ✓ | ✓ | ✓ |
//! | Local/Ollama | [`LocalProvider`] | ✓ | — | — |
//!
//! ## Configuration
//!
//! ### Environment Variables
//!
//! ```bash
//! ANTHROPIC_API_KEY=your-key      # For Claude
//! OPENAI_API_KEY=your-key          # For GPT
//! ZAI_API_KEY=your-key             # For z.ai
//! ```
//!
//! ### Programmatic Configuration
//!
//! ```rust,no_run
//! use limit_llm::{Config, ProviderFactory};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load from ~/.limit/config.toml
//! let config = Config::load()?;
//!
//! // Create provider from config
//! let provider = ProviderFactory::create_provider(&config)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Token Tracking
//!
//! ```rust,no_run
//! use limit_llm::TrackingDb;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let tracking = TrackingDb::new()?;
//!
//! // Track a request
//! tracking.track_request(
//!     "claude-sonnet-4-6-20260217",
//!     100,  // input tokens
//!     50,   // output tokens
//!     0.001, // cost in USD
//!     1500,  // duration in ms
//! )?;
//!
//! // Get statistics for last 7 days
//! let stats = tracking.get_usage_stats(7)?;
//! println!("Total cost: ${:.4}", stats.total_cost);
//! # Ok(())
//! # }
//! ```
//!
//! ## State Persistence
//!
//! ```rust,no_run
//! use limit_llm::{StatePersistence, Message};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let persistence = StatePersistence::new("~/.limit/state/session.bin");
//!
//! // Save conversation
//! let messages: Vec<Message> = vec![];
//! persistence.save(&messages)?;
//!
//! // Restore later
//! let restored = persistence.load()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Model Handoff
//!
//! The `ModelHandoff` type provides token counting and message compaction
//! for transitioning between models with different context windows:
//!
//! ```rust,no_run
//! use limit_llm::ModelHandoff;
//!
//! # fn main() {
//! let handoff = ModelHandoff::new();
//!
//! // Count tokens in a message
//! let tokens = handoff.count_tokens("Hello, world!");
//! println!("Token count: {}", tokens);
//!
//! // Compact messages to fit a target context window
//! // let compacted = handoff.handoff_to_model("claude-3-5-sonnet", "claude-3-5-haiku", &messages);
//! # }
//! ```

pub mod cache;
pub mod client;
pub mod config;
pub mod error;
pub mod handoff;
pub mod local_provider;
pub mod openai_provider;
pub mod persistence;
pub mod provider_factory;
pub mod providers;
pub mod tracking;
pub mod types;
pub mod zai_provider;

pub use cache::apply_cache_control;
pub use client::AnthropicClient;
pub use config::{BrowserConfigSection, CacheSettings, CompactionSettings, Config, ProviderConfig};
pub use error::LlmError;
pub use handoff::ModelHandoff;
pub use local_provider::LocalProvider;
pub use openai_provider::OpenAiProvider;
pub use persistence::StatePersistence;
pub use provider_factory::ProviderFactory;
pub use providers::{LlmProvider, ProviderResponseChunk};
pub use tracking::TrackingDb;
pub use types::{
    CacheControl, FunctionCall, Message, Response, Role, Tool, ToolCall, ToolFunction, Usage,
};
pub use zai_provider::{ThinkingConfig, ZaiProvider};
