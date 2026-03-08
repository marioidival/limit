use limit_llm::{Config, LlmProvider, ProviderConfig, ProviderFactory, ThinkingConfig, ZaiProvider};
use std::env;


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
fn test_thinking_config() {
    let config = ThinkingConfig::default();
    assert_eq!(config.thinking_enabled, false);
    assert_eq!(config.clear_thinking, true);

    let custom = ThinkingConfig {
        thinking_enabled: true,
        clear_thinking: false,
    };
    assert_eq!(custom.thinking_enabled, true);
    assert_eq!(custom.clear_thinking, false);
}

#[test]
fn test_thinking_config_default() {
    let config = ThinkingConfig::default();
    assert!(!config.thinking_enabled);
    assert!(config.clear_thinking);
}

#[test]
fn test_zai_provider_with_thinking_enabled() {
    let thinking_config = ThinkingConfig {
        thinking_enabled: true,
        clear_thinking: false,
    };

    let provider = ZaiProvider::new(
        "test-key".to_string(),
        None,
        "glm-4.7",
        4096,
        60,
        thinking_config.clone(),
    );

    assert_eq!(provider.provider_name(), "zai");
    assert_eq!(provider.model_name(), "glm-4.7");
}

#[test]
fn test_zai_config_validation() {
    let config_content = r#"
provider = "zai"

[providers.zai]
api_key = "test-key"
model = "glm-4.7"
"#;

    let config: Config = toml::from_str(config_content).unwrap();
    config.validate().unwrap();
}

#[test]
fn test_zai_config_validation_with_env_var() {
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
fn test_zai_provider_factory() {
    let config_content = r#"
provider = "zai"

[providers.zai]
api_key = "test-key"
model = "glm-4.7"
"#;

    let config: Config = toml::from_str(config_content).unwrap();
    let provider = ProviderFactory::create_provider(&config).unwrap();

    assert_eq!(provider.provider_name(), "openai"); // Uses OpenAiProvider for now
    assert_eq!(provider.model_name(), "glm-4.7");
}

#[test]
fn test_zai_api_key_env_var() {
    env::remove_var("ZAI_API_KEY"); // Clean up first
    env::set_var("ZAI_API_KEY", "env-test-key");

    let provider_config = ProviderConfig {
        api_key: None,
        model: "glm-4.7".to_string(),
        base_url: None,
        max_tokens: 4096,
        timeout: 60,
    };

    let key = provider_config.api_key_or_env("zai");
    assert_eq!(key, Some("env-test-key".to_string()));

    env::remove_var("ZAI_API_KEY");
}

#[test]
fn test_zai_api_key_from_config() {
    let provider_config = ProviderConfig {
        api_key: Some("config-key".to_string()),
        model: "glm-4.7".to_string(),
        base_url: None,
        max_tokens: 4096,
        timeout: 60,
    };

    let key = provider_config.api_key_or_env("zai");
    assert_eq!(key, Some("config-key".to_string()));
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

#[test]
fn test_zai_provider_with_all_params() {
    let provider = ZaiProvider::new(
        "my-secret-key".to_string(),
        Some("https://custom.z.ai/api/v4/chat"),
        "glm-5",
        16384,
        300,
        ThinkingConfig {
            thinking_enabled: true,
            clear_thinking: true,
        },
    );

    assert_eq!(provider.provider_name(), "zai");
    assert_eq!(provider.model_name(), "glm-5");
}

#[test]
fn test_zai_config_missing_api_key_no_env() {
    // Ensure no env var is set
    env::remove_var("ZAI_API_KEY");

    let config_content = r#"
provider = "zai"

[providers.zai]
model = "glm-4.7"
"#;

    let config: Config = toml::from_str(config_content).unwrap();
    let result = config.validate();

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("API key"));
    }
}
