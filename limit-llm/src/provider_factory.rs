use crate::client::AnthropicClient;
use crate::config::Config;
use crate::error::LlmError;
use crate::openai_provider::OpenAiProvider;
use crate::providers::LlmProvider;
use crate::zai_provider::{ThinkingConfig, ZaiProvider};
use std::boxed::Box;

pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create_provider(config: &Config) -> Result<Box<dyn LlmProvider>, LlmError> {
        let provider_config = config.providers.get(&config.provider).ok_or_else(|| {
            LlmError::ConfigError(format!(
                "Provider '{}' not found in config",
                config.provider
            ))
        })?;

        let api_key = provider_config.api_key_or_env(&config.provider)
            .ok_or_else(|| LlmError::ConfigError(format!("No API key found for provider '{}'. Set api_key in config or {}_API_KEY env var",
                config.provider, config.provider.to_uppercase())))?;

        match config.provider.as_str() {
            "anthropic" => Ok(Box::new(AnthropicClient::new(
                api_key,
                provider_config.base_url.as_deref(),
                provider_config.timeout,
                &provider_config.model,
                provider_config.max_tokens,
            ))),
            "openai" => Ok(Box::new(OpenAiProvider::new(
                api_key,
                provider_config.base_url.as_deref(),
                &provider_config.model,
                provider_config.max_tokens,
                provider_config.timeout,
            ))),
            "zai" => {
                let thinking_config = ThinkingConfig {
                    thinking_enabled: provider_config.thinking_enabled,
                    clear_thinking: provider_config.clear_thinking,
                };
                Ok(Box::new(ZaiProvider::new(
                    api_key,
                    provider_config.base_url.as_deref(),
                    &provider_config.model,
                    provider_config.max_tokens,
                    provider_config.timeout,
                    thinking_config,
                )))
            }
            _ => Err(LlmError::ConfigError(format!(
                "Unknown provider: {}",
                config.provider
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zai_provider_factory() {
        let config_content = r#"
provider = "zai"

[providers.zai]
api_key = "test-zai-key"
model = "glm-4.7"
"#;
        let config: Config = toml::from_str(config_content).unwrap();
        let provider = ProviderFactory::create_provider(&config).unwrap();
        assert_eq!(provider.provider_name(), "zai");
        // Now uses ZaiProvider
    }
}
