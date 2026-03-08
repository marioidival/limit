#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    #[error("Tool error: {0}")]
    ToolError(String),
    #[error("State error: {0}")]
    StateError(String),
    #[error("Sandbox error: {0}")]
    SandboxError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Bincode error: {0}")]
    BincodeError(String),
}

impl Clone for AgentError {
    fn clone(&self) -> Self {
        match self {
            AgentError::ToolError(s) => AgentError::ToolError(s.clone()),
            AgentError::StateError(s) => AgentError::StateError(s.clone()),
            AgentError::SandboxError(s) => AgentError::SandboxError(s.clone()),
            AgentError::IoError(s) => AgentError::IoError(s.clone()),
            AgentError::BincodeError(s) => AgentError::BincodeError(s.clone()),
        }
    }
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        AgentError::IoError(err.to_string())
    }
}
