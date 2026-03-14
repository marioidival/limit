#[allow(clippy::enum_variant_names)]
#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Readline error: {0}")]
    ReadlineError(#[from] rustyline::error::ReadlineError),
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Agent error: {0}")]
    AgentError(#[from] limit_agent::error::AgentError),
    #[error("{0}")]
    Other(String),
}
