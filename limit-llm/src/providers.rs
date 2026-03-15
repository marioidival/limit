//! Multi-provider LLM support.
//!
//! This module provides the [`LlmProvider`] trait that all LLM providers implement,
//! enabling a unified interface for working with different LLM backends.
//!
//! # Available Providers
//!
//! | Provider | Type | Features |
//! |----------|------|----------|
//! | Anthropic Claude | [`AnthropicClient`] | Streaming, Tools, Thinking |
//! | OpenAI GPT | [`OpenAiProvider`] | Streaming, Tools |
//! | z.ai GLM | [`ZaiProvider`] | Streaming, Tools, Thinking |
//! | Local/Ollama | [`LocalProvider`] | Streaming |
//!
//! # Using the Provider Trait
//!
//! ```rust,no_run
//! use limit_llm::{LlmProvider, Message, Tool};
//! use futures::StreamExt;
//!
//! async fn complete_with_provider(
//!     provider: Box<dyn LlmProvider>,
//!     messages: Vec<Message>,
//! ) -> Result<String, limit_llm::LlmError> {
//!     let mut stream = provider.send(messages, vec![]).await?;
//!     let mut result = String::new();
//!     
//!     while let Some(chunk) = stream.next().await {
//!         match chunk {
//!             Ok(limit_llm::ProviderResponseChunk::ContentDelta(text)) => {
//!                 result.push_str(&text);
//!             }
//!             Ok(limit_llm::ProviderResponseChunk::Done(usage)) => {
//!                 eprintln!("Tokens: {} in, {} out",
//!                     usage.input_tokens, usage.output_tokens);
//!             }
//!             Err(e) => return Err(e),
//!             _ => {}
//!         }
//!     }
//!     
//!     Ok(result)
//! }
//! ```
//!
//! # Response Chunks
//!
//! Providers return a stream of [`ProviderResponseChunk`] variants:
//!
//! - [`ContentDelta`](ProviderResponseChunk::ContentDelta) — Incremental text content
//! - [`ReasoningDelta`](ProviderResponseChunk::ReasoningDelta) — Thinking/reasoning content
//! - [`ToolCallDelta`](ProviderResponseChunk::ToolCallDelta) — Tool call with arguments
//! - [`Done`](ProviderResponseChunk::Done) — Completion with token usage

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::error::LlmError;
use crate::types::{Message, Tool, Usage};

/// A chunk of response data from an LLM provider.
///
/// Providers stream responses as a sequence of chunks, allowing for
/// real-time display of generated content.
#[derive(Debug, Clone)]
pub enum ProviderResponseChunk {
    /// Incremental text content from the assistant.
    ///
    /// Concatenate these deltas to build the complete response.
    ContentDelta(String),

    /// Incremental reasoning/thinking content.
    ///
    /// Some providers (Claude, z.ai) support extended thinking mode
    /// where the model shows its reasoning process.
    ReasoningDelta(String),

    /// A tool call with accumulated arguments.
    ///
    /// The `arguments` field contains the JSON arguments parsed so far.
    /// For streaming tool calls, arguments may be partially complete.
    ToolCallDelta {
        /// Unique identifier for this tool call.
        id: String,

        /// Name of the tool to call.
        name: String,

        /// JSON arguments (may be partial during streaming).
        arguments: serde_json::Value,
    },

    /// Response completion with token usage statistics.
    ///
    /// This is always the final chunk in the stream.
    Done(Usage),
}

/// Common trait for all LLM providers.
///
/// This trait provides a unified interface for sending messages to different
/// LLM backends. All providers support streaming responses and optional tool calling.
///
/// # Implementation
///
/// ```rust,ignore
/// use limit_llm::{LlmProvider, Message, Tool, ProviderResponseChunk, LlmError};
/// use async_trait::async_trait;
/// use futures::Stream;
/// use std::pin::Pin;
///
/// struct MyProvider {
///     // provider-specific fields
/// }
///
/// #[async_trait]
/// impl LlmProvider for MyProvider {
///     async fn send(
///         &self,
///         messages: Vec<Message>,
///         tools: Vec<Tool>,
///     ) -> Result<
///         Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + '_>>,
///         LlmError,
///     > {
///         // Implementation that streams response chunks
///     }
///     
///     fn provider_name(&self) -> &str { "my_provider" }
///     fn model_name(&self) -> &str { "my-model" }
///     fn clone_box(&self) -> Box<dyn LlmProvider> {
///         Box::new(self.clone())
///     }
/// }
/// ```
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send messages to the LLM and receive a streaming response.
    ///
    /// # Arguments
    ///
    /// * `messages` — The conversation history as a vector of messages
    /// * `tools` — Optional tool definitions for function calling
    ///
    /// # Returns
    ///
    /// A stream of response chunks. The stream will always end with a
    /// [`Done`](ProviderResponseChunk::Done) chunk on success.
    ///
    /// # Errors
    ///
    /// Returns [`LlmError`] if the request fails before streaming begins.
    #[allow(clippy::type_complexity)]
    async fn send(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + '_>>,
        LlmError,
    >;

    /// Returns the provider name (e.g., "anthropic", "openai").
    fn provider_name(&self) -> &str;

    /// Returns the model name (e.g., "claude-3-5-sonnet-20241022").
    fn model_name(&self) -> &str;

    /// Clones the provider into a boxed trait object.
    ///
    /// This enables cloning of `Box<dyn LlmProvider>`.
    fn clone_box(&self) -> Box<dyn LlmProvider>;
}

/// Implement Clone for `Box<dyn LlmProvider>`.
impl Clone for Box<dyn LlmProvider> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Provider configuration variants for TOML deserialization.
///
/// This enum is used in the main [`Config`](crate::Config) to specify
/// which provider to use and its settings.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum ProviderConfig {
    /// Anthropic Claude configuration.
    Anthropic(AnthropicConfig),

    /// OpenAI GPT configuration.
    OpenAI(OpenAIConfig),

    /// Unknown/unsupported provider.
    #[serde(other)]
    Unknown,
}

/// Anthropic-specific configuration.
///
/// # Example TOML
///
/// ```toml
/// provider = "anthropic"
///
/// [providers.anthropic]
/// api_key = "sk-ant-api03-..."
/// model = "claude-sonnet-4-6-20260217"
/// max_tokens = 4096
/// timeout = 60
/// ```
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AnthropicConfig {
    /// API key (falls back to `ANTHROPIC_API_KEY` env var).
    pub api_key: Option<String>,

    /// Model to use (default: "claude-3-5-sonnet-20241022").
    #[serde(default = "default_anthropic_model")]
    pub model: String,

    /// Maximum tokens in the response (default: 4096).
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Request timeout in seconds (default: 60).
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Custom API endpoint (optional).
    #[serde(default)]
    pub base_url: Option<String>,
}

/// OpenAI-specific configuration.
///
/// # Example TOML
///
/// ```toml
/// provider = "openai"
///
/// [providers.openai]
/// api_key = "sk-..."
/// model = "gpt-5.4"
/// max_tokens = 4096
/// timeout = 60
/// base_url = "http://localhost:8080/v1/chat/completions"
/// ```
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OpenAIConfig {
    /// API key (falls back to `OPENAI_API_KEY` env var).
    pub api_key: Option<String>,

    /// Model to use (default: "gpt-4").
    #[serde(default = "default_openai_model")]
    pub model: String,

    /// Maximum tokens in the response (default: 4096).
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Request timeout in seconds (default: 60).
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Custom API endpoint (optional).
    /// Note: Must include full path (e.g., `/v1/chat/completions`).
    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_anthropic_model() -> String {
    "claude-3-5-sonnet-20241022".to_string()
}

fn default_openai_model() -> String {
    "gpt-4".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_timeout() -> u64 {
    60
}
