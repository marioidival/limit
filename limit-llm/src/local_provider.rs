use crate::error::LlmError;
use crate::openai_provider::OpenAiProvider;
use crate::providers::{LlmProvider, ProviderResponseChunk};
use crate::types::{Message, Tool};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Local LLM provider for Ollama, LM Studio, vLLM, and other OpenAI-compatible local servers.
///
/// This provider wraps OpenAiProvider with sensible defaults for local inference:
/// - No API key required (empty string by default)
/// - Default localhost URL (Ollama: 11434)
/// - Extended timeout for slower local inference
#[derive(Clone)]
pub struct LocalProvider {
    openai: OpenAiProvider,
}

impl LocalProvider {
    /// Default Ollama endpoint
    pub const DEFAULT_OLLAMA_URL: &'static str = "http://localhost:11434/v1/chat/completions";

    /// Default LM Studio endpoint
    pub const DEFAULT_LMSTUDIO_URL: &'static str = "http://localhost:1234/v1/chat/completions";

    /// Default vLLM endpoint
    pub const DEFAULT_VLLM_URL: &'static str = "http://localhost:8000/v1/chat/completions";

    /// Create a new local provider
    ///
    /// # Arguments
    /// * `base_url` - Optional custom endpoint. Defaults to Ollama.
    /// * `model` - Model name to use (required)
    /// * `max_tokens` - Maximum output tokens
    /// * `timeout` - Request timeout in seconds (default: 120 for slower local inference)
    pub fn new(base_url: Option<&str>, model: &str, max_tokens: u32, timeout: u64) -> Self {
        let url = base_url.unwrap_or(Self::DEFAULT_OLLAMA_URL);
        // Local servers typically don't require auth, use placeholder
        let api_key = "local".to_string();

        Self {
            openai: OpenAiProvider::new(api_key, Some(url), model, max_tokens, timeout),
        }
    }

    /// Create provider configured for Ollama
    pub fn ollama(model: &str, max_tokens: u32, timeout: u64) -> Self {
        Self::new(Some(Self::DEFAULT_OLLAMA_URL), model, max_tokens, timeout)
    }

    /// Create provider configured for LM Studio
    pub fn lmstudio(model: &str, max_tokens: u32, timeout: u64) -> Self {
        Self::new(Some(Self::DEFAULT_LMSTUDIO_URL), model, max_tokens, timeout)
    }

    /// Create provider configured for vLLM
    pub fn vllm(model: &str, max_tokens: u32, timeout: u64) -> Self {
        Self::new(Some(Self::DEFAULT_VLLM_URL), model, max_tokens, timeout)
    }
}

#[async_trait]
impl LlmProvider for LocalProvider {
    #[allow(clippy::type_complexity)]
    async fn send(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + '_>>,
        LlmError,
    > {
        self.openai.send(messages, tools).await
    }

    fn provider_name(&self) -> &str {
        "local"
    }

    fn model_name(&self) -> &str {
        self.openai.model_name()
    }

    fn clone_box(&self) -> Box<dyn LlmProvider> {
        Box::new(self.clone())
    }

    fn with_max_tokens(&self, _max_tokens: u32) -> Box<dyn LlmProvider> {
        // Local delegates to OpenAiProvider which has private fields.
        self.clone_box()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_provider_creation() {
        let provider = LocalProvider::new(None, "llama3.2", 4096, 120);
        assert_eq!(provider.provider_name(), "local");
        assert_eq!(provider.model_name(), "llama3.2");
    }

    #[test]
    fn test_local_provider_custom_url() {
        let provider = LocalProvider::new(
            Some("http://custom:8080/v1/chat/completions"),
            "custom-model",
            8192,
            60,
        );
        assert_eq!(provider.provider_name(), "local");
        assert_eq!(provider.model_name(), "custom-model");
    }

    #[test]
    fn test_ollama_preset() {
        let provider = LocalProvider::ollama("llama3.2", 4096, 120);
        assert_eq!(provider.provider_name(), "local");
        assert_eq!(provider.model_name(), "llama3.2");
    }

    #[test]
    fn test_lmstudio_preset() {
        let provider = LocalProvider::lmstudio("local-model", 4096, 120);
        assert_eq!(provider.provider_name(), "local");
        assert_eq!(provider.model_name(), "local-model");
    }

    #[test]
    fn test_vllm_preset() {
        let provider = LocalProvider::vllm("meta-llama/Llama-3.2-3B", 4096, 120);
        assert_eq!(provider.provider_name(), "local");
        assert_eq!(provider.model_name(), "meta-llama/Llama-3.2-3B");
    }

    #[test]
    fn test_local_provider_clone() {
        let provider = LocalProvider::new(None, "test-model", 4096, 120);
        let cloned = provider.clone_box();
        assert_eq!(cloned.provider_name(), "local");
        assert_eq!(cloned.model_name(), "test-model");
    }
}
