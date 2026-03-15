//! Browser command for TUI
//!
//! Provides `/browser` command for browser automation in the TUI.

use super::{Command, CommandContext, CommandResult};
use crate::error::CliError;
use crate::tools::browser::client_ext::{
    InteractionExt, NavigationExt, QueryExt, StorageExt, TabsExt, WaitingExt,
};
use crate::tools::browser::{BrowserClient, BrowserConfig};
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
    pub fn with_config(config: BrowserConfig) -> Self {
        use crate::tools::browser::executor::CliExecutor;
        let executor = Arc::new(CliExecutor::new(config));
        let client = BrowserClient::new(executor);
        Self {
            client: Arc::new(Mutex::new(client)),
        }
    }

    /// Show help for the browser command
    fn show_help(&self, ctx: &mut CommandContext) -> CommandResult {
        let help_text = r#"Browser automation commands:

Navigation:
  /browser open <url>        Open a URL in the browser
  /browser back              Navigate back in history
  /browser forward           Navigate forward in history
  /browser reload            Reload the current page

Page Interaction:
  /browser snapshot          Take an accessibility snapshot
  /browser click <selector>  Click an element
  /browser fill <sel> <text> Fill a form field (instant)
  /browser type <sel> <text> Type text character by character
  /browser hover <selector>  Hover over an element
  /browser select <sel> <val> Select option in dropdown
  /browser press <key>       Press a keyboard key

State & Info:
  /browser screenshot <path> Save a screenshot
  /browser get <what>        Get page content (text, html, url, title)
  /browser scroll <dir> [px] Scroll page (up/down/left/right)
  /browser is <what> <sel>   Check element state (visible, enabled, etc.)
  /browser close             Close the browser
  /browser help              Show this help

Examples:
  /browser open https://example.com
  /browser snapshot
  /browser click "button.submit"
  /browser fill "input[name=email]" "test@example.com"
  /browser type "input[name=password]" "secret"
  /browser hover "@e5"
  /browser press Enter
  /browser scroll down 100
  /browser is visible "@e3"
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
            "/browser type <selector> <text>",
            "/browser hover <selector>",
            "/browser press <key>",
            "/browser select <selector> <value>",
            "/browser screenshot <path>",
            "/browser get <text|html|url|title>",
            "/browser back",
            "/browser forward",
            "/browser reload",
            "/browser scroll <up|down|left|right> [pixels]",
            "/browser is <visible|hidden|enabled|disabled|editable> <selector>",
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

                // Navigation commands
                "back" => {
                    client
                        .back()
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to navigate back: {}", e)))?;
                    ctx.add_system_message("Navigated back".to_string());
                    Ok(CommandResult::Continue)
                }

                "forward" => {
                    client
                        .forward()
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to navigate forward: {}", e)))?;
                    ctx.add_system_message("Navigated forward".to_string());
                    Ok(CommandResult::Continue)
                }

                "reload" => {
                    client
                        .reload()
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to reload: {}", e)))?;
                    ctx.add_system_message("Page reloaded".to_string());
                    Ok(CommandResult::Continue)
                }

                // Input commands
                "type" => {
                    if parsed.len() < 3 {
                        return Err(CliError::Other(
                            "Usage: /browser type <selector> <text>".to_string(),
                        ));
                    }
                    let selector = &parsed[1];
                    let text = &parsed[2];
                    client
                        .type_text(selector, text)
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to type: {}", e)))?;
                    ctx.add_system_message(format!("Typed text into {}", selector));
                    Ok(CommandResult::Continue)
                }

                "press" => {
                    if parsed.len() < 2 {
                        return Err(CliError::Other("Usage: /browser press <key>".to_string()));
                    }
                    let key = &parsed[1];
                    client
                        .press(key)
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to press key: {}", e)))?;
                    ctx.add_system_message(format!("Pressed: {}", key));
                    Ok(CommandResult::Continue)
                }

                "hover" => {
                    if parsed.len() < 2 {
                        return Err(CliError::Other(
                            "Usage: /browser hover <selector>".to_string(),
                        ));
                    }
                    let selector = &parsed[1];
                    client
                        .hover(selector)
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to hover: {}", e)))?;
                    ctx.add_system_message(format!("Hovered over: {}", selector));
                    Ok(CommandResult::Continue)
                }

                "select" => {
                    if parsed.len() < 3 {
                        return Err(CliError::Other(
                            "Usage: /browser select <selector> <value>".to_string(),
                        ));
                    }
                    let selector = &parsed[1];
                    let value = &parsed[2];
                    client
                        .select_option(selector, value)
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to select: {}", e)))?;
                    ctx.add_system_message(format!("Selected '{}' in {}", value, selector));
                    Ok(CommandResult::Continue)
                }

                // State commands
                "scroll" => {
                    if parsed.len() < 2 {
                        return Err(CliError::Other(
                            "Usage: /browser scroll <up|down|left|right> [pixels]".to_string(),
                        ));
                    }
                    let direction = &parsed[1];
                    let pixels = parsed.get(2).and_then(|s| s.parse::<u32>().ok());
                    client
                        .scroll(direction, pixels)
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to scroll: {}", e)))?;
                    ctx.add_system_message(format!("Scrolled {}", direction));
                    Ok(CommandResult::Continue)
                }

                "is" => {
                    if parsed.len() < 3 {
                        return Err(CliError::Other(
                            "Usage: /browser is <visible|hidden|enabled|disabled|editable> <selector>".to_string(),
                        ));
                    }
                    let what = &parsed[1];
                    let selector = &parsed[2];
                    let result = client
                        .is_(what, selector)
                        .await
                        .map_err(|e| CliError::Other(format!("Failed to check state: {}", e)))?;
                    ctx.add_system_message(format!("Element {} is {}: {}", selector, what, result));
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
