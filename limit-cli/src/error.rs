#[allow(clippy::enum_variant_names)]
#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Agent error: {0}")]
    AgentError(#[from] limit_agent::error::AgentError),
    #[error("Session tree error: {0}")]
    SessionTreeError(#[from] crate::session_tree::SessionTreeError),
    #[error("{0}")]
    Other(String),
}
