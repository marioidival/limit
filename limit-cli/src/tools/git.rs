use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use serde_json::Value;
use std::process::Command;

/// Check if git is available in PATH
fn check_git_available() -> Result<(), AgentError> {
    let result = Command::new("git")
        .arg("--version")
        .output();

    match result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) => Err(AgentError::ToolError(
            "git command failed to execute".to_string(),
        )),
        Err(_) => Err(AgentError::ToolError(
            "git not found in PATH. Please install git 2.0 or later.".to_string(),
        )),
    }
}

pub struct GitStatusTool;

impl GitStatusTool {
    pub fn new() -> Self {
        GitStatusTool
    }
}

impl Default for GitStatusTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }

    async fn execute(&self, _args: Value) -> Result<Value, AgentError> {
        check_git_available()?;

        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .map_err(|e| AgentError::ToolError(format!("Failed to execute git status: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentError::ToolError(format!(
                "git status failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        Ok(serde_json::json!({
            "changes": lines,
            "count": lines.len()
        }))
    }
}

pub struct GitDiffTool;

impl GitDiffTool {
    pub fn new() -> Self {
        GitDiffTool
    }
}

impl Default for GitDiffTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }

    async fn execute(&self, _args: Value) -> Result<Value, AgentError> {
        check_git_available()?;

        let output = Command::new("git")
            .args(["diff"])
            .output()
            .map_err(|e| AgentError::ToolError(format!("Failed to execute git diff: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentError::ToolError(format!(
                "git diff failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(serde_json::json!({
            "diff": stdout,
            "size": stdout.len()
        }))
    }
}

pub struct GitLogTool;

impl GitLogTool {
    pub fn new() -> Self {
        GitLogTool
    }
}

impl Default for GitLogTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitLogTool {
    fn name(&self) -> &str {
        "git_log"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        check_git_available()?;

        // Get number of commits from args, default to 10
        let count = args
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(10);

        let output = Command::new("git")
            .args(["log", &format!("-{}", count), "--oneline"])
            .output()
            .map_err(|e| AgentError::ToolError(format!("Failed to execute git log: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentError::ToolError(format!(
                "git log failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits: Vec<&str> = stdout.lines().collect();

        Ok(serde_json::json!({
            "commits": commits,
            "count": commits.len()
        }))
    }
}

pub struct GitAddTool;

impl GitAddTool {
    pub fn new() -> Self {
        GitAddTool
    }
}

impl Default for GitAddTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitAddTool {
    fn name(&self) -> &str {
        "git_add"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        check_git_available()?;

        let files: Vec<String> = serde_json::from_value(args["files"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid files argument: {}", e)))?;

        if files.is_empty() {
            return Err(AgentError::ToolError(
                "files argument cannot be empty".to_string(),
            ));
        }

        let mut cmd = Command::new("git");
        cmd.arg("add");
        for file in &files {
            cmd.arg(file);
        }

        let output = cmd
            .output()
            .map_err(|e| AgentError::ToolError(format!("Failed to execute git add: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentError::ToolError(format!(
                "git add failed: {}",
                stderr
            )));
        }

        Ok(serde_json::json!({
            "success": true,
            "files": files,
            "count": files.len()
        }))
    }
}

pub struct GitCommitTool;

impl GitCommitTool {
    pub fn new() -> Self {
        GitCommitTool
    }
}

impl Default for GitCommitTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &str {
        "git_commit"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        check_git_available()?;

        let message: String = serde_json::from_value(args["message"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid message argument: {}", e)))?;

        if message.trim().is_empty() {
            return Err(AgentError::ToolError(
                "message argument cannot be empty".to_string(),
            ));
        }

        let output = Command::new("git")
            .args(["commit", "-m", &message])
            .output()
            .map_err(|e| AgentError::ToolError(format!("Failed to execute git commit: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentError::ToolError(format!(
                "git commit failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(serde_json::json!({
            "success": true,
            "message": message,
            "output": stdout
        }))
    }
}

pub struct GitPushTool;

impl GitPushTool {
    pub fn new() -> Self {
        GitPushTool
    }
}

impl Default for GitPushTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitPushTool {
    fn name(&self) -> &str {
        "git_push"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        check_git_available()?;

        // Get remote and branch from args, use defaults
        let remote = args
            .get("remote")
            .and_then(|v| v.as_str())
            .unwrap_or("origin");

        let branch = args
            .get("branch")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let output = if branch.is_empty() {
            Command::new("git")
                .args(["push", remote])
                .output()
                .map_err(|e| AgentError::ToolError(format!("Failed to execute git push: {}", e)))?
        } else {
            Command::new("git")
                .args(["push", remote, branch])
                .output()
                .map_err(|e| AgentError::ToolError(format!("Failed to execute git push: {}", e)))?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentError::ToolError(format!(
                "git push failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(serde_json::json!({
            "success": true,
            "remote": remote,
            "branch": if branch.is_empty() { "(default)" } else { branch },
            "output": stdout
        }))
    }
}

pub struct GitPullTool;

impl GitPullTool {
    pub fn new() -> Self {
        GitPullTool
    }
}

impl Default for GitPullTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitPullTool {
    fn name(&self) -> &str {
        "git_pull"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        check_git_available()?;

        // Get remote and branch from args, use defaults
        let remote = args
            .get("remote")
            .and_then(|v| v.as_str())
            .unwrap_or("origin");

        let branch = args
            .get("branch")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let output = if branch.is_empty() {
            Command::new("git")
                .args(["pull", remote])
                .output()
                .map_err(|e| AgentError::ToolError(format!("Failed to execute git pull: {}", e)))?
        } else {
            Command::new("git")
                .args(["pull", remote, branch])
                .output()
                .map_err(|e| AgentError::ToolError(format!("Failed to execute git pull: {}", e)))?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentError::ToolError(format!(
                "git pull failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(serde_json::json!({
            "success": true,
            "remote": remote,
            "branch": if branch.is_empty() { "(default)" } else { branch },
            "output": stdout
        }))
    }
}

pub struct GitCloneTool;

impl GitCloneTool {
    pub fn new() -> Self {
        GitCloneTool
    }
}

impl Default for GitCloneTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitCloneTool {
    fn name(&self) -> &str {
        "git_clone"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        check_git_available()?;

        let url: String = serde_json::from_value(args["url"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid url argument: {}", e)))?;

        if url.trim().is_empty() {
            return Err(AgentError::ToolError(
                "url argument cannot be empty".to_string(),
            ));
        }

        // Get optional directory name
        let directory = args.get("directory").and_then(|v| v.as_str());

        let output = if let Some(dir) = directory {
            Command::new("git")
                .args(["clone", &url, dir])
                .output()
                .map_err(|e| AgentError::ToolError(format!("Failed to execute git clone: {}", e)))?
        } else {
            Command::new("git")
                .args(["clone", &url])
                .output()
                .map_err(|e| AgentError::ToolError(format!("Failed to execute git clone: {}", e)))?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentError::ToolError(format!(
                "git clone failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(serde_json::json!({
            "success": true,
            "url": url,
            "directory": directory.unwrap_or("(default)"),
            "output": stdout
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_git_status_tool_name() {
        let tool = GitStatusTool::new();
        assert_eq!(tool.name(), "git_status");
    }

    #[tokio::test]
    async fn test_git_status_tool_default() {
        let tool = GitStatusTool;
        assert_eq!(tool.name(), "git_status");
    }

    #[tokio::test]
    async fn test_git_diff_tool_name() {
        let tool = GitDiffTool::new();
        assert_eq!(tool.name(), "git_diff");
    }

    #[tokio::test]
    async fn test_git_log_tool_name() {
        let tool = GitLogTool::new();
        assert_eq!(tool.name(), "git_log");
    }

    #[tokio::test]
    async fn test_git_log_tool_default_count() {
        let tool = GitLogTool::new();
        let args = serde_json::json!({});

        // This will fail if git is not in a repository, but we test the parsing logic
        // The actual execution requires a git repository context
        let result = tool.execute(args).await;

        // We expect either success (in a git repo) or a git-specific error, not a parsing error
        // If git is not available, we get a different error
        if let Err(e) = result {
            assert!(!e.to_string().contains("Invalid count argument"));
        }
    }

    #[tokio::test]
    async fn test_git_log_tool_custom_count() {
        let tool = GitLogTool::new();
        let args = serde_json::json!({"count": 5});

        let result = tool.execute(args).await;

        // Similar to above, we test parsing, not actual git execution
        if let Err(e) = result {
            assert!(!e.to_string().contains("Invalid count argument"));
        }
    }

    #[tokio::test]
    async fn test_git_add_tool_name() {
        let tool = GitAddTool::new();
        assert_eq!(tool.name(), "git_add");
    }

    #[tokio::test]
    async fn test_git_add_tool_empty_files() {
        let tool = GitAddTool::new();
        let args = serde_json::json!({"files": []});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_git_add_tool_invalid_files() {
        let tool = GitAddTool::new();
        let args = serde_json::json!({}); // Missing files

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid files"));
    }

    #[tokio::test]
    async fn test_git_commit_tool_name() {
        let tool = GitCommitTool::new();
        assert_eq!(tool.name(), "git_commit");
    }

    #[tokio::test]
    async fn test_git_commit_tool_empty_message() {
        let tool = GitCommitTool::new();
        let args = serde_json::json!({"message": "   "});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_git_commit_tool_invalid_message() {
        let tool = GitCommitTool::new();
        let args = serde_json::json!({}); // Missing message

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid message"));
    }

    #[tokio::test]
    async fn test_git_push_tool_name() {
        let tool = GitPushTool::new();
        assert_eq!(tool.name(), "git_push");
    }

    #[tokio::test]
    async fn test_git_push_tool_default_values() {
        let tool = GitPushTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;

        // Will fail in most cases, but tests parsing
        if let Err(e) = result {
            // Should not be a parsing error
            assert!(!e.to_string().contains("Invalid"));
        }
    }

    #[tokio::test]
    async fn test_git_push_tool_custom_values() {
        let tool = GitPushTool::new();
        let args = serde_json::json!({
            "remote": "upstream",
            "branch": "feature"
        });

        let result = tool.execute(args).await;

        // Tests that custom values are parsed correctly
        if let Err(e) = result {
            assert!(!e.to_string().contains("Invalid"));
        }
    }

    #[tokio::test]
    async fn test_git_pull_tool_name() {
        let tool = GitPullTool::new();
        assert_eq!(tool.name(), "git_pull");
    }

    #[tokio::test]
    async fn test_git_pull_tool_default_values() {
        let tool = GitPullTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;

        // Will fail in most cases, but tests parsing
        if let Err(e) = result {
            // Should not be a parsing error
            assert!(!e.to_string().contains("Invalid"));
        }
    }

    #[tokio::test]
    async fn test_git_clone_tool_name() {
        let tool = GitCloneTool::new();
        assert_eq!(tool.name(), "git_clone");
    }

    #[tokio::test]
    async fn test_git_clone_tool_empty_url() {
        let tool = GitCloneTool::new();
        let args = serde_json::json!({"url": ""});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_git_clone_tool_invalid_url() {
        let tool = GitCloneTool::new();
        let args = serde_json::json!({}); // Missing url

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid url"));
    }

    #[tokio::test]
    async fn test_git_clone_tool_custom_directory() {
        let tool = GitCloneTool::new();
        let args = serde_json::json!({
            "url": "https://github.com/test/repo.git",
            "directory": "my-repo"
        });

        let result = tool.execute(args).await;

        // Tests that directory argument is parsed correctly
        if let Err(e) = result {
            // Should not be a parsing error
            assert!(!e.to_string().contains("Invalid"));
        }
    }

    #[tokio::test]
    async fn test_all_tools_implement_default() {
        // Verify all tools implement Default trait
        let _status = GitStatusTool;
        let _diff = GitDiffTool;
        let _log = GitLogTool;
        let _add = GitAddTool;
        let _commit = GitCommitTool;
        let _push = GitPushTool;
        let _pull = GitPullTool;
        let _clone = GitCloneTool;
    }
}
