//! Browser executor abstraction
//!
//! Provides the trait for browser execution and CLI implementation.

use super::config::BrowserConfig;
use async_trait::async_trait;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::process::Command as AsyncCommand;
use tokio::time::{timeout, Duration};

/// Result from browser operations
#[derive(Debug, Clone)]
pub struct BrowserOutput {
    /// stdout from the command
    pub stdout: String,
    /// stderr from the command
    pub stderr: String,
    /// Exit code (None if killed)
    pub exit_code: Option<i32>,
    /// Whether the operation succeeded
    pub success: bool,
}

impl BrowserOutput {
    /// Create a successful output
    pub fn success(stdout: String) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            exit_code: Some(0),
            success: true,
        }
    }

    /// Create a failed output
    pub fn failure(stderr: String, exit_code: Option<i32>) -> Self {
        Self {
            stdout: String::new(),
            stderr,
            exit_code,
            success: false,
        }
    }
}

/// Error type for browser operations
#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    /// Failed to execute the browser command
    #[error("Failed to execute browser command: {0}")]
    ExecutionFailed(String),

    /// Operation timed out
    #[error("Browser operation timed out after {0}ms")]
    Timeout(u64),

    /// Invalid arguments provided
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// Browser not installed or not found
    #[error("Browser binary not found: {0}")]
    NotFound(String),

    /// Parse error for output
    #[error("Failed to parse output: {0}")]
    ParseError(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

/// Trait for browser execution backends
#[async_trait]
pub trait BrowserExecutor: Send + Sync {
    /// Execute a browser command with the given arguments
    async fn execute(&self, args: &[&str]) -> Result<BrowserOutput, BrowserError>;

    /// Check if the browser daemon is running
    fn is_daemon_running(&self) -> bool;

    /// Get the configuration
    fn config(&self) -> &BrowserConfig;
}

/// CLI-based executor using agent-browser binary
pub struct CliExecutor {
    config: BrowserConfig,
    /// Track if we have an open session
    is_open: Arc<AtomicBool>,
}

impl CliExecutor {
    /// Create a new CLI executor with the given configuration
    pub fn new(config: BrowserConfig) -> Self {
        Self {
            config,
            is_open: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Build a command with common configuration
    fn build_command(&self, args: &[&str]) -> AsyncCommand {
        let mut cmd: AsyncCommand = AsyncCommand::new(self.config.binary());

        // Add engine flag if not Chrome (default)
        if self.config.engine.as_arg() != "chrome" {
            cmd.arg("--engine").arg(self.config.engine.as_arg());
        }

        // Add headless flag
        if self.config.headless {
            cmd.arg("--headless");
        }

        cmd.args(args);
        cmd
    }

    /// Check if agent-browser is installed
    pub fn is_installed(&self) -> bool {
        Command::new(self.config.binary())
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[async_trait]
impl BrowserExecutor for CliExecutor {
    async fn execute(&self, args: &[&str]) -> Result<BrowserOutput, BrowserError> {
        let mut cmd = self.build_command(args);

        let result = timeout(Duration::from_millis(self.config.timeout_ms), cmd.output())
            .await
            .map_err(|_| BrowserError::Timeout(self.config.timeout_ms))?
            .map_err(|e| BrowserError::ExecutionFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();
        let success = result.status.success();
        let exit_code = result.status.code();

        // Track open state for specific commands
        if !args.is_empty() {
            match args[0] {
                "open" => {
                    self.is_open.store(true, Ordering::SeqCst);
                }
                "close" => {
                    self.is_open.store(false, Ordering::SeqCst);
                }
                _ => {}
            }
        }

        Ok(BrowserOutput {
            stdout,
            stderr,
            exit_code,
            success,
        })
    }

    fn is_daemon_running(&self) -> bool {
        self.is_open.load(Ordering::SeqCst)
    }

    fn config(&self) -> &BrowserConfig {
        &self.config
    }
}

impl Drop for CliExecutor {
    fn drop(&mut self) {
        // Close browser on drop if still open
        if self.is_open.load(Ordering::SeqCst) {
            let _ = Command::new(self.config.binary()).arg("close").output();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_output_success() {
        let output = BrowserOutput::success("test output".to_string());
        assert!(output.success);
        assert_eq!(output.stdout, "test output");
        assert_eq!(output.exit_code, Some(0));
    }

    #[test]
    fn test_browser_output_failure() {
        let output = BrowserOutput::failure("error message".to_string(), Some(1));
        assert!(!output.success);
        assert_eq!(output.stderr, "error message");
        assert_eq!(output.exit_code, Some(1));
    }

    #[test]
    fn test_browser_error_display() {
        let err = BrowserError::Timeout(5000);
        assert!(err.to_string().contains("5000ms"));

        let err = BrowserError::NotFound("agent-browser".to_string());
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_cli_executor_creation() {
        let config = BrowserConfig::default();
        let executor = CliExecutor::new(config);
        assert!(!executor.is_daemon_running());
    }
}
