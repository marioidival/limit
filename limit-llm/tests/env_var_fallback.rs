use limit_llm::{Config, ProviderConfig};
use serial_test::serial;
use std::collections::HashMap;
#[test]
#[serial]
fn test_anthropic_env_fallback() {
    // Clean up all env vars first
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("ZAI_API_KEY");

    std::env::set_var("ANTHROPIC_API_KEY", "env-test-key");

    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: None,
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
            max_iterations: 100,
        },
    );
    let config = Config {
        provider: "anthropic".to_string(),
        providers,
    };

    let provider_config = config.providers.get("anthropic").unwrap();
    assert_eq!(
        provider_config.api_key_or_env("anthropic"),
        Some("env-test-key".to_string())
    );

    std::env::remove_var("ANTHROPIC_API_KEY");
}

#[test]
#[serial]
fn test_openai_env_fallback() {
    // Clean up all env vars first
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("ZAI_API_KEY");

    std::env::set_var("OPENAI_API_KEY", "env-test-key");

    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ProviderConfig {
            api_key: None,
            model: "gpt-4".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
            max_iterations: 100,
        },
    );
    let config = Config {
        provider: "openai".to_string(),
        providers,
    };

    let provider_config = config.providers.get("openai").unwrap();
    assert_eq!(
        provider_config.api_key_or_env("openai"),
        Some("env-test-key".to_string())
    );

    std::env::remove_var("OPENAI_API_KEY");
}

#[test]
#[serial]
fn test_openai_zai_api_key_fallback() {
    // Clean up all env vars first
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("ZAI_API_KEY");

    std::env::set_var("ZAI_API_KEY", "zai-env-key");

    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ProviderConfig {
            api_key: None,
            model: "gpt-4".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
            max_iterations: 100,
        },
    );
    let config = Config {
        provider: "openai".to_string(),
        providers,
    };

    let provider_config = config.providers.get("openai").unwrap();
    assert_eq!(
        provider_config.api_key_or_env("openai"),
        Some("zai-env-key".to_string())
    );

    std::env::remove_var("ZAI_API_KEY");
}
