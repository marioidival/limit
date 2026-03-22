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
        "browser" => format_browser(args),
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

/// Format AST grep operation
fn format_ast_grep(args: &serde_json::Value) -> String {
    let command = args
        .get("command")
        .and_then(|c| c.as_str())
        .unwrap_or("search");

    match command {
        "replace" => {
            let pattern = args.get("pattern").and_then(|p| p.as_str()).unwrap_or("?");
            let rewrite = args.get("rewrite").and_then(|r| r.as_str()).unwrap_or("?");
            format!(
                "AST replacing '{}' → '{}'...",
                truncate_command(pattern, 80),
                truncate_command(rewrite, 80)
            )
        }
        "scan" => {
            let rule = args
                .get("rule")
                .and_then(|r| r.as_str())
                .map(|r| truncate_command(r, 100));
            let inline = args
                .get("inline_rules")
                .and_then(|r| r.as_str())
                .map(|r| truncate_command(r, 100));
            match (rule, inline) {
                (Some(r), _) => format!("AST scanning with rule '{}'...", r),
                (_, Some(i)) => format!("AST scanning with inline rule '{}'...", i),
                _ => "AST scanning...".to_string(),
            }
        }
        _ => args
            .get("pattern")
            .and_then(|p| p.as_str())
            .map(|p| format!("AST searching '{}'...", truncate_command(p, 150)))
            .unwrap_or_else(|| "AST searching...".to_string()),
    }
}

/// Format LSP operation
fn format_lsp(args: &serde_json::Value) -> String {
    args.get("command")
        .and_then(|c| c.as_str())
        .map(|c| format!("Running LSP {}...", c))
        .unwrap_or_else(|| "Running LSP...".to_string())
}

fn format_browser(args: &serde_json::Value) -> String {
    let action = args
        .get("action")
        .and_then(|a| a.as_str())
        .unwrap_or("unknown");

    let detail = match action {
        "open" => args
            .get("url")
            .and_then(|u| u.as_str())
            .map(|u| format!("opening {}", truncate_path(u, 80)))
            .unwrap_or_else(|| "opening URL".to_string()),
        "close" => "closing browser".to_string(),
        "snapshot" => "taking DOM snapshot".to_string(),
        "screenshot" => args
            .get("path")
            .and_then(|p| p.as_str())
            .map(|p| format!("taking screenshot to {}", truncate_path(p, 60)))
            .unwrap_or_else(|| "taking screenshot".to_string()),
        "back" => "navigating back".to_string(),
        "forward" => "navigating forward".to_string(),
        "reload" => "reloading page".to_string(),

        "click" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("clicking {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "clicking element".to_string()),
        "dblclick" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("double-clicking {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "double-clicking element".to_string()),
        "hover" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("hovering {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "hovering element".to_string()),
        "focus" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("focusing {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "focusing element".to_string()),
        "scrollintoview" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("scrolling to {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "scrolling element into view".to_string()),

        "fill" => {
            let selector = args.get("selector").and_then(|s| s.as_str());
            let text = args.get("text").and_then(|t| t.as_str());
            match (selector, text) {
                (Some(s), Some(t)) => format!(
                    "filling {} with \"{}\"",
                    truncate_command(s, 40),
                    truncate_command(t, 40)
                ),
                (Some(s), None) => format!("filling {}", truncate_command(s, 80)),
                _ => "filling input".to_string(),
            }
        }
        "type" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("typing into {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "typing".to_string()),
        "press" => args
            .get("key")
            .and_then(|k| k.as_str())
            .map(|k| format!("pressing {}", k))
            .unwrap_or_else(|| "pressing key".to_string()),
        "select" => {
            let selector = args.get("selector").and_then(|s| s.as_str());
            let value = args.get("value").and_then(|v| v.as_str());
            match (selector, value) {
                (Some(s), Some(v)) => format!(
                    "selecting \"{}\" in {}",
                    truncate_command(v, 40),
                    truncate_command(s, 40)
                ),
                (Some(s), None) => format!("selecting option in {}", truncate_command(s, 80)),
                _ => "selecting option".to_string(),
            }
        }
        "check" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("checking {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "checking checkbox".to_string()),
        "uncheck" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("unchecking {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "unchecking checkbox".to_string()),
        "drag" => {
            let source = args.get("source").and_then(|s| s.as_str());
            let target = args.get("target").and_then(|t| t.as_str());
            match (source, target) {
                (Some(s), Some(t)) => format!(
                    "dragging {} to {}",
                    truncate_command(s, 40),
                    truncate_command(t, 40)
                ),
                _ => "dragging element".to_string(),
            }
        }
        "upload" => {
            let selector = args.get("selector").and_then(|s| s.as_str());
            let path = args.get("path").and_then(|p| p.as_str());
            match (selector, path) {
                (Some(s), Some(p)) => format!(
                    "uploading {} to {}",
                    truncate_path(p, 40),
                    truncate_command(s, 40)
                ),
                _ => "uploading file".to_string(),
            }
        }
        "pdf" => args
            .get("path")
            .and_then(|p| p.as_str())
            .map(|p| format!("saving PDF to {}", truncate_path(p, 60)))
            .unwrap_or_else(|| "saving PDF".to_string()),

        "get" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("getting element {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "getting element".to_string()),
        "get_attr" => {
            let selector = args.get("selector").and_then(|s| s.as_str());
            let attr = args.get("attr").and_then(|a| a.as_str());
            match (selector, attr) {
                (Some(s), Some(a)) => {
                    format!("getting {} attribute from {}", a, truncate_command(s, 60))
                }
                _ => "getting attribute".to_string(),
            }
        }
        "get_count" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("counting {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "counting elements".to_string()),
        "get_box" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("getting bounding box of {}", truncate_command(s, 60)))
            .unwrap_or_else(|| "getting bounding box".to_string()),
        "get_styles" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("getting styles of {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "getting styles".to_string()),

        "wait" => args
            .get("ms")
            .and_then(|m| m.as_u64())
            .map(|m| format!("waiting {}ms", m))
            .unwrap_or_else(|| "waiting".to_string()),
        "wait_for_text" => args
            .get("text")
            .and_then(|t| t.as_str())
            .map(|t| format!("waiting for text \"{}\"", truncate_command(t, 60)))
            .unwrap_or_else(|| "waiting for text".to_string()),
        "wait_for_url" => args
            .get("url")
            .and_then(|u| u.as_str())
            .map(|u| format!("waiting for URL {}", truncate_path(u, 60)))
            .unwrap_or_else(|| "waiting for URL".to_string()),
        "wait_for_load" => "waiting for page load".to_string(),
        "wait_for_download" => "waiting for download".to_string(),
        "wait_for_fn" => "waiting for function result".to_string(),
        "wait_for_state" => args
            .get("state")
            .and_then(|s| s.as_str())
            .map(|s| format!("waiting for {} state", s))
            .unwrap_or_else(|| "waiting for state".to_string()),

        "find" => args
            .get("selector")
            .and_then(|s| s.as_str())
            .map(|s| format!("finding {}", truncate_command(s, 80)))
            .unwrap_or_else(|| "finding element".to_string()),
        "scroll" => {
            let x = args.get("x").and_then(|v| v.as_i64());
            let y = args.get("y").and_then(|v| v.as_i64());
            match (x, y) {
                (Some(x), Some(y)) => format!("scrolling to ({}, {})", x, y),
                _ => "scrolling".to_string(),
            }
        }
        "is" => {
            let selector = args.get("selector").and_then(|s| s.as_str());
            let state = args.get("state").and_then(|s| s.as_str());
            match (selector, state) {
                (Some(s), Some(st)) => {
                    format!("checking if {} is {}", truncate_command(s, 60), st)
                }
                _ => "checking element state".to_string(),
            }
        }
        "download" => args
            .get("url")
            .and_then(|u| u.as_str())
            .map(|u| format!("downloading {}", truncate_path(u, 60)))
            .unwrap_or_else(|| "downloading".to_string()),

        "tab_list" => "listing tabs".to_string(),
        "tab_new" => args
            .get("url")
            .and_then(|u| u.as_str())
            .map(|u| format!("opening new tab {}", truncate_path(u, 60)))
            .unwrap_or_else(|| "opening new tab".to_string()),
        "tab_close" => args
            .get("index")
            .and_then(|i| i.as_u64())
            .map(|i| format!("closing tab {}", i))
            .unwrap_or_else(|| "closing tab".to_string()),
        "tab_select" => args
            .get("index")
            .and_then(|i| i.as_u64())
            .map(|i| format!("selecting tab {}", i))
            .unwrap_or_else(|| "selecting tab".to_string()),

        "dialog_accept" => args
            .get("text")
            .and_then(|t| t.as_str())
            .map(|t| format!("accepting dialog with \"{}\"", truncate_command(t, 40)))
            .unwrap_or_else(|| "accepting dialog".to_string()),
        "dialog_dismiss" => "dismissing dialog".to_string(),

        "cookies" => args
            .get("url")
            .and_then(|u| u.as_str())
            .map(|u| format!("getting cookies for {}", truncate_path(u, 60)))
            .unwrap_or_else(|| "getting cookies".to_string()),
        "cookies_set" => "setting cookies".to_string(),
        "storage_get" => args
            .get("key")
            .and_then(|k| k.as_str())
            .map(|k| format!("getting storage \"{}\"", truncate_command(k, 40)))
            .unwrap_or_else(|| "getting from storage".to_string()),
        "storage_set" => args
            .get("key")
            .and_then(|k| k.as_str())
            .map(|k| format!("setting storage \"{}\"", truncate_command(k, 40)))
            .unwrap_or_else(|| "setting storage".to_string()),
        "network_requests" => "getting network requests".to_string(),

        "set_viewport" => {
            let width = args.get("width").and_then(|w| w.as_u64());
            let height = args.get("height").and_then(|h| h.as_u64());
            match (width, height) {
                (Some(w), Some(h)) => format!("setting viewport to {}x{}", w, h),
                _ => "setting viewport".to_string(),
            }
        }
        "set_device" => args
            .get("device")
            .and_then(|d| d.as_str())
            .map(|d| format!("setting device to {}", d))
            .unwrap_or_else(|| "setting device".to_string()),
        "set_geo" => {
            let lat = args.get("latitude").and_then(|l| l.as_f64());
            let lon = args.get("longitude").and_then(|l| l.as_f64());
            match (lat, lon) {
                (Some(lat), Some(lon)) => format!("setting geolocation to ({}, {})", lat, lon),
                _ => "setting geolocation".to_string(),
            }
        }

        "eval" => args
            .get("script")
            .and_then(|s| s.as_str())
            .map(|s| format!("evaluating {}", truncate_command(s, 60)))
            .unwrap_or_else(|| "evaluating script".to_string()),

        _ => action.to_string(),
    };

    format!("Executing browser: {}...", detail)
}

/// Truncate a path for display, showing the end
fn truncate_path(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let skip = char_count.saturating_sub(max_len - 3);
        let truncated: String = s.chars().skip(skip).collect();
        format!("...{truncated}")
    }
}

/// Truncate a command for display, showing the beginning
fn truncate_command(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{truncated}...")
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
    fn test_truncate_path_utf8() {
        let multi_byte = "════════════════════════════════════════════════";
        let result = truncate_path(multi_byte, 10);
        assert!(result.starts_with("..."));
        assert!(result.chars().count() <= 10);
    }

    #[test]
    fn test_truncate_command() {
        assert_eq!(truncate_command("short", 10), "short");
        assert_eq!(truncate_command("very_long_command_here", 10), "very_lo...");
    }

    #[test]
    fn test_truncate_command_utf8() {
        let multi_byte = "════════════════════════════════════════════════";
        let result = truncate_command(multi_byte, 10);
        assert!(result.ends_with("..."));
        assert!(result.chars().count() <= 10);
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

    #[test]
    fn test_browser_navigation() {
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "open", "url": "https://example.com"})
            ),
            "Executing browser: opening https://example.com..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "close"})),
            "Executing browser: closing browser..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "screenshot", "path": "/tmp/shot.png"})
            ),
            "Executing browser: taking screenshot to /tmp/shot.png..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "back"})),
            "Executing browser: navigating back..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "reload"})),
            "Executing browser: reloading page..."
        );
    }

    #[test]
    fn test_browser_interaction() {
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "click", "selector": "#submit"})
            ),
            "Executing browser: clicking #submit..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "fill", "selector": "#email", "text": "test@example.com"})
            ),
            "Executing browser: filling #email with \"test@example.com\"..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "press", "key": "Enter"})),
            "Executing browser: pressing Enter..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "hover", "selector": ".menu"})),
            "Executing browser: hovering .menu..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "check", "selector": "#terms"})),
            "Executing browser: checking #terms..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "uncheck", "selector": "#newsletter"})
            ),
            "Executing browser: unchecking #newsletter..."
        );
    }

    #[test]
    fn test_browser_wait() {
        assert_eq!(
            format_activity_message("browser", &json!({"action": "wait", "ms": 1000})),
            "Executing browser: waiting 1000ms..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "wait_for_text", "text": "Login"})
            ),
            "Executing browser: waiting for text \"Login\"..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "wait_for_url", "url": "/dashboard"})
            ),
            "Executing browser: waiting for URL /dashboard..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "wait_for_load"})),
            "Executing browser: waiting for page load..."
        );
    }

    #[test]
    fn test_browser_tabs() {
        assert_eq!(
            format_activity_message("browser", &json!({"action": "tab_list"})),
            "Executing browser: listing tabs..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "tab_new", "url": "https://example.com"})
            ),
            "Executing browser: opening new tab https://example.com..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "tab_select", "index": 2})),
            "Executing browser: selecting tab 2..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "tab_close", "index": 1})),
            "Executing browser: closing tab 1..."
        );
    }

    #[test]
    fn test_browser_query() {
        assert_eq!(
            format_activity_message("browser", &json!({"action": "get", "selector": ".item"})),
            "Executing browser: getting element .item..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "get_attr", "selector": "a", "attr": "href"})
            ),
            "Executing browser: getting href attribute from a..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "get_count", "selector": ".items"})
            ),
            "Executing browser: counting .items..."
        );
    }

    #[test]
    fn test_browser_settings() {
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "set_viewport", "width": 1920, "height": 1080})
            ),
            "Executing browser: setting viewport to 1920x1080..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "set_device", "device": "iPhone 12"})
            ),
            "Executing browser: setting device to iPhone 12..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "set_geo", "latitude": 37.7749, "longitude": -122.4194})
            ),
            "Executing browser: setting geolocation to (37.7749, -122.4194)..."
        );
    }

    #[test]
    fn test_browser_dialog() {
        assert_eq!(
            format_activity_message("browser", &json!({"action": "dialog_accept", "text": "OK"})),
            "Executing browser: accepting dialog with \"OK\"..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "dialog_dismiss"})),
            "Executing browser: dismissing dialog..."
        );
    }

    #[test]
    fn test_browser_storage() {
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "cookies", "url": "https://example.com"})
            ),
            "Executing browser: getting cookies for https://example.com..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "storage_get", "key": "token"})),
            "Executing browser: getting storage \"token\"..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "storage_set", "key": "session"})
            ),
            "Executing browser: setting storage \"session\"..."
        );
    }

    #[test]
    fn test_browser_eval() {
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "eval", "script": "return 1 + 1"})
            ),
            "Executing browser: evaluating return 1 + 1..."
        );
    }

    #[test]
    fn test_browser_unknown_action() {
        assert_eq!(
            format_activity_message("browser", &json!({"action": "unknown_action"})),
            "Executing browser: unknown_action..."
        );
    }

    #[test]
    fn test_browser_missing_args() {
        assert_eq!(
            format_activity_message("browser", &json!({"action": "open"})),
            "Executing browser: opening URL..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "click"})),
            "Executing browser: clicking element..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "wait"})),
            "Executing browser: waiting..."
        );
    }

    #[test]
    fn test_browser_navigation_extras() {
        assert_eq!(
            format_activity_message("browser", &json!({"action": "snapshot"})),
            "Executing browser: taking DOM snapshot..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "forward"})),
            "Executing browser: navigating forward..."
        );
    }

    #[test]
    fn test_browser_interaction_extras() {
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "dblclick", "selector": "#btn"})
            ),
            "Executing browser: double-clicking #btn..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "focus", "selector": "#input"})),
            "Executing browser: focusing #input..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "scrollintoview", "selector": "#footer"})
            ),
            "Executing browser: scrolling to #footer..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "type", "selector": "#search"})),
            "Executing browser: typing into #search..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "select", "selector": "#country", "value": "BR"})
            ),
            "Executing browser: selecting \"BR\" in #country..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "drag", "source": "#item", "target": "#dropzone"})
            ),
            "Executing browser: dragging #item to #dropzone..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "upload", "selector": "#file", "path": "/tmp/file.txt"})
            ),
            "Executing browser: uploading /tmp/file.txt to #file..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "pdf", "path": "/tmp/page.pdf"})
            ),
            "Executing browser: saving PDF to /tmp/page.pdf..."
        );
    }

    #[test]
    fn test_browser_query_extras() {
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "get_box", "selector": "#modal"})
            ),
            "Executing browser: getting bounding box of #modal..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "get_styles", "selector": ".button"})
            ),
            "Executing browser: getting styles of .button..."
        );
    }

    #[test]
    fn test_browser_wait_extras() {
        assert_eq!(
            format_activity_message("browser", &json!({"action": "wait_for_download"})),
            "Executing browser: waiting for download..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "wait_for_fn"})),
            "Executing browser: waiting for function result..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "wait_for_state", "state": "visible"})
            ),
            "Executing browser: waiting for visible state..."
        );
    }

    #[test]
    fn test_browser_state() {
        assert_eq!(
            format_activity_message("browser", &json!({"action": "find", "selector": ".item"})),
            "Executing browser: finding .item..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "scroll", "x": 0, "y": 500})),
            "Executing browser: scrolling to (0, 500)..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "is", "selector": "#btn", "state": "visible"})
            ),
            "Executing browser: checking if #btn is visible..."
        );
        assert_eq!(
            format_activity_message(
                "browser",
                &json!({"action": "download", "url": "https://example.com/file.zip"})
            ),
            "Executing browser: downloading https://example.com/file.zip..."
        );
    }

    #[test]
    fn test_browser_storage_network() {
        assert_eq!(
            format_activity_message("browser", &json!({"action": "cookies_set"})),
            "Executing browser: setting cookies..."
        );
        assert_eq!(
            format_activity_message("browser", &json!({"action": "network_requests"})),
            "Executing browser: getting network requests..."
        );
    }
}
