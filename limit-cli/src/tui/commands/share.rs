//! Share command - export sessions
//!
//! Handles /share clipboard/md/json

use super::{Command, CommandContext, CommandResult};
use crate::clipboard::ClipboardManager;
use crate::error::CliError;
use crate::session_share::ExportFormat;

/// Share command - exports session to clipboard or file
pub struct ShareCommand {
    clipboard: Option<ClipboardManager>,
}

impl ShareCommand {
    pub fn new() -> Self {
        let clipboard = match ClipboardManager::new() {
            Ok(cb) => Some(cb),
            Err(_) => None,
        };
        Self { clipboard }
    }

    fn get_format(&self, args: &str) -> Option<ExportFormat> {
        match args.trim().to_lowercase().as_str() {
            "" | "clipboard" | "cb" => Some(ExportFormat::Markdown),
            "md" | "markdown" => Some(ExportFormat::Markdown),
            "json" => Some(ExportFormat::Json),
            _ => None,
        }
    }
}

impl Command for ShareCommand {
    fn name(&self) -> &str {
        "share"
    }

    fn description(&self) -> &str {
        "Export session to clipboard or file"
    }

    fn usage(&self) -> Vec<&str> {
        vec![
            "/share",
            "/share md",
            "/share json",
        ]
    }

    fn execute(&self, args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        let format = match self.get_format(args) {
            Some(f) => f,
            None => {
                ctx.add_system_message(
                    "Invalid format. Use: /share, /share md, /share json".to_string()
                );
                return Ok(CommandResult::Continue);
            }
        };

        // For now, just show a message that this is being refactored
        // Full implementation requires access to messages and tokens from TuiBridge
        ctx.add_system_message(
            "Share command is being refactored. Full functionality coming soon.".to_string()
        );
        ctx.add_system_message(
            format!("Requested format: {:?}", format)
        );

        Ok(CommandResult::Continue)
    }
}

impl Default for ShareCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_command() {
        let cmd = ShareCommand::new();
        assert_eq!(cmd.name(), "share");
    }

    #[test]
    fn test_share_default() {
        let cmd = ShareCommand::default();
        assert_eq!(cmd.name(), "share");
    }

    #[test]
    fn test_get_format() {
        let cmd = ShareCommand::new();
        
        assert!(matches!(cmd.get_format(""), Some(ExportFormat::Markdown)));
        assert!(matches!(cmd.get_format("md"), Some(ExportFormat::Markdown)));
        assert!(matches!(cmd.get_format("json"), Some(ExportFormat::Json)));
        assert!(cmd.get_format("invalid").is_none());
    }
}
