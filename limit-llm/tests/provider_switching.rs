use limit_llm::{
    BrowserConfigSection, CompactionSettings, Config, ProviderConfig, ProviderFactory,
};
use std::collections::HashMap;

#[test]
fn test_anthropic_provider_creation() {
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
        provider: "anthropic".to_string(),
        providers,
        browser: BrowserConfigSection::default(),
        compaction: CompactionSettings::default(),
            cache: CacheSettings::default(),
            cache: CacheSettings::default(),
    };

    let provider = ProviderFactory::create_provider(&config).unwrap();
    assert_eq!(provider.provider_name(), "anthropic");
}

#[test]
fn test_openai_provider_creation() {
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ProviderConfig {
            api_key: Some("test-key".to_string()),
            model: "gpt-4".to_string(),
            base_url: Some("https://api.z.ai/api/paas/v4/chat/completions".to_string()),
            max_tokens: 4096,
            timeout: 60,
            max_iterations: 100,
            thinking_enabled: false,
            clear_thinking: true,
        },
    );
    let config = Config {
        provider: "openai".to_string(),
        providers,
        browser: BrowserConfigSection::default(),
        compaction: CompactionSettings::default(),
            cache: CacheSettings::default(),
            cache: CacheSettings::default(),
    };

    let provider = ProviderFactory::create_provider(&config).unwrap();
    assert_eq!(provider.provider_name(), "openai");
}
