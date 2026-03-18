use crate::error::ConfigError;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs, io};

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Config {
    pub provider: String,
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub browser: BrowserConfigSection,
    /// Raw team config from `[team]` section in `config.toml` (opaque `toml::Value`).
    /// Parse with `limit_agent::team::TeamSection::from_raw(value)`.
    #[serde(default, rename = "team")]
    pub team: Option<toml::Value>,
}

/// Browser configuration section in config.toml
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct BrowserConfigSection {
    /// Enable browser tool (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// Path to agent-browser binary (default: "agent-browser" in PATH)
    #[serde(default)]
    pub binary_path: Option<PathBuf>,
    /// Browser engine to use (default: "chrome")
    #[serde(default = "default_browser_engine")]
    pub engine: String,
    /// Run in headless mode (default: true)
    #[serde(default = "default_true")]
    pub headless: bool,
    /// Timeout for operations in milliseconds (default: 30000)
    #[serde(default = "default_browser_timeout")]
    pub timeout_ms: u64,
}

impl Default for BrowserConfigSection {
    fn default() -> Self {
        Self {
            enabled: false,
            binary_path: None,
            engine: default_browser_engine(),
            headless: default_true(),
            timeout_ms: default_browser_timeout(),
        }
    }
}

fn default_browser_engine() -> String {
    "chrome".to_string()
}

fn default_true() -> bool {
    true
}

fn default_browser_timeout() -> u64 {
    30_000
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// Maximum iterations for agent loop (0 = unlimited, default: 100)
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// Enable thinking/reasoning mode (Z.AI only, default: false)
    #[serde(default)]
    pub thinking_enabled: bool,
    /// Preserve thinking between turns (Z.AI only, default: true)
    /// Set to false for Preserved Thinking in multi-turn conversations
    #[serde(default = "default_clear_thinking")]
    pub clear_thinking: bool,
}

fn default_model() -> String {
    "claude-3-5-sonnet-20241022".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_timeout() -> u64 {
    60
}

/// Maximum allowed timeout in seconds (10 minutes).
/// Prevents accidentally high values (e.g., 300000 = 83 hours).
const MAX_TIMEOUT_SECS: u64 = 600;

fn default_max_iterations() -> usize {
    100
}

fn default_clear_thinking() -> bool {
    true
}

impl ProviderConfig {
    pub fn api_key_or_env(&self, provider: &str) -> Option<String> {
        if let Some(key) = &self.api_key {
            return Some(key.clone());
        }

        match provider {
            "anthropic" => env::var("ANTHROPIC_API_KEY").ok(),
            "openai" => env::var("OPENAI_API_KEY")
                .ok()
                .or_else(|| env::var("ZAI_API_KEY").ok()),
            "zai" => env::var("ZAI_API_KEY").ok(),
            // Local providers don't require API key, return placeholder
            "local" | "ollama" | "lmstudio" | "vllm" => Some("local".to_string()),
            _ => None,
        }
    }
}

impl Config {
    /// Clamp all provider timeouts to MAX_TIMEOUT_SECS and log a warning if adjusted.
    fn clamp_timeouts(mut self) -> Self {
        for (name, cfg) in &mut self.providers {
            if cfg.timeout > MAX_TIMEOUT_SECS {
                tracing::warn!(
                    "Provider '{}' timeout {}s exceeds max {}s — clamped",
                    name,
                    cfg.timeout,
                    MAX_TIMEOUT_SECS,
                );
                cfg.timeout = MAX_TIMEOUT_SECS;
            }
        }
        self
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        // Valid provider names (includes local LLM aliases)
        let valid_providers = [
            "anthropic",
            "openai",
            "zai",
            "local",
            "ollama",
            "lmstudio",
            "vllm",
        ];

        // Check provider field is valid
        if !valid_providers.contains(&self.provider.as_str()) {
            return Err(ConfigError::InvalidProvider(self.provider.clone()));
        }

        // Check provider config exists
        if !self.providers.contains_key(&self.provider) {
            return Err(ConfigError::MissingProvider(self.provider.clone()));
        }

        // Local providers don't require API key
        let local_providers = ["local", "ollama", "lmstudio", "vllm"];
        if local_providers.contains(&self.provider.as_str()) {
            return Ok(());
        }

        // Check active provider has required fields
        let provider_config = self.providers.get(&self.provider).unwrap();
        if provider_config.api_key.is_none() {
            // Check if env var exists
            let env_var = match self.provider.as_str() {
                "anthropic" => "ANTHROPIC_API_KEY",
                "openai" => "OPENAI_API_KEY",
                "zai" => "ZAI_API_KEY",
                _ => "API_KEY",
            };
            if env::var(env_var).is_err() {
                // For openai, also check ZAI_API_KEY as fallback
                let env_var_display = if self.provider == "openai" {
                    "OPENAI_API_KEY or ZAI_API_KEY"
                } else {
                    env_var
                };
                return Err(ConfigError::MissingApiKey {
                    provider: self.provider.clone(),
                    env_var: env_var_display.to_string(),
                });
            }
        }

        Ok(())
    }
}

impl Config {
    pub fn load() -> Result<Self, io::Error> {
        let config_path = config_path();

        if !config_path.exists() {
            return Ok(Config::default());
        }

        let config_content = fs::read_to_string(&config_path)?;

        // Check for old format (detect by presence of 'api_key' at top level)
        if config_content.contains("api_key") && !config_content.contains("[providers.") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Old config format detected. Please update to multi-provider format.\n\nNew format:\nprovider = \"anthropic\"\n\n[providers.anthropic]\nmodel = \"claude-3-5-sonnet-20241022\"\napi_key = \"...\"  # optional, falls back to ANTHROPIC_API_KEY env var"
            ));
        }

        let config: Config = toml::from_str(&config_content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Clamp timeout values to MAX_TIMEOUT_SECS
        let config = Config::clamp_timeouts(config);

        config
            .validate()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                api_key: None,
                model: default_model(),
                base_url: None,
                max_tokens: default_max_tokens(),
                timeout: default_timeout(),
                max_iterations: default_max_iterations(),
                thinking_enabled: false,
                clear_thinking: true,
            },
        );
        Config {
            provider: "anthropic".to_string(),
            providers,
            browser: BrowserConfigSection::default(),
            team: None,
        }
    }
}

fn config_path() -> std::path::PathBuf {
    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    home_dir.join(".limit").join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_from_actual_config() {
        // Load config from actual path (tests loading with existing file)
        let config = Config::load().unwrap();

        // Should have loaded a valid config (provider and config entry exist)
        assert!(!config.provider.is_empty());
        assert!(config.providers.contains_key(&config.provider));
    }

    #[test]
    fn test_load_valid_config() {
        let config_content = r#"
provider = "anthropic"

[providers.anthropic]
api_key = "sk-ant-test123"
model = "claude-3-5-sonnet-20241022"
"#;

        let config: Config = toml::from_str(config_content).unwrap();

        assert_eq!(config.provider, "anthropic");
        assert!(config.providers.contains_key("anthropic"));
        let anthropic = config.providers.get("anthropic").unwrap();
        assert_eq!(anthropic.api_key, Some("sk-ant-test123".to_string()));
        assert_eq!(anthropic.model, "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_load_partial_config_uses_defaults() {
        let config_content = r#"
provider = "anthropic"

[providers.anthropic]
api_key = "sk-ant-partial"
model = "custom-model"
"#;

        let config: Config = toml::from_str(config_content).unwrap();

        assert_eq!(config.provider, "anthropic");
        let anthropic = config.providers.get("anthropic").unwrap();
        assert_eq!(anthropic.api_key, Some("sk-ant-partial".to_string()));
        assert_eq!(anthropic.model, "custom-model");
        assert!(anthropic.base_url.is_none()); // default
    }

    #[test]
    fn test_load_config_with_base_url() {
        let config_content = r#"
provider = "openai"

[providers.openai]
api_key = "sk-test123"
model = "gpt-4"
base_url = "https://api.z.ai/api/paas/v4/chat/completions"
"#;

        let config: Config = toml::from_str(config_content).unwrap();

        assert_eq!(config.provider, "openai");
        let openai = config.providers.get("openai").unwrap();
        assert_eq!(openai.api_key, Some("sk-test123".to_string()));
        assert_eq!(openai.model, "gpt-4");
        assert_eq!(
            openai.base_url,
            Some("https://api.z.ai/api/paas/v4/chat/completions".to_string())
        );
    }

    #[test]
    fn test_load_config_without_base_url() {
        let config_content = r#"
provider = "anthropic"

[providers.anthropic]
api_key = "sk-ant-test456"
model = "claude-3-5-sonnet-20241022"
"#;

        let config: Config = toml::from_str(config_content).unwrap();

        assert_eq!(config.provider, "anthropic");
        let anthropic = config.providers.get("anthropic").unwrap();
        assert_eq!(anthropic.api_key, Some("sk-ant-test456".to_string()));
        assert_eq!(anthropic.model, "claude-3-5-sonnet-20241022");
        assert!(anthropic.base_url.is_none()); // default
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.provider, "anthropic");
        assert!(config.providers.contains_key("anthropic"));
        let anthropic = config.providers.get("anthropic").unwrap();
        assert_eq!(anthropic.model, "claude-3-5-sonnet-20241022");
        assert!(anthropic.api_key.is_none());
        assert!(anthropic.base_url.is_none());
    }

    #[test]
    fn test_old_format_detection() {
        let config_content = r#"
api_key = "sk-ant-test123"
model = "claude-3-5-sonnet-20241022"
"#;

        let result: Result<Config, _> = toml::from_str(config_content);
        assert!(result.is_err(), "Old format should fail to parse");
    }

    #[test]
    fn test_api_key_or_env_from_config() {
        let provider_config = ProviderConfig {
            api_key: Some("sk-from-config".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
            max_iterations: 100,
            thinking_enabled: false,
            clear_thinking: true,
        };

        let key = provider_config.api_key_or_env("anthropic");
        assert_eq!(key, Some("sk-from-config".to_string()));
    }

    #[test]
    fn test_api_key_or_env_from_env() {
        env::set_var("ANTHROPIC_API_KEY", "sk-from-env");
        let provider_config = ProviderConfig {
            api_key: None,
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
            max_iterations: 100,
            thinking_enabled: false,
            clear_thinking: true,
        };

        let key = provider_config.api_key_or_env("anthropic");
        assert_eq!(key, Some("sk-from-env".to_string()));
        env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_openai_fallback_to_zai_api_key() {
        env::set_var("ZAI_API_KEY", "sk-zai-key");
        let provider_config = ProviderConfig {
            api_key: None,
            model: "gpt-4".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
            max_iterations: 100,
            thinking_enabled: false,
            clear_thinking: true,
        };

        let key = provider_config.api_key_or_env("openai");
        assert_eq!(key, Some("sk-zai-key".to_string()));
        env::remove_var("ZAI_API_KEY");
    }

    #[test]
    fn test_unknown_provider_no_env_var() {
        let provider_config = ProviderConfig {
            api_key: None,
            model: "test-model".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
            max_iterations: 100,
            thinking_enabled: false,
            clear_thinking: true,
        };

        let key = provider_config.api_key_or_env("unknown");
        assert_eq!(key, None);
    }

    #[test]
    fn test_clamp_timeouts() {
        let config_content = r#"
provider = "anthropic"

[providers.anthropic]
api_key = "sk-test"
model = "claude-3-5-sonnet-20241022"
timeout = 300000
"#;
        let mut config: Config = toml::from_str(config_content).unwrap();
        let anthropic = config.providers.get("anthropic").unwrap();
        assert_eq!(anthropic.timeout, 300000);

        config = Config::clamp_timeouts(config);
        let anthropic = config.providers.get("anthropic").unwrap();
        assert_eq!(anthropic.timeout, MAX_TIMEOUT_SECS);
    }

    #[test]
    fn test_clamp_timeouts_normal_value() {
        let config_content = r#"
provider = "anthropic"

[providers.anthropic]
api_key = "sk-test"
model = "claude-3-5-sonnet-20241022"
timeout = 120
"#;
        let config: Config = toml::from_str(config_content).unwrap();
        let config = Config::clamp_timeouts(config);
        let anthropic = config.providers.get("anthropic").unwrap();
        assert_eq!(anthropic.timeout, 120);
    }

    #[test]
    fn test_zai_config_validation() {
        env::set_var("ZAI_API_KEY", "test-zai-key");
        let config_content = r#"
provider = "zai"

[providers.zai]
model = "glm-4.7"
"#;
        let config: Config = toml::from_str(config_content).unwrap();
        config.validate().unwrap();
        env::remove_var("ZAI_API_KEY");
    }
    #[test]
    fn test_zai_api_key_env_var() {
        env::set_var("ZAI_API_KEY", "test-zai-key");
        let provider_config = ProviderConfig {
            api_key: None,
            model: "glm-4.7".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
            max_iterations: 100,
            thinking_enabled: false,
            clear_thinking: true,
        };
        let key = provider_config.api_key_or_env("zai");
        assert_eq!(key, Some("test-zai-key".to_string()));
        env::remove_var("ZAI_API_KEY");
    }
}
