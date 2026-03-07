use crate::error::AgentError;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;

    async fn execute(&self, args: Value) -> Result<Value, AgentError>;
}

/// Example tool that echoes back its input arguments
pub struct EchoTool;

impl EchoTool {
    pub fn new() -> Self {
        EchoTool
    }
}

impl Default for EchoTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        // Echo back the input arguments
        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_tool_name() {
        let tool = EchoTool::new();
        assert_eq!(tool.name(), "echo");
    }

    #[tokio::test]
    async fn test_echo_tool_execute() {
        let tool = EchoTool::new();
        let input = serde_json::json!({
            "message": "hello",
            "count": 42
        });

        let result = tool.execute(input.clone()).await.unwrap();
        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn test_echo_tool_default() {
        let tool = EchoTool::default();
        let input = serde_json::json!("test");

        let result = tool.execute(input.clone()).await.unwrap();
        assert_eq!(result, input);
    }
}
