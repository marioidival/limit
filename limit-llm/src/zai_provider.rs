use crate::error::LlmError;
use crate::openai_provider::OpenAiProvider;
use crate::providers::{LlmProvider, ProviderResponseChunk};
use crate::types::{Message, Tool};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

#[derive(Clone, Debug)]
pub struct ThinkingConfig {
    pub thinking_enabled: bool,
    pub clear_thinking: bool,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self {
            thinking_enabled: false,
            clear_thinking: true,
        }
    }
}

#[derive(Clone)]
pub struct ZaiProvider {
    openai: OpenAiProvider,
    #[allow(dead_code)]
    thinking_config: ThinkingConfig,
}
impl ZaiProvider {
    pub fn new(
        api_key: String,
        base_url: Option<&str>,
        model: &str,
        max_tokens: u32,
        timeout: u64,
        thinking_config: ThinkingConfig,
    ) -> Self {
        let default_url = "https://api.z.ai/api/coding/paas/v4/chat/completions";

        // Build extra_body with thinking config
        let extra_body = if thinking_config.thinking_enabled {
            let mut body = serde_json::Map::new();
            let thinking = serde_json::json!({
                "type": "enabled",
                "clear_thinking": thinking_config.clear_thinking
            });
            body.insert("thinking".to_string(), thinking);
            Some(body)
        } else {
            // Disabled thinking - explicitly disable to avoid default interleaved thinking
            let mut body = serde_json::Map::new();
            let thinking = serde_json::json!({
                "type": "disabled"
            });
            body.insert("thinking".to_string(), thinking);
            Some(body)
        };

        Self {
            openai: OpenAiProvider::with_extra_body(
                api_key,
                base_url.or(Some(default_url)),
                model,
                max_tokens,
                timeout,
                extra_body,
            ),
            thinking_config,
        }
    }
}

// ZAI uses numeric error codes like {"error":{"code":"1214","message":"..."}}
// Error parsing is handled by OpenAiProvider's do_request() method
// Full ZAI-specific error parsing would require modifying OpenAiProvider's error handling

#[async_trait]
impl LlmProvider for ZaiProvider {
    #[allow(clippy::type_complexity)]
    async fn send(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + '_>>,
        LlmError,
    > {
        // Delegate to OpenAiProvider which handles streaming
        // Note: reasoning_content parsing would require modifying OpenAiProvider's
        // SSE stream parsing - deferred to follow-up task
        self.openai.send(messages, tools).await
    }

    fn provider_name(&self) -> &str {
        "zai"
    }

    fn model_name(&self) -> &str {
        self.openai.model_name()
    }

    fn clone_box(&self) -> Box<dyn LlmProvider> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zai_provider_creation() {
        let provider = ZaiProvider::new(
            "test-key".to_string(),
            None,
            "glm-4.7",
            4096,
            60,
            ThinkingConfig::default(),
        );
        assert_eq!(provider.provider_name(), "zai");
        assert_eq!(provider.model_name(), "glm-4.7");
    }

    #[test]
    fn test_zai_provider_with_custom_url() {
        let custom_url = "https://custom.api.com/chat";
        let provider = ZaiProvider::new(
            "test-key".to_string(),
            Some(custom_url),
            "glm-5",
            8192,
            120,
            ThinkingConfig::default(),
        );
        assert_eq!(provider.provider_name(), "zai");
        assert_eq!(provider.model_name(), "glm-5");
    }

    #[test]
    fn test_thinking_config_default() {
        let config = ThinkingConfig::default();
        assert!(!config.thinking_enabled);
        assert!(config.clear_thinking);
    }

    #[test]
    fn test_zai_provider_clone() {
        let provider = ZaiProvider::new(
            "test-key".to_string(),
            None,
            "glm-4.7",
            4096,
            60,
            ThinkingConfig {
                thinking_enabled: true,
                clear_thinking: false,
            },
        );
        let cloned = provider.clone_box();
        assert_eq!(cloned.provider_name(), "zai");
        assert_eq!(cloned.model_name(), "glm-4.7");
    }
}
