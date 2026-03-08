#[derive(thiserror::Error, Debug)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Persistence error: {0}")]
    PersistenceError(String),
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Invalid provider: '{0}'. Must be 'anthropic' or 'openai'")]
    InvalidProvider(String),

    #[error("Provider '{0}' not found in config")]
    MissingProvider(String),

    #[error("No API key for provider '{provider}'. Set api_key in config or {env_var} env var")]
    MissingApiKey { provider: String, env_var: String },

    #[error("Old config format detected. Please update to multi-provider format")]
    OldFormat,
}
