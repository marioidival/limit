//! Built-in commands (help, clear, exit)
//!
//! Simple commands that don't require complex state management.

use super::{Command, CommandContext, CommandResult};
use crate::error::CliError;

/// Help command - displays available commands
pub struct HelpCommand;

impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["?", "h"]
    }

    fn description(&self) -> &str {
        "Show available commands and usage information"
    }

    fn usage(&self) -> Vec<&str> {
        vec!["/help", "/help <command>"]
    }

    fn execute(&self, args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        if args.is_empty() {
            // General help
            let help_text = "Available commands:\n\
                 /help  - Show this help message\n\
                 /clear - Clear chat history\n\
                 /exit  - Exit the application\n\
                 /quit  - Exit the application\n\
                 /tldr  - Enable code analysis for this project\n\
                 /session list  - List all sessions\n\
                 /session new   - Create a new session\n\
                 /session load  <id> - Load a session by ID\n\
                 /share         - Copy session to clipboard (markdown)\n\
                 /share md      - Export session as markdown file\n\
                 /share json    - Export session as JSON file\n\
                 \n\
                 Page Up/Down - Scroll chat history";
            ctx.add_system_message(help_text.to_string());
        } else {
            // Help for specific command (future enhancement)
            ctx.add_system_message(format!("Help for command: {}", args));
        }

        Ok(CommandResult::Continue)
    }
}

/// Clear command - clears the chat view
pub struct ClearCommand;

impl Command for ClearCommand {
    fn name(&self) -> &str {
        "clear"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["cls"]
    }

    fn description(&self) -> &str {
        "Clear the chat history"
    }

    fn usage(&self) -> Vec<&str> {
        vec!["/clear"]
    }

    fn execute(&self, _args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        ctx.clear_chat();
        ctx.add_system_message("Chat cleared".to_string());
        Ok(CommandResult::ClearChat)
    }
}

/// Exit command - exits the application
pub struct ExitCommand;

impl Command for ExitCommand {
    fn name(&self) -> &str {
        "exit"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["quit", "q"]
    }

    fn description(&self) -> &str {
        "Exit the application"
    }

    fn usage(&self) -> Vec<&str> {
        vec!["/exit", "/quit"]
    }

    fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        Ok(CommandResult::Exit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_command() {
        assert_eq!(HelpCommand.name(), "help");
        assert!(HelpCommand.aliases().contains(&"?"));
    }

    #[test]
    fn test_clear_command() {
        assert_eq!(ClearCommand.name(), "clear");
        assert!(ClearCommand.aliases().contains(&"cls"));
    }

    #[test]
    fn test_exit_command() {
        assert_eq!(ExitCommand.name(), "exit");
        assert!(ExitCommand.aliases().contains(&"quit"));
    }
}
