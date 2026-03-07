use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        BashTool
    }

    const DEFAULT_TIMEOUT_SECS: u64 = 60;

    /// Check if a command is dangerous and should be blocked
    fn is_dangerous_command(command: &str) -> bool {
        let lower_cmd = command.to_lowercase();
        
        // Block specific dangerous patterns
        let dangerous_patterns = [
            "rm -rf /",           // Remove root directory
            "rm -rf /*",          // Remove all files
            ":(){ :|:& };:",      // Fork bomb
            "dd if=/dev/zero",    // Disk wipe
            "mkfs.",              // Format filesystem
            "mv / /dev/null",     // Move root to null
            "chmod -R 777 /",     // Recursive chmod on root
            "chown -R",           // Recursive chown on root
            "killall -9",         // Kill all processes (with no filter)
            "kill -9 -1",         // Kill all processes
            "shred",              // Secure delete
            "> /dev/sda",         // Direct write to disk
            "wget http://",       // Random downloads
            "curl http://",       // Random downloads
        ];

        dangerous_patterns.iter().any(|pattern| lower_cmd.contains(pattern))
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        // Extract command from arguments
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::ToolError("Missing 'command' argument".to_string()))?;

        // Check for dangerous commands
        if Self::is_dangerous_command(command) {
            return Err(AgentError::ToolError(format!(
                "Dangerous command blocked: {}",
                command
            )));
        }

        // Get working directory from current directory or args
        let workdir = args
            .get("workdir")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        // Validate working directory exists
        if !Path::new(workdir).exists() {
            return Err(AgentError::ToolError(format!(
                "Working directory does not exist: {}",
                workdir
            )));
        }

        // Get timeout from args or use default
        let timeout_secs = args
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(Self::DEFAULT_TIMEOUT_SECS);

        // Execute command with timeout
        let result = timeout(
            Duration::from_secs(timeout_secs),
            Command::new("sh")
                .args(["-c", command])
                .current_dir(workdir)
                .output(),
        )
        .await;

        match result {
            Ok(output) => {
                let output = output.map_err(|e| {
                    AgentError::ToolError(format!("Failed to execute command: {}", e))
                })?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                Ok(serde_json::json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": exit_code
                }))
            }
            Err(_) => Err(AgentError::ToolError(format!(
                "Command timed out after {} seconds",
                timeout_secs
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bash_tool_name() {
        let tool = BashTool::new();
        assert_eq!(tool.name(), "bash");
    }

    #[tokio::test]
    async fn test_bash_tool_default() {
        let tool = BashTool;
        assert_eq!(tool.name(), "bash");
    }

    #[tokio::test]
    async fn test_bash_tool_execute_simple() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "echo 'hello world'"
        });

        let result = tool.execute(args).await.unwrap();
        
        assert_eq!(result["stdout"], "hello world\n");
        assert_eq!(result["exit_code"], 0);
        assert!(result["stderr"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_bash_tool_execute_with_stderr() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "echo 'error' >&2; exit 1"
        });

        let result = tool.execute(args).await.unwrap();
        
        assert_eq!(result["stderr"], "error\n");
        assert_eq!(result["exit_code"], 1);
    }

    #[tokio::test]
    async fn test_bash_tool_missing_command() {
        let tool = BashTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'command'"));
    }

    #[tokio::test]
    async fn test_bash_tool_dangerous_command_blocked() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "rm -rf /"
        });

        let result = tool.execute(args).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Dangerous command blocked"));
    }

    #[tokio::test]
    async fn test_bash_tool_fork_bomb_blocked() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": ":(){ :|:& };:"
        });

        let result = tool.execute(args).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Dangerous command blocked"));
    }

    #[tokio::test]
    async fn test_bash_tool_timeout() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "sleep 10",
            "timeout": 1
        });

        let result = tool.execute(args).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_bash_tool_invalid_workdir() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "echo test",
            "workdir": "/nonexistent/directory"
        });

        let result = tool.execute(args).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Working directory does not exist"));
    }

    #[tokio::test]
    async fn test_bash_tool_current_dir() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "pwd"
        });

        let result = tool.execute(args).await.unwrap();
        
        // Just check we got some output
        assert!(!result["stdout"].as_str().unwrap().is_empty());
        assert_eq!(result["exit_code"], 0);
    }
}
