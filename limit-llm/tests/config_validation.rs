use limit_llm::{BrowserConfigSection, Config, ProviderConfig, ProviderFactory};
use std::collections::HashMap;

#[test]
fn test_unknown_provider_error() {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
            max_iterations: 100,
            thinking_enabled: false,
            clear_thinking: true,
        },
    );
    let config = Config {
        provider: "openai".to_string(), // Known but not configured
        providers,
        browser: BrowserConfigSection::default(),
    };

    let result = ProviderFactory::create_provider(&config);
    assert!(result.is_err());
    if let Err(e) = result {
        // OpenAI is a known provider but not configured
        assert!(e.to_string().contains("not found in config") || e.to_string().contains("Unknown"));
    }
}

#[test]
fn test_missing_provider_error() {
    let config = Config {
        provider: "anthropic".to_string(),
        providers: HashMap::new(),
        browser: BrowserConfigSection::default(),
    };

    let result = ProviderFactory::create_provider(&config);
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("not found in config"));
    }
}

#[test]
fn test_missing_api_key_error() {
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
            thinking_enabled: false,
            clear_thinking: true,
        },
    );
    let config = Config {
        provider: "anthropic".to_string(),
        providers,
        browser: BrowserConfigSection::default(),
    };

    // Ensure no env var is set
    std::env::remove_var("ANTHROPIC_API_KEY");

    let result = ProviderFactory::create_provider(&config);
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("No API key found"));
    }
}
