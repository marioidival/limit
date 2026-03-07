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
