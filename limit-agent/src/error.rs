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
    #[error("Team error: {0}")]
    TeamError(String),
    #[error("LLM provider error: {0}")]
    LlmError(String),
    #[error("Actor error: {0}")]
    ActorError(String),
    #[error("Phase '{phase}' timed out after {seconds}s")]
    PhaseTimeout { phase: String, seconds: u64 },
}

impl Clone for AgentError {
    fn clone(&self) -> Self {
        match self {
            AgentError::ToolError(s) => AgentError::ToolError(s.clone()),
            AgentError::StateError(s) => AgentError::StateError(s.clone()),
            AgentError::SandboxError(s) => AgentError::SandboxError(s.clone()),
            AgentError::IoError(s) => AgentError::IoError(s.clone()),
            AgentError::BincodeError(s) => AgentError::BincodeError(s.clone()),
            AgentError::TeamError(s) => AgentError::TeamError(s.clone()),
            AgentError::LlmError(s) => AgentError::LlmError(s.clone()),
            AgentError::ActorError(s) => AgentError::ActorError(s.clone()),
            AgentError::PhaseTimeout { phase, seconds } => AgentError::PhaseTimeout {
                phase: phase.clone(),
                seconds: *seconds,
            },
        }
    }
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        AgentError::IoError(err.to_string())
    }
}

impl From<limit_llm::LlmError> for AgentError {
    fn from(err: limit_llm::LlmError) -> Self {
        AgentError::LlmError(err.to_string())
    }
}
