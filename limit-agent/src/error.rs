use thiserror::Error;
#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    #[error("Tool error: {0}")]
    ToolError(String),
    #[error("State error: {0}")]
    StateError(String),
    #[error("Sandbox error: {0}")]
    SandboxError(String),
}
