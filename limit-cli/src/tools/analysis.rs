use ast_grep_core::Pattern;
use ast_grep_language::{LanguageExt, SupportLang};
use async_trait::async_trait;
use ignore::WalkBuilder;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
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

    fn get_language_support(lang: &str) -> Result<SupportLang, AgentError> {
        lang.parse()
            .map_err(|_| AgentError::ToolError(format!("Unsupported language: {}. Use a valid language name or alias (e.g., rs, py, js, rust, python, javascript).", lang)))
    }

    async fn execute_search(&self, args: Value) -> Result<Value, AgentError> {
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

        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(AgentError::ToolError(format!("Path not found: {}", path)));
        }

        let context_after = args
            .get("context_after")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let context_before = args
            .get("context_before")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let globs: Option<Vec<String>> = args.get("globs").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|val| val.as_str().map(String::from))
                    .collect()
            })
        });

        let mut all_matches = Vec::new();

        let search_pattern = Pattern::try_new(&pattern, lang)
            .map_err(|e| AgentError::ToolError(format!("Invalid pattern: {}", e)))?;

        if path_obj.is_file() {
            let content = fs::read_to_string(path_obj)
                .map_err(|e| AgentError::ToolError(format!("Failed to read file: {}", e)))?;

            let grep = lang.ast_grep(&content);

            for match_ in grep.root().find_all(&search_pattern) {
                let line = match_.start_pos().line();
                let text = match_.text();

                let mut match_obj = serde_json::json!({
                    "file": path,
                    "line": line,
                    "text": text,
                    "language": language
                });

                if context_after > 0 || context_before > 0 {
                    let lines: Vec<&str> = content.lines().collect();
                    let start_line = line.saturating_sub(context_before as usize);
                    let end_line = (line + context_after as usize + 1).min(lines.len());

                    let context_lines: Vec<String> = lines[start_line..end_line]
                        .iter()
                        .map(|s: &&str| s.to_string())
                        .collect();

                    match_obj["context_lines"] = serde_json::json!(context_lines);
                }

                all_matches.push(match_obj);
            }
        } else {
            let mut builder = WalkBuilder::new(path);

            if let Some(ref glob_patterns) = globs {
                let mut override_builder = ignore::overrides::OverrideBuilder::new(path);
                for glob in glob_patterns {
                    if let Err(e) = override_builder.add(glob) {
                        return Err(AgentError::ToolError(format!(
                            "Invalid glob pattern '{}': {}",
                            glob, e
                        )));
                    }
                }
                if let Ok(overrides) = override_builder.build() {
                    builder.overrides(overrides);
                }
            }

            for entry in builder.build().filter_map(|e| e.ok()) {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    let file_path = entry.path();

                    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        let lang_str = lang.to_string();

                        let matches_lang = match lang_str.as_str() {
                            "rust" => ext_lower == "rs",
                            "python" => ext_lower == "py",
                            "javascript" => ext_lower == "js",
                            "typescript" => ext_lower == "ts",
                            "tsx" => ext_lower == "tsx",
                            "go" => ext_lower == "go",
                            "java" => ext_lower == "java",
                            "c" => ext_lower == "c",
                            "cpp" => ext_lower == "cpp" || ext_lower == "cc" || ext_lower == "cxx",
                            "csharp" => ext_lower == "cs",
                            "ruby" => ext_lower == "rb",
                            "php" => ext_lower == "php",
                            "swift" => ext_lower == "swift",
                            "kotlin" => ext_lower == "kt",
                            "scala" => ext_lower == "scala",
                            "haskell" => ext_lower == "hs",
                            "lua" => ext_lower == "lua",
                            "elixir" => ext_lower == "ex",
                            "nix" => ext_lower == "nix",
                            "solidity" => ext_lower == "sol",
                            "bash" => ext_lower == "sh" || ext_lower == "bash",
                            "yaml" => ext_lower == "yaml" || ext_lower == "yml",
                            "json" => ext_lower == "json",
                            "html" => ext_lower == "html" || ext_lower == "htm",
                            "css" => ext_lower == "css",
                            _ => false,
                        };

                        if !matches_lang {
                            continue;
                        }
                    }

                    let content = match fs::read_to_string(file_path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    let grep = lang.ast_grep(&content);

                    for match_ in grep.root().find_all(&search_pattern) {
                        let line = match_.start_pos().line();
                        let text = match_.text();
                        let display_path = file_path.display().to_string();

                        let mut match_obj = serde_json::json!({
                            "file": display_path,
                            "line": line,
                            "text": text,
                            "language": language
                        });

                        if context_after > 0 || context_before > 0 {
                            let lines: Vec<&str> = content.lines().collect();
                            let start_line = line.saturating_sub(context_before as usize);
                            let end_line = (line + context_after as usize + 1).min(lines.len());

                            let context_lines: Vec<String> = lines[start_line..end_line]
                                .iter()
                                .map(|s: &&str| s.to_string())
                                .collect();

                            match_obj["context_lines"] = serde_json::json!(context_lines);
                        }

                        all_matches.push(match_obj);
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "matches": all_matches,
            "count": all_matches.len(),
            "pattern": pattern,
            "language": language,
            "command": "search"
        }))
    }

    async fn execute_replace(&self, args: Value) -> Result<Value, AgentError> {
        let pattern: String = serde_json::from_value(args["pattern"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid pattern argument: {}", e)))?;

        if pattern.trim().is_empty() {
            return Err(AgentError::ToolError(
                "pattern argument cannot be empty".to_string(),
            ));
        }

        let language: String = serde_json::from_value(args["language"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid language argument: {}", e)))?;

        let rewrite: String = serde_json::from_value(args["rewrite"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid rewrite argument: {}", e)))?;

        if rewrite.trim().is_empty() {
            return Err(AgentError::ToolError(
                "rewrite argument cannot be empty".to_string(),
            ));
        }

        let lang = Self::get_language_support(&language)?;

        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(AgentError::ToolError(format!("Path not found: {}", path)));
        }

        let globs: Option<Vec<String>> = args.get("globs").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|val| val.as_str().map(String::from))
                    .collect()
            })
        });

        let mut all_matches = Vec::new();

        if path_obj.is_file() {
            let content = fs::read_to_string(path_obj)
                .map_err(|e| AgentError::ToolError(format!("Failed to read file: {}", e)))?;

            let search_pattern = Pattern::try_new(&pattern, lang)
                .map_err(|e| AgentError::ToolError(format!("Invalid pattern: {}", e)))?;

            let grep = lang.ast_grep(&content);

            for match_ in grep.root().find_all(&search_pattern) {
                let text = match_.text();
                all_matches.push(serde_json::json!({
                    "file": path,
                    "text": text
                }));
            }

            if !all_matches.is_empty() && !dry_run {
                let mut content = content;
                loop {
                    let mut grep = lang.ast_grep(&content);
                    let replaced =
                        grep.replace(pattern.as_str(), rewrite.as_str())
                            .map_err(|e| {
                                AgentError::ToolError(format!("Failed to apply pattern: {}", e))
                            })?;
                    if !replaced {
                        break;
                    }
                    content = grep.generate();
                }
                fs::write(path_obj, content)
                    .map_err(|e| AgentError::ToolError(format!("Failed to write file: {}", e)))?;
            }
        } else {
            let mut builder = WalkBuilder::new(path);

            if let Some(ref glob_patterns) = globs {
                let mut override_builder = ignore::overrides::OverrideBuilder::new(path);
                for glob in glob_patterns {
                    if let Err(e) = override_builder.add(glob) {
                        return Err(AgentError::ToolError(format!(
                            "Invalid glob pattern '{}': {}",
                            glob, e
                        )));
                    }
                }
                if let Ok(overrides) = override_builder.build() {
                    builder.overrides(overrides);
                }
            }

            for entry in builder.build().filter_map(|e| e.ok()) {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    let file_path = entry.path();

                    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        let lang_str = lang.to_string();

                        let matches_lang = match lang_str.as_str() {
                            "rust" => ext_lower == "rs",
                            "python" => ext_lower == "py",
                            "javascript" => ext_lower == "js",
                            "typescript" => ext_lower == "ts",
                            "tsx" => ext_lower == "tsx",
                            "go" => ext_lower == "go",
                            "java" => ext_lower == "java",
                            "c" => ext_lower == "c",
                            "cpp" => ext_lower == "cpp" || ext_lower == "cc" || ext_lower == "cxx",
                            "csharp" => ext_lower == "cs",
                            "ruby" => ext_lower == "rb",
                            "php" => ext_lower == "php",
                            "swift" => ext_lower == "swift",
                            "kotlin" => ext_lower == "kt",
                            "scala" => ext_lower == "scala",
                            "haskell" => ext_lower == "hs",
                            "lua" => ext_lower == "lua",
                            "elixir" => ext_lower == "ex",
                            "nix" => ext_lower == "nix",
                            "solidity" => ext_lower == "sol",
                            "bash" => ext_lower == "sh" || ext_lower == "bash",
                            "yaml" => ext_lower == "yaml" || ext_lower == "yml",
                            "json" => ext_lower == "json",
                            "html" => ext_lower == "html" || ext_lower == "htm",
                            "css" => ext_lower == "css",
                            _ => false,
                        };

                        if !matches_lang {
                            continue;
                        }
                    }

                    let display_path = file_path.display().to_string();
                    let content = match fs::read_to_string(file_path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    let search_pattern = Pattern::try_new(&pattern, lang)
                        .map_err(|e| AgentError::ToolError(format!("Invalid pattern: {}", e)))?;

                    let grep = lang.ast_grep(&content);

                    let file_matches: Vec<serde_json::Value> = grep
                        .root()
                        .find_all(&search_pattern)
                        .map(|match_| {
                            let text = match_.text();
                            serde_json::json!({
                                "file": display_path,
                                "text": text
                            })
                        })
                        .collect();

                    if !file_matches.is_empty() && !dry_run {
                        let mut file_content = content;
                        loop {
                            let mut grep = lang.ast_grep(&file_content);
                            let replaced = grep
                                .replace(pattern.as_str(), rewrite.as_str())
                                .map_err(|e| {
                                    AgentError::ToolError(format!("Failed to apply pattern: {}", e))
                                })?;
                            if !replaced {
                                break;
                            }
                            file_content = grep.generate();
                        }
                        if let Err(e) = fs::write(file_path, file_content) {
                            return Err(AgentError::ToolError(format!(
                                "Failed to write file {}: {}",
                                display_path, e
                            )));
                        }
                    }

                    all_matches.extend(file_matches);
                }
            }
        }

        Ok(serde_json::json!({
            "matches": all_matches,
            "count": all_matches.len(),
            "pattern": pattern,
            "language": language,
            "rewrite": rewrite,
            "dry_run": dry_run,
            "command": "replace"
        }))
    }

    async fn execute_scan(&self, _args: Value) -> Result<Value, AgentError> {
        Err(AgentError::ToolError(
            "scan command is not yet supported via ast-grep crates. Please use the search or replace commands.".to_string(),
        ))
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
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("search");

        match command {
            "search" => self.execute_search(args).await,
            "replace" => self.execute_replace(args).await,
            "scan" => self.execute_scan(args).await,
            _ => Err(AgentError::ToolError(format!(
                "Unsupported command: {}. Supported: search, replace, scan",
                command
            ))),
        }
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot be empty"));
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
    async fn test_ast_grep_search_single_file() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "fn foo() {{}}").unwrap();
        writeln!(temp_file, "fn bar() {{}}").unwrap();
        temp_file.flush().unwrap();

        let args = serde_json::json!({
            "pattern": "fn $NAME() {}",
            "language": "rust",
            "path": temp_file.path()
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["count"], 2);
        assert_eq!(value["matches"].as_array().unwrap().len(), 2);
        assert_eq!(value["command"], "search");
    }

    #[tokio::test]
    async fn test_ast_grep_search_multi_file() {
        let tool = AstGrepTool::new();

        let mut temp_file1 = NamedTempFile::new().unwrap();
        writeln!(temp_file1, "fn foo() {{}}").unwrap();
        writeln!(temp_file1, "fn bar() {{}}").unwrap();
        temp_file1.flush().unwrap();

        let mut temp_file2 = NamedTempFile::new().unwrap();
        writeln!(temp_file2, "fn baz() {{}}").unwrap();
        temp_file2.flush().unwrap();

        let args1 = serde_json::json!({
            "pattern": "fn $NAME() {}",
            "language": "rust",
            "path": temp_file1.path()
        });

        let args2 = serde_json::json!({
            "pattern": "fn $NAME() {}",
            "language": "rust",
            "path": temp_file2.path()
        });

        let result1 = tool.execute(args1).await;
        let result2 = tool.execute(args2).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let value1 = result1.unwrap();
        let value2 = result2.unwrap();

        let count1 = value1["count"].as_u64().unwrap_or(0);
        let count2 = value2["count"].as_u64().unwrap_or(0);

        assert_eq!(count1, 2);
        assert_eq!(count2, 1);
    }

    #[tokio::test]
    async fn test_ast_grep_search_with_globs() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "fn foo() {{}}").unwrap();
        writeln!(temp_file, "fn bar() {{}}").unwrap();
        temp_file.flush().unwrap();

        let args = serde_json::json!({
            "pattern": "fn $NAME() {}",
            "language": "rust",
            "path": temp_file.path(),
            "globs": ["*.rs"]
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["count"], 2);
    }

    #[tokio::test]
    async fn test_ast_grep_search_no_match() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "fn foo() {{}}").unwrap();
        temp_file.flush().unwrap();

        let args = serde_json::json!({
            "pattern": "fn bar() {}",
            "language": "rust",
            "path": temp_file.path()
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["count"], 0);
        assert_eq!(value["matches"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_ast_grep_replace_dry_run() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "var x = 1;").unwrap();
        writeln!(temp_file, "var y = 2;").unwrap();
        temp_file.flush().unwrap();
        let path = temp_file.path().to_path_buf();

        let args = serde_json::json!({
            "command": "replace",
            "pattern": "var $A = $B;",
            "rewrite": "let $A = $B;",
            "language": "javascript",
            "path": &path,
            "dry_run": true
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["count"], 2);
        assert_eq!(value["dry_run"], true);

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("var x = 1;"));
        assert!(content.contains("var y = 2;"));
    }

    #[tokio::test]
    async fn test_ast_grep_replace_writes_file() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "var x = 1;").unwrap();
        writeln!(temp_file, "var y = 2;").unwrap();
        temp_file.flush().unwrap();
        let path = temp_file.path().to_path_buf();

        let args = serde_json::json!({
            "command": "replace",
            "pattern": "var $A = $B;",
            "rewrite": "let $A = $B;",
            "language": "javascript",
            "path": &path,
            "dry_run": false
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["count"], 2);
        assert_eq!(value["dry_run"], false);

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("let x = 1;"));
        assert!(content.contains("let y = 2;"));
        assert!(!content.contains("var x = 1;"));
        assert!(!content.contains("var y = 2;"));
    }

    #[tokio::test]
    async fn test_ast_grep_language_case_insensitive() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "fn foo() {{}}").unwrap();
        temp_file.flush().unwrap();

        for lang in ["RUST", "Rust", "rust"] {
            let args = serde_json::json!({
                "pattern": "fn $NAME() {}",
                "language": lang,
                "path": temp_file.path()
            });

            let result = tool.execute(args).await;
            assert!(result.is_ok(), "Failed for language: {}", lang);
            let value = result.unwrap();
            assert_eq!(value["count"], 1);
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

    #[tokio::test]
    async fn test_ast_grep_tool_new_language_go() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::with_suffix(".go").unwrap();
        writeln!(temp_file, "func foo() {{}}").unwrap();
        writeln!(temp_file, "func bar() {{}}").unwrap();
        temp_file.flush().unwrap();

        let args = serde_json::json!({
            "pattern": "func $NAME($$$) { }",
            "language": "go",
            "path": temp_file.path()
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        let value = result.unwrap();
        assert_eq!(value["count"], 2);
        assert_eq!(value["command"], "search");
    }

    #[tokio::test]
    async fn test_ast_grep_tool_language_alias_js() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::with_suffix(".js").unwrap();
        writeln!(temp_file, "console.log('hello');").unwrap();
        writeln!(temp_file, "console.log('world');").unwrap();
        temp_file.flush().unwrap();

        let args = serde_json::json!({
            "pattern": "console.log($X)",
            "language": "js",
            "path": temp_file.path()
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        let value = result.unwrap();
        assert_eq!(value["count"], 2);
        assert_eq!(value["command"], "search");
    }

    #[tokio::test]
    async fn test_ast_grep_tool_language_alias_py() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::with_suffix(".py").unwrap();
        writeln!(temp_file, "def foo():").unwrap();
        writeln!(temp_file, "def bar():").unwrap();
        temp_file.flush().unwrap();

        let args = serde_json::json!({
            "pattern": "def $FUNC():",
            "language": "py",
            "path": temp_file.path()
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        let value = result.unwrap();
        assert_eq!(value["count"], 2);
        assert_eq!(value["command"], "search");
    }

    #[tokio::test]
    async fn test_ast_grep_tool_language_alias_rs() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::with_suffix(".rs").unwrap();
        writeln!(temp_file, "fn foo() {{}}").unwrap();
        writeln!(temp_file, "fn bar() {{}}").unwrap();
        temp_file.flush().unwrap();

        let args = serde_json::json!({
            "pattern": "fn $NAME() {}",
            "language": "rs",
            "path": temp_file.path()
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        let value = result.unwrap();
        assert_eq!(value["count"], 2);
        assert_eq!(value["command"], "search");
    }

    #[tokio::test]
    async fn test_ast_grep_tool_unsupported_command() {
        let tool = AstGrepTool::new();
        let args = serde_json::json!({
            "command": "test",
            "pattern": "fn main()",
            "language": "rust"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported command"));
    }

    #[tokio::test]
    async fn test_ast_grep_tool_replace_missing_rewrite() {
        let tool = AstGrepTool::new();
        let args = serde_json::json!({
            "command": "replace",
            "pattern": "console.log($X)",
            "language": "javascript"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ast_grep_tool_scan_path_not_found() {
        let tool = AstGrepTool::new();
        let args = serde_json::json!({
            "command": "scan",
            "path": "/nonexistent/path"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet supported"));
    }

    #[tokio::test]
    async fn test_ast_grep_tool_backward_compat_no_command() {
        let tool = AstGrepTool::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "fn foo() {{}}").unwrap();
        writeln!(temp_file, "fn bar() {{}}").unwrap();
        temp_file.flush().unwrap();

        let args = serde_json::json!({
            "pattern": "fn $NAME() {}",
            "language": "rust",
            "path": temp_file.path()
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        let value = result.unwrap();
        assert_eq!(value["command"], "search");
        assert_eq!(value["count"], 2);
    }
}
