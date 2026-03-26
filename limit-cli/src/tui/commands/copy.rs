//! Copy command - copies output to clipboard
//!
//! Provides the /copy command for copying the last AI output to system clipboard.

use super::{Command, CommandContext, CommandResult};
use crate::clipboard_text;
use crate::error::CliError;

/// Copy command - copies last output to clipboard
pub struct CopyCommand;

impl Command for CopyCommand {
    fn name(&self) -> &str {
        "copy"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["cp"]
    }

    fn description(&self) -> &str {
        "Copy the last AI output to clipboard"
    }

    fn usage(&self) -> Vec<&str> {
        vec!["/copy"]
    }

    fn execute(&self, _args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        // Get last AI output from messages
        let messages = ctx.messages.lock().unwrap();
        let last_assistant_message = messages.iter().rev().find_map(|msg| {
            if msg.role == limit_llm::Role::Assistant {
                msg.content.clone()
            } else {
                None
            }
        });

        drop(messages); // Release lock before clipboard operation

        match last_assistant_message {
            Some(text) => match clipboard_text::copy_text_to_clipboard(&text.to_text()) {
                Ok(()) => {
                    ctx.add_system_message("Copied latest output to clipboard.".to_string());
                }
                Err(err) => {
                    ctx.add_system_message(format!("Failed to copy to clipboard: {err}"));
                }
            },
            None => {
                ctx.add_system_message(
                    "`/copy` is unavailable before the first output.".to_string(),
                );
            }
        }

        Ok(CommandResult::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_command() {
        assert_eq!(CopyCommand.name(), "copy");
        assert!(CopyCommand.aliases().contains(&"cp"));
    }
}
