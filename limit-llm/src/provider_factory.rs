use crate::client::AnthropicClient;
use crate::config::{Config, ProviderConfig};
use crate::error::LlmError;
use crate::local_provider::LocalProvider;
use crate::openai_provider::OpenAiProvider;
use crate::providers::LlmProvider;
use crate::zai_provider::{ThinkingConfig, ZaiProvider};
use std::boxed::Box;
use std::env;

const DEFAULT_TIMEOUT: u64 = 60;

pub struct ProviderFactory;

/// Resolve API key from environment variables only (used when no config section exists).
fn resolve_api_key_from_env(provider_type: &str) -> Option<String> {
    match provider_type {
        "anthropic" => env::var("ANTHROPIC_API_KEY").ok(),
        "openai" => env::var("OPENAI_API_KEY")
            .ok()
            .or_else(|| env::var("ZAI_API_KEY").ok()),
        "zai" => env::var("ZAI_API_KEY").ok(),
        "local" | "ollama" | "lmstudio" | "vllm" => Some("local".to_string()),
        _ => None,
    }
}

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
            // Local LLM providers (all use LocalProvider with different defaults)
            "local" | "ollama" | "lmstudio" | "vllm" => {
                // Local providers don't require API key, use placeholder if empty
                let _ = api_key; // Suppress unused warning
                Ok(Box::new(LocalProvider::new(
                    provider_config.base_url.as_deref(),
                    &provider_config.model,
                    provider_config.max_tokens,
                    provider_config.timeout,
                )))
            }
            _ => Err(LlmError::ConfigError(format!(
                "Unknown provider: {}",
                config.provider
            ))),
        }
    }

    /// Create a provider for a specific team role.
    ///
    /// When `provider_config` is `Some`, inherits `api_key` and `base_url`
    /// from the main config's `[providers.<type>]` section (falling back to
    /// env vars for the api key). This lets roles reuse the main provider's
    /// credentials without repeating them.
    ///
    /// `base_url_override` and `max_tokens` from the role config take
    /// precedence over inherited values.
    pub fn create_for_role(
        provider_type: &str,
        model: &str,
        base_url_override: Option<&str>,
        max_tokens: u32,
        provider_config: Option<&ProviderConfig>,
    ) -> Result<Box<dyn LlmProvider>, LlmError> {
        // Resolve api_key: try config section first, then env vars.
        let api_key = provider_config
            .and_then(|pc| pc.api_key_or_env(provider_type))
            .or_else(|| resolve_api_key_from_env(provider_type))
            .ok_or_else(|| {
                LlmError::ConfigError(format!(
                    "No API key for role provider '{}'. Set api_key in [providers.{}] or {}_API_KEY env var",
                    provider_type, provider_type, provider_type.to_uppercase()
                ))
            })?;

        // Inherit base_url from config if role doesn't override it.
        let base_url: Option<String> = match base_url_override {
            Some(url) => Some(url.to_string()),
            None => provider_config.and_then(|pc| pc.base_url.clone()),
        };

        // Inherit timeout from config, default to 60s.
        let timeout = provider_config
            .map(|pc| pc.timeout)
            .unwrap_or(DEFAULT_TIMEOUT);

        match provider_type {
            "anthropic" => Ok(Box::new(AnthropicClient::new(
                api_key,
                base_url.as_deref(),
                timeout,
                model,
                max_tokens,
            ))),
            "openai" => Ok(Box::new(OpenAiProvider::new(
                api_key,
                base_url.as_deref(),
                model,
                max_tokens,
                timeout,
            ))),
            "zai" => {
                let thinking_config = provider_config
                    .map(|pc| ThinkingConfig {
                        thinking_enabled: pc.thinking_enabled,
                        clear_thinking: pc.clear_thinking,
                    })
                    .unwrap_or_default();
                Ok(Box::new(ZaiProvider::new(
                    api_key,
                    base_url.as_deref(),
                    model,
                    max_tokens,
                    timeout,
                    thinking_config,
                )))
            }
            "local" | "ollama" | "lmstudio" | "vllm" => Ok(Box::new(LocalProvider::new(
                base_url.as_deref(),
                model,
                max_tokens,
                timeout,
            ))),
            _ => Err(LlmError::ConfigError(format!(
                "Unknown provider for role: {}",
                provider_type
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
    }

    #[test]
    fn test_create_for_role_openai() {
        std::env::set_var("OPENAI_API_KEY", "test-key");
        let provider =
            ProviderFactory::create_for_role("openai", "gpt-4o-mini", None, 8192, None).unwrap();
        assert_eq!(provider.provider_name(), "openai");
        std::env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn test_create_for_role_local() {
        let provider = ProviderFactory::create_for_role(
            "ollama",
            "llama3",
            Some("http://localhost:11434"),
            4096,
            None,
        )
        .unwrap();
        assert_eq!(provider.provider_name(), "local");
    }

    #[test]
    fn test_create_for_role_inherits_api_key_from_config() {
        let config_content = r#"
provider = "zai"

[providers.zai]
api_key = "inherited-key"
model = "glm-5"
base_url = "https://api.z.ai/v4"
timeout = 120
"#;
        let config: Config = toml::from_str(config_content).unwrap();
        let pc = config.providers.get("zai").unwrap();

        // Role uses zai but with a different model — should inherit api_key, base_url, timeout
        let provider = ProviderFactory::create_for_role(
            "zai",
            "glm-4.7",
            None, // no base_url override — should inherit from config
            8192,
            Some(pc),
        )
        .unwrap();
        assert_eq!(provider.provider_name(), "zai");
    }

    #[test]
    fn test_create_for_role_base_url_override() {
        let config_content = r#"
provider = "zai"

[providers.zai]
api_key = "inherited-key"
model = "glm-5"
base_url = "https://api.z.ai/v4"
"#;
        let config: Config = toml::from_str(config_content).unwrap();
        let pc = config.providers.get("zai").unwrap();

        // Role overrides base_url — should win over config
        let provider = ProviderFactory::create_for_role(
            "zai",
            "glm-4.7",
            Some("https://custom.endpoint/v1"), // override
            8192,
            Some(pc),
        )
        .unwrap();
        assert_eq!(provider.provider_name(), "zai");
    }

    #[test]
    fn test_create_for_role_missing_key() {
        std::env::remove_var("ANTHROPIC_API_KEY");
        let result = ProviderFactory::create_for_role("anthropic", "claude-3", None, 4096, None);
        match result {
            Ok(_) => panic!("expected error for missing API key"),
            Err(e) => assert!(e.to_string().contains("No API key")),
        }
    }

    #[test]
    fn test_create_for_role_falls_back_to_env_from_config() {
        std::env::set_var("ZAI_API_KEY", "env-key");
        // Config has no api_key — should fallback to env var
        let config_content = r#"
provider = "zai"

[providers.zai]
model = "glm-5"
"#;
        let config: Config = toml::from_str(config_content).unwrap();
        let pc = config.providers.get("zai").unwrap();

        let provider =
            ProviderFactory::create_for_role("zai", "glm-4.7", None, 4096, Some(pc)).unwrap();
        assert_eq!(provider.provider_name(), "zai");
        std::env::remove_var("ZAI_API_KEY");
    }
}
