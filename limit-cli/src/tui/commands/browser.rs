//! Browser command for TUI
//!
//! Provides `/browser` command for browser automation in the TUI.

use super::{Command, CommandContext, CommandResult};
use crate::error::CliError;
use crate::tools::browser::BrowserClient;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Browser automation command
pub struct BrowserCommand {
    client: Arc<Mutex<BrowserClient>>,
}

impl BrowserCommand {
    /// Create a new browser command
    pub fn new() -> Self {
        let client = BrowserClient::with_default_config();
        Self {
            client: Arc::new(Mutex::new(client)),
        }
    }

    /// Create a browser command with custom config
    #[allow(dead_code)]
    pub fn with_config(_config: crate::tools::browser::BrowserConfig) -> Self {
        let client = BrowserClient::with_default_config();
        Self {
            client: Arc::new(Mutex::new(client)),
        }
    }

    /// Show help for the browser command
    fn show_help(&self, ctx: &mut CommandContext) -> CommandResult {
        let help_text = r#"Browser automation commands:

  /browser open <url>        Open a URL in the browser
  /browser close             Close the browser
  /browser snapshot          Take an accessibility snapshot
  /browser click <selector>  Click an element
  /browser fill <sel> <text> Fill a form field
  /browser screenshot <path> Save a screenshot
  /browser get <what>        Get page content (text, html, url, title)
  /browser help              Show this help

Examples:
  /browser open https://example.com
  /browser snapshot
  /browser click "button.submit"
  /browser fill "input[name=email]" "test@example.com"
  /browser screenshot /tmp/page.png
  /browser get title"#;

        ctx.add_system_message(help_text.to_string());
        CommandResult::Continue
    }

    /// Parse arguments from the input string
    fn parse_args(&self, input: &str) -> Vec<String> {
        // Simple argument parsing - handle quoted strings
        let mut args = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';

        for ch in input.chars() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                }
                c if c == quote_char && in_quotes => {
                    in_quotes = false;
                    quote_char = ' ';
                }
                ' ' if !in_quotes => {
                    if !current.is_empty() {
                        args.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            args.push(current);
        }

        args
    }
}

impl Default for BrowserCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl Command for BrowserCommand {
    fn name(&self) -> &str {
        "browser"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["b"]
    }

    fn description(&self) -> &str {
        "Browser automation for testing, scraping, and screenshots"
    }

    fn usage(&self) -> Vec<&str> {
        vec![
            "/browser open <url>",
            "/browser close",
            "/browser snapshot",
            "/browser click <selector>",
            "/browser fill <selector> <text>",
            "/browser screenshot <path>",
            "/browser get <text|html|url|title>",
        ]
    }

    fn execute(&self, args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        let args = args.trim();

        // Show help if no args or explicit help request
        if args.is_empty() || args == "help" {
            return Ok(self.show_help(ctx));
        }

        let parsed = self.parse_args(args);
        if parsed.is_empty() {
            return Ok(self.show_help(ctx));
        }

        let action = parsed[0].to_lowercase();
        let client = self.client.clone();

        // Create a tokio runtime for async operations
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| CliError::Other(format!("Failed to create runtime: {}", e)))?;

        let result = rt.block_on(async {
            let client = client.lock().await;

            match action.as_str() {
                "open" => {
                    if parsed.len() < 2 {
                        return Err(CliError::Other("Usage: /browser open <url>".to_string()));
                    }
                    let url = &parsed[1];
                    client
                        .open(url)
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to open URL: {}", e)))?;
                    ctx.add_system_message(format!("Opened: {}", url));
                    Ok(CommandResult::Continue)
                }

                "close" => {
                    client
                        .close()
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to close browser: {}", e)))?;
                    ctx.add_system_message("Browser closed".to_string());
                    Ok(CommandResult::Continue)
                }

                "snapshot" => {
                    let result = client
                        .snapshot()
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to take snapshot: {}", e)))?;

                    let mut msg = String::new();
                    if let Some(title) = &result.title {
                        msg.push_str(&format!("Title: {}\n", title));
                    }
                    if let Some(url) = &result.url {
                        msg.push_str(&format!("URL: {}\n", url));
                    }
                    msg.push_str("\n--- Snapshot ---\n");
                    msg.push_str(&result.content);

                    ctx.add_system_message(msg);
                    Ok(CommandResult::Continue)
                }

                "click" => {
                    if parsed.len() < 2 {
                        return Err(CliError::Other(
                            "Usage: /browser click <selector>".to_string(),
                        ));
                    }
                    let selector = &parsed[1];
                    client
                        .click(selector)
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to click: {}", e)))?;
                    ctx.add_system_message(format!("Clicked: {}", selector));
                    Ok(CommandResult::Continue)
                }

                "fill" => {
                    if parsed.len() < 3 {
                        return Err(CliError::Other(
                            "Usage: /browser fill <selector> <text>".to_string(),
                        ));
                    }
                    let selector = &parsed[1];
                    let text = &parsed[2];
                    client
                        .fill(selector, text)
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to fill: {}", e)))?;
                    ctx.add_system_message(format!("Filled {} with text", selector));
                    Ok(CommandResult::Continue)
                }

                "screenshot" => {
                    if parsed.len() < 2 {
                        return Err(CliError::Other(
                            "Usage: /browser screenshot <path>".to_string(),
                        ));
                    }
                    let path = &parsed[1];
                    client.screenshot(path).await.map_err(|e| {
                        CliError::Other(format!("Failed to take screenshot: {}", e))
                    })?;
                    ctx.add_system_message(format!("Screenshot saved to: {}", path));
                    Ok(CommandResult::Continue)
                }

                "get" => {
                    if parsed.len() < 2 {
                        return Err(CliError::Other(
                            "Usage: /browser get <text|html|url|title>".to_string(),
                        ));
                    }
                    let what = &parsed[1];
                    let content = client
                        .get(what)
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to get {}: {}", what, e)))?;
                    ctx.add_system_message(format!("{}: {}", what, content));
                    Ok(CommandResult::Continue)
                }

                _ => {
                    ctx.add_system_message(format!("Unknown browser action: {}", action));
                    ctx.add_system_message("Type /browser help for available commands".to_string());
                    Ok(CommandResult::Continue)
                }
            }
        });

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_command_name() {
        let cmd = BrowserCommand::new();
        assert_eq!(cmd.name(), "browser");
    }

    #[test]
    fn test_browser_command_aliases() {
        let cmd = BrowserCommand::new();
        assert_eq!(cmd.aliases(), vec!["b"]);
    }

    #[test]
    fn test_browser_command_description() {
        let cmd = BrowserCommand::new();
        assert!(!cmd.description().is_empty());
    }

    #[test]
    fn test_parse_args_simple() {
        let cmd = BrowserCommand::new();
        let args = cmd.parse_args("open https://example.com");
        assert_eq!(args, vec!["open", "https://example.com"]);
    }

    #[test]
    fn test_parse_args_quoted() {
        let cmd = BrowserCommand::new();
        let args = cmd.parse_args("fill 'input[name=email]' \"test text\"");
        assert_eq!(args, vec!["fill", "input[name=email]", "test text"]);
    }

    #[test]
    fn test_parse_args_empty() {
        let cmd = BrowserCommand::new();
        let args = cmd.parse_args("");
        assert!(args.is_empty());
    }
}
