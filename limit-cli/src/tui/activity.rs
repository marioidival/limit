//! Activity message formatting for TUI
//!
//! Formats tool execution messages for display in the activity feed.

/// Format a tool activity message based on tool name and arguments
pub fn format_activity_message(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "file_read" => format_file_read(args),
        "file_write" => format_file_write(args),
        "file_edit" => format_file_edit(args),
        "bash" => format_bash(args),
        "git_status" => "Checking git status...".to_string(),
        "git_diff" => "Checking git diff...".to_string(),
        "git_log" => "Checking git log...".to_string(),
        "git_add" => "Staging files...".to_string(),
        "git_commit" => "Creating commit...".to_string(),
        "git_push" => "Pushing to remote...".to_string(),
        "git_pull" => "Pulling from remote...".to_string(),
        "git_clone" => format_git_clone(args),
        "grep" => format_grep(args),
        "ast_grep" => format_ast_grep(args),
        "lsp" => format_lsp(args),
        _ => format!("Executing {}...", tool_name),
    }
}

/// Format file read operation
fn format_file_read(args: &serde_json::Value) -> String {
    args.get("path")
        .and_then(|p| p.as_str())
        .map(|p| format!("Reading {}...", truncate_path(p, 150)))
        .unwrap_or_else(|| "Reading file...".to_string())
}

/// Format file write operation
fn format_file_write(args: &serde_json::Value) -> String {
    args.get("path")
        .and_then(|p| p.as_str())
        .map(|p| format!("Writing {}...", truncate_path(p, 150)))
        .unwrap_or_else(|| "Writing file...".to_string())
}

/// Format file edit operation
fn format_file_edit(args: &serde_json::Value) -> String {
    args.get("path")
        .and_then(|p| p.as_str())
        .map(|p| format!("Editing {}...", truncate_path(p, 150)))
        .unwrap_or_else(|| "Editing file...".to_string())
}

/// Format bash command
fn format_bash(args: &serde_json::Value) -> String {
    args.get("command")
        .and_then(|c| c.as_str())
        .map(|c| format!("Running {}...", truncate_command(c, 150)))
        .unwrap_or_else(|| "Executing command...".to_string())
}

/// Format git clone operation
fn format_git_clone(args: &serde_json::Value) -> String {
    args.get("url")
        .and_then(|u| u.as_str())
        .map(|u| format!("Cloning {}...", truncate_path(u, 150)))
        .unwrap_or_else(|| "Cloning repository...".to_string())
}

/// Format grep search
fn format_grep(args: &serde_json::Value) -> String {
    args.get("pattern")
        .and_then(|p| p.as_str())
        .map(|p| format!("Searching for '{}'...", truncate_command(p, 150)))
        .unwrap_or_else(|| "Searching...".to_string())
}

/// Format AST grep search
fn format_ast_grep(args: &serde_json::Value) -> String {
    args.get("pattern")
        .and_then(|p| p.as_str())
        .map(|p| format!("AST searching '{}'...", truncate_command(p, 150)))
        .unwrap_or_else(|| "AST searching...".to_string())
}

/// Format LSP operation
fn format_lsp(args: &serde_json::Value) -> String {
    args.get("command")
        .and_then(|c| c.as_str())
        .map(|c| format!("Running LSP {}...", c))
        .unwrap_or_else(|| "Running LSP...".to_string())
}

/// Truncate a path for display, showing the end
fn truncate_path(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("...{}", &s[s.len().saturating_sub(max_len - 3)..])
    }
}

/// Truncate a command for display, showing the beginning
fn truncate_command(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_file_read() {
        let args = json!({"path": "/some/long/path/to/file.txt"});
        let msg = format_activity_message("file_read", &args);
        assert!(msg.contains("Reading"));
    }

    #[test]
    fn test_format_bash() {
        let args = json!({"command": "echo hello"});
        let msg = format_activity_message("bash", &args);
        assert!(msg.contains("Running"));
    }

    #[test]
    fn test_format_unknown_tool() {
        let args = json!({});
        let msg = format_activity_message("unknown_tool", &args);
        assert_eq!(msg, "Executing unknown_tool...");
    }

    #[test]
    fn test_truncate_path() {
        assert_eq!(truncate_path("short", 10), "short");
        assert_eq!(
            truncate_path("very_long_path_to_file.txt", 14),
            "...to_file.txt"
        );
    }

    #[test]
    fn test_truncate_command() {
        assert_eq!(truncate_command("short", 10), "short");
        assert_eq!(truncate_command("very_long_command_here", 10), "very_lo...");
    }

    #[test]
    fn test_git_commands() {
        assert_eq!(
            format_activity_message("git_status", &json!({})),
            "Checking git status..."
        );
        assert_eq!(
            format_activity_message("git_diff", &json!({})),
            "Checking git diff..."
        );
        assert_eq!(
            format_activity_message("git_add", &json!({})),
            "Staging files..."
        );
        assert_eq!(
            format_activity_message("git_commit", &json!({})),
            "Creating commit..."
        );
    }

    #[test]
    fn test_file_operations() {
        let args = json!({"path": "/test.txt"});
        assert!(format_activity_message("file_read", &args).contains("Reading"));
        assert!(format_activity_message("file_write", &args).contains("Writing"));
        assert!(format_activity_message("file_edit", &args).contains("Editing"));
    }
}
