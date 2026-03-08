// limit-llm: Multi-Provider Support

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::error::LlmError;
use crate::types::{Message, Tool, Usage};

/// Response chunk from streaming LLM providers
pub enum ProviderResponseChunk {
    ContentDelta(String),
    ReasoningDelta(String),
    ToolCallDelta {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    Done(Usage),
}

/// Common trait for all LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send messages to the LLM and receive streaming response
    #[allow(clippy::type_complexity)]
    async fn send(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + '_>>,
        LlmError,
    >;

    /// Get provider name
    fn provider_name(&self) -> &str;

    /// Get model name
    fn model_name(&self) -> &str;

    /// Clone the provider
    fn clone_box(&self) -> Box<dyn LlmProvider>;
}

/// Implement Clone for Box<dyn LlmProvider>
impl Clone for Box<dyn LlmProvider> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Provider configuration variants
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum ProviderConfig {
    Anthropic(AnthropicConfig),
    OpenAI(OpenAIConfig),
    #[serde(other)]
    Unknown,
}

/// Anthropic-specific configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AnthropicConfig {
    pub api_key: Option<String>,
    #[serde(default = "default_anthropic_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub base_url: Option<String>,
}

/// OpenAI-specific configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OpenAIConfig {
    pub api_key: Option<String>,
    #[serde(default = "default_openai_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
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
