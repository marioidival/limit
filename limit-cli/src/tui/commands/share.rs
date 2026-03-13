//! Share command - export sessions
//!
//! Handles /share clipboard/md/json

use super::{Command, CommandContext, CommandResult};
use crate::error::CliError;
use crate::session_share::{ExportFormat, SessionShare};

/// Share command - exports session to clipboard or file
pub struct ShareCommand;

impl ShareCommand {
    pub fn new() -> Self {
        Self
    }

    fn get_format(&self, args: &str) -> Option<ExportFormat> {
        match args.trim().to_lowercase().as_str() {
            "" | "clipboard" | "cb" => Some(ExportFormat::Markdown),
            "md" | "markdown" => Some(ExportFormat::Markdown),
            "json" => Some(ExportFormat::Json),
            _ => None,
        }
    }

    fn handle_clipboard(
        &self,
        ctx: &mut CommandContext,
        format: ExportFormat,
        session_id: &str,
    ) -> Result<CommandResult, CliError> {
        let messages = ctx.messages.lock().unwrap().clone();
        let total_input_tokens = *ctx.total_input_tokens.lock().unwrap();
        let total_output_tokens = *ctx.total_output_tokens.lock().unwrap();

        // Check if there are messages to share
        let user_assistant_count = messages
            .iter()
            .filter(|m| matches!(m.role, limit_llm::Role::User | limit_llm::Role::Assistant))
            .count();

        if user_assistant_count == 0 {
            ctx.add_system_message("⚠ No messages to share. Start a conversation first.".to_string());
            return Ok(CommandResult::Continue);
        }

        match SessionShare::generate_share_content(
            session_id,
            &messages,
            total_input_tokens,
            total_output_tokens,
            None, // model - not available in context
            format,
        ) {
            Ok(content) => {
                if let Some(ref clipboard) = ctx.clipboard {
                    match clipboard.lock().unwrap().set_text(&content) {
                        Ok(()) => {
                            let short_id = &session_id[..session_id.len().min(8)];
                            ctx.add_system_message(format!(
                                "✓ Session {} copied to clipboard ({} messages, {} tokens)",
                                short_id,
                                user_assistant_count,
                                total_input_tokens + total_output_tokens
                            ));
                        }
                        Err(e) => {
                            ctx.add_system_message(format!(
                                "❌ Failed to copy to clipboard: {}",
                                e
                            ));
                        }
                    }
                } else {
                    ctx.add_system_message(
                        "❌ Clipboard not available. Try '/share md' to save as file."
                            .to_string(),
                    );
                }
            }
            Err(e) => {
                ctx.add_system_message(format!("❌ Failed to generate share content: {}", e));
            }
        }

        Ok(CommandResult::Continue)
    }

    fn handle_file_export(
        &self,
        ctx: &mut CommandContext,
        format: ExportFormat,
        session_id: &str,
    ) -> Result<CommandResult, CliError> {
        let messages = ctx.messages.lock().unwrap().clone();
        let total_input_tokens = *ctx.total_input_tokens.lock().unwrap();
        let total_output_tokens = *ctx.total_output_tokens.lock().unwrap();

        // Check if there are messages to share
        let user_assistant_count = messages
            .iter()
            .filter(|m| matches!(m.role, limit_llm::Role::User | limit_llm::Role::Assistant))
            .count();

        if user_assistant_count == 0 {
            ctx.add_system_message("⚠ No messages to share. Start a conversation first.".to_string());
            return Ok(CommandResult::Continue);
        }

        match SessionShare::export_session(
            session_id,
            &messages,
            total_input_tokens,
            total_output_tokens,
            None, // model - not available in context
            format,
        ) {
            Ok((filepath, export)) => {
                let short_id = &session_id[..session_id.len().min(8)];
                let extension = match format {
                    ExportFormat::Markdown => "md",
                    ExportFormat::Json => "json",
                };
                ctx.add_system_message(format!(
                    "✓ Session {} exported to {}\n  ({} messages, {} tokens)\n  Location: ~/.limit/exports/",
                    short_id,
                    extension,
                    user_assistant_count,
                    total_input_tokens + total_output_tokens
                ));

                tracing::info!(
                    "Session exported to {:?} ({} messages)",
                    filepath,
                    export.messages.len()
                );
            }
            Err(e) => {
                ctx.add_system_message(format!("❌ Failed to export session: {}", e));
            }
        }

        Ok(CommandResult::Continue)
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
        vec!["/share", "/share md", "/share json"]
    }

    fn execute(&self, args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        let format = match self.get_format(args) {
            Some(f) => f,
            None => {
                ctx.add_system_message(
                    "Invalid format. Use: /share, /share md, /share json".to_string(),
                );
                return Ok(CommandResult::Continue);
            }
        };

        let session_id = ctx.session_id.clone();
        let args_lower = args.trim().to_lowercase();

        // If clipboard mode (empty, "clipboard", or "cb")
        if args_lower.is_empty() || args_lower == "clipboard" || args_lower == "cb" {
            self.handle_clipboard(ctx, format, &session_id)
        } else {
            // File export mode
            self.handle_file_export(ctx, format, &session_id)
        }
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
