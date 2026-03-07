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
    #[error("Max iterations ({0}) reached")]
    MaxIterationsReached(u32),
    #[error("Loop detected: {tool_name} with args {args} called {count} times")]
    LoopDetected {
        tool_name: String,
        args: String,
        count: usize,
    },
}

impl Clone for AgentError {
    fn clone(&self) -> Self {
        match self {
            AgentError::ToolError(s) => AgentError::ToolError(s.clone()),
            AgentError::StateError(s) => AgentError::StateError(s.clone()),
            AgentError::SandboxError(s) => AgentError::SandboxError(s.clone()),
            AgentError::IoError(s) => AgentError::IoError(s.clone()),
            AgentError::BincodeError(s) => AgentError::BincodeError(s.clone()),
            AgentError::MaxIterationsReached(n) => AgentError::MaxIterationsReached(*n),
            AgentError::LoopDetected {
                tool_name,
                args,
                count,
            } => AgentError::LoopDetected {
                tool_name: tool_name.clone(),
                args: args.clone(),
                count: *count,
            },
        }
    }
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        AgentError::IoError(err.to_string())
    }
}
