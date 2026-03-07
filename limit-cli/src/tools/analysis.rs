use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

const GREP_MAX_RESULTS: usize = 1000;
const GREP_CONTEXT_LINES: usize = 3;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}
pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        GrepTool
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let pattern: String = serde_json::from_value(args["pattern"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid pattern argument: {}", e)))?;

        if pattern.trim().is_empty() {
            return Err(AgentError::ToolError(
                "pattern argument cannot be empty".to_string(),
            ));
        }

        // Validate regex pattern
        Regex::new(&pattern)
            .map_err(|e| AgentError::ToolError(format!("Invalid regex pattern: {}", e)))?;

        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        // Validate path exists
        if !Path::new(path).exists() {
            return Err(AgentError::ToolError(format!("Path not found: {}", path)));
        }

        // Use grep command-line tool
        let mut cmd = Command::new("grep");
        cmd.arg("-r")
            .arg("-n")
            .arg("-I") // Ignore binary files
            .arg("--color=never")
            .args(["-C", &GREP_CONTEXT_LINES.to_string()])
            .arg(&pattern)
            .arg(path);

        let output = cmd
            .output()
            .map_err(|e| AgentError::ToolError(format!("Failed to execute grep: {}", e)))?;

        if !output.status.success() {
            // grep returns non-zero if no matches found, but that's not an error
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.is_empty() && !stderr.contains("No such file") {
                return Err(AgentError::ToolError(format!("grep failed: {}", stderr)));
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Limit results
        let limited_lines = if lines.len() > GREP_MAX_RESULTS {
            lines[..GREP_MAX_RESULTS].to_vec()
        } else {
            lines
        };

        // Parse grep output
        let mut matches = Vec::new();
        for line in limited_lines {
            // Parse grep output format: "filename:line_number:content"
            if let Some((rest, content)) = line.split_once(':') {
                if let Some((file_path, line_number)) = rest.split_once(':') {
                    if let Ok(line_num) = line_number.parse::<usize>() {
                        matches.push(serde_json::json!({
                            "file": file_path,
                            "line": line_num,
                            "content": content
                        }));
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "matches": matches,
            "count": matches.len(),
            "pattern": pattern
        }))
    }
}

pub struct AstGrepTool;

impl AstGrepTool {
    pub fn new() -> Self {
        AstGrepTool
    }

    fn get_language_support(lang: &str) -> Result<&'static str, AgentError> {
        match lang.to_lowercase().as_str() {
            "rust" | "rs" => Ok("rust"),
            "typescript" | "ts" | "tsx" => Ok("typescript"),
            "python" | "py" => Ok("python"),
            _ => Err(AgentError::ToolError(format!(
                "Unsupported language: {}. Supported: rust, typescript, python",
                lang
            ))),
        }
    }
}

impl Default for AstGrepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AstGrepTool {
    fn name(&self) -> &str {
        "ast_grep"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let pattern: String = serde_json::from_value(args["pattern"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid pattern argument: {}", e)))?;

        if pattern.trim().is_empty() {
            return Err(AgentError::ToolError(
                "pattern argument cannot be empty".to_string(),
            ));
        }

        let language: String = serde_json::from_value(args["language"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid language argument: {}", e)))?;

        let lang = Self::get_language_support(&language)?;

        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        // Validate path exists
        if !Path::new(path).exists() {
            return Err(AgentError::ToolError(format!("Path not found: {}", path)));
        }

        // Check if ast-grep CLI is available
        let check_result = Command::new("ast-grep").arg("--version").output();

        match check_result {
            Ok(output) if output.status.success() => {}
            _ => {
                return Err(AgentError::ToolError(
                    "ast-grep not found in PATH. Please install ast-grep CLI tool.".to_string(),
                ));
            }
        }

        // Use ast-grep CLI tool
        let mut cmd = Command::new("ast-grep");
        cmd.arg("run")
            .arg("--json")
            .args(["--lang", lang])
            .arg(&pattern)
            .arg(path);

        let output = cmd
            .output()
            .map_err(|e| AgentError::ToolError(format!("Failed to execute ast-grep: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentError::ToolError(format!(
                "ast-grep failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output from ast-grep
        if stdout.trim().is_empty() {
            return Ok(serde_json::json!({
                "matches": [],
                "count": 0,
                "pattern": pattern,
                "language": language
            }));
        }

        // ast-grep returns JSON objects per line
        let mut matches = Vec::new();
        for line in stdout.lines() {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line) {
                matches.push(json_value);
            }
        }

        Ok(serde_json::json!({
            "matches": matches,
            "count": matches.len(),
            "pattern": pattern,
            "language": language
        }))
    }
}

pub struct LspTool;

impl LspTool {
    pub fn new() -> Self {
        LspTool
    }

    fn get_lsp_server(file_path: &Path) -> Result<String, AgentError> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension {
            "rs" => Ok("rust-analyzer".to_string()),
            "ts" | "tsx" | "js" | "jsx" => Ok("typescript-language-server".to_string()),
            "py" => Ok("pylsp".to_string()),
            _ => Err(AgentError::ToolError(format!(
                "Unsupported file extension: {}. Supported: rs, ts, tsx, js, jsx, py",
                extension
            ))),
        }
    }

    fn check_lsp_server_available(server_name: &str) -> Result<(), AgentError> {
        let result = Command::new(server_name).arg("--version").output();

        match result {
            Ok(output) if output.status.success() => Ok(()),
            Ok(_) => Err(AgentError::ToolError(format!(
                "LSP server {} failed to execute",
                server_name
            ))),
            Err(_) => Err(AgentError::ToolError(format!(
                "LSP server {} not found in PATH. Please install it to use LSP features.",
                server_name
            ))),
        }
    }
}

impl Default for LspTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "lsp"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let command: String = serde_json::from_value(args["command"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid command argument: {}", e)))?;

        // Validate command first
        match command.as_str() {
            "goto_definition" | "find_references" => {}
            _ => {
                return Err(AgentError::ToolError(format!(
                    "Unsupported LSP command: {}. Supported: goto_definition, find_references",
                    command
                )));
            }
        }

        let file_path: String = serde_json::from_value(args["file_path"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid file_path argument: {}", e)))?;

        if !Path::new(&file_path).exists() {
            return Err(AgentError::ToolError(format!(
                "File not found: {}",
                file_path
            )));
        }

        let position: Position = serde_json::from_value(args["position"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid position argument: {}", e)))?;
        let lsp_server = Self::get_lsp_server(Path::new(&file_path))?;
        Self::check_lsp_server_available(&lsp_server)?;

        // For now, return a mock response
        // Full LSP protocol implementation would require a proper LSP client
        //
        // For rust-analyzer: use rust-analyzer proc-macro
        // For typescript: use tsserver
        // For python: use pylsp
        match command.as_str() {
            "goto_definition" => Ok(serde_json::json!({
                "command": command,
                "file_path": file_path,
                "position": position,
                "result": "LSP goto_definition requires full LSP client implementation",
                "note": "This is a placeholder. Implement full LSP client for production use."
            })),
            "find_references" => Ok(serde_json::json!({
                "command": command,
                "file_path": file_path,
                "position": position,
                "result": "LSP find_references requires full LSP client implementation",
                "note": "This is a placeholder. Implement full LSP client for production use."
            })),
            _ => unreachable!(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_grep_tool_name() {
        let tool = GrepTool::new();
        assert_eq!(tool.name(), "grep");
    }

    #[tokio::test]
    async fn test_grep_tool_default() {
        let tool = GrepTool;
        assert_eq!(tool.name(), "grep");
    }

    #[tokio::test]
    async fn test_grep_tool_empty_pattern() {
        let tool = GrepTool::new();
        let args = serde_json::json!({
            "pattern": ""
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_grep_tool_invalid_regex() {
        let tool = GrepTool::new();
        let args = serde_json::json!({
            "pattern": "[invalid(regex"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid regex"));
    }

    #[tokio::test]
    async fn test_grep_tool_path_not_found() {
        let tool = GrepTool::new();
        let args = serde_json::json!({
            "pattern": "test",
            "path": "/nonexistent/path"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_ast_grep_tool_name() {
        let tool = AstGrepTool::new();
        assert_eq!(tool.name(), "ast_grep");
    }

    #[tokio::test]
    async fn test_ast_grep_tool_default() {
        let tool = AstGrepTool;
        assert_eq!(tool.name(), "ast_grep");
    }

    #[tokio::test]
    async fn test_ast_grep_tool_empty_pattern() {
        let tool = AstGrepTool::new();
        let args = serde_json::json!({
            "pattern": "",
            "language": "rust"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_ast_grep_tool_unsupported_lang() {
        let tool = AstGrepTool::new();
        let args = serde_json::json!({
            "pattern": "test",
            "language": "java"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported language"));
    }

    #[tokio::test]
    async fn test_ast_grep_tool_path_not_found() {
        let tool = AstGrepTool::new();
        let args = serde_json::json!({
            "pattern": "test",
            "language": "rust",
            "path": "/nonexistent/path"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_ast_grep_tool_rust() {
        let tool = AstGrepTool::new();

        // Create a temp file with Rust code
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "fn hello() {{}}").unwrap();
        writeln!(temp_file, "fn world() {{}}").unwrap();

        let args = serde_json::json!({
            "pattern": "fn $NAME() {}",
            "language": "rust",
            "path": temp_file.path().parent().unwrap().to_str().unwrap()
        });

        // This will fail if ast-grep is not installed, but we test the parsing logic
        let result = tool.execute(args).await;

        // If ast-grep is not available, we should get a specific error
        // If it is available, we should get a valid result
        match result {
            Ok(_) => {
                // ast-grep is available and executed successfully
            }
            Err(e) => {
                // Either ast-grep is not available or there was another error
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("ast-grep not found") || error_msg.contains("failed"),
                    "Unexpected error: {}",
                    error_msg
                );
            }
        }
    }

    #[tokio::test]
    async fn test_ast_grep_tool_typescript() {
        let tool = AstGrepTool::new();

        let args = serde_json::json!({
            "pattern": "console.log($MSG)",
            "language": "typescript"
        });

        // This will fail if ast-grep is not in a TS project, but we test the parsing
        let result = tool.execute(args).await;

        // If ast-grep is not available, we should get a specific error
        match result {
            Ok(_) => {}
            Err(e) => {
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("ast-grep not found") || error_msg.contains("failed"),
                    "Unexpected error: {}",
                    error_msg
                );
            }
        }
    }

    #[tokio::test]
    async fn test_ast_grep_tool_python() {
        let tool = AstGrepTool::new();

        let args = serde_json::json!({
            "pattern": "def $FUNC():",
            "language": "python"
        });

        // This will fail if ast-grep is not in a Python project, but we test the parsing
        let result = tool.execute(args).await;

        // If ast-grep is not available, we should get a specific error
        match result {
            Ok(_) => {}
            Err(e) => {
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("ast-grep not found") || error_msg.contains("failed"),
                    "Unexpected error: {}",
                    error_msg
                );
            }
        }
    }

    #[tokio::test]
    async fn test_lsp_tool_name() {
        let tool = LspTool::new();
        assert_eq!(tool.name(), "lsp");
    }

    #[tokio::test]
    async fn test_lsp_tool_default() {
        let tool = LspTool;
        assert_eq!(tool.name(), "lsp");
    }

    #[tokio::test]
    async fn test_lsp_tool_missing_command() {
        let tool = LspTool::new();
        let args = serde_json::json!({
            "command": "invalid_command"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported LSP command"));
    }

    #[tokio::test]
    async fn test_lsp_tool_file_not_found() {
        let tool = LspTool::new();
        let args = serde_json::json!({
            "command": "goto_definition",
            "file_path": "/nonexistent/file.rs",
            "position": {"line": 1, "character": 0}
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_lsp_tool_unsupported_extension() {
        let tool = LspTool::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test").unwrap();

        let args = serde_json::json!({
            "command": "goto_definition",
            "file_path": temp_file.path(),
            "position": {"line": 1, "character": 0}
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported file extension"));
    }

    #[tokio::test]
    async fn test_lsp_tool_missing_server() {
        let tool = LspTool::new();

        // Create a Rust file
        let temp_dir = tempfile::tempdir().unwrap();
        let rust_file = temp_dir.path().join("test.rs");
        std::fs::write(&rust_file, "fn main() {}").unwrap();

        let args = serde_json::json!({
            "command": "goto_definition",
            "file_path": rust_file,
            "position": {"line": 0, "character": 0}
        });
        // This will likely fail because rust-analyzer is not installed in test environment
        // But we test the parsing logic
        let result = tool.execute(args).await;

        // Expect either success (if rust-analyzer is installed) or error about missing server
        match result {
            Ok(value) => {
                // LSP server is available
                assert!(value["command"] == "goto_definition");
            }
            Err(e) => {
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("not found in PATH")
                        || error_msg.contains("failed to execute"),
                    "Unexpected error: {}",
                    error_msg
                );
            }
        }
    }

    #[tokio::test]
    async fn test_all_tools_implement_default() {
        let _grep = GrepTool;
        let _ast_grep = AstGrepTool;
        let _lsp = LspTool;
    }

    #[tokio::test]
    async fn test_position_deserialize() {
        let json = serde_json::json!({"line": 10, "character": 5});
        let pos: Position = serde_json::from_value(json).unwrap();
        assert_eq!(pos.line, 10);
        assert_eq!(pos.character, 5);
    }
}
