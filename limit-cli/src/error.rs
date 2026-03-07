#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Agent error: {0}")]
    AgentError(#[from] limit_agent::error::AgentError),
}
