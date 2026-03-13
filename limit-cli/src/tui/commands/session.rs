//! Session management commands
//!
//! Handles /session list, /session new, /session load

use super::{Command, CommandContext, CommandResult};
use crate::error::CliError;
use limit_tui::components::Message;

/// Session command - manages sessions
pub struct SessionCommand {
    // No state needed - all context is in CommandContext
}

impl SessionCommand {
    pub fn new() -> Self {
        Self {}
    }

    fn handle_list(&self, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        let session_manager = ctx.session_manager.lock().unwrap();
        
        match session_manager.list_sessions() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    ctx.add_system_message("No sessions found.".to_string());
                } else {
                    let mut output = vec!["Sessions (most recent first):".to_string()];
                    for (i, session) in sessions.iter().enumerate() {
                        let current = if session.id == ctx.session_id {
                            " (current)"
                        } else {
                            ""
                        };
                        let short_id = if session.id.len() > 8 {
                            &session.id[..8]
                        } else {
                            &session.id
                        };
                        output.push(format!(
                            "  {}. {}{} - {} messages, {} in tokens, {} out tokens",
                            i + 1,
                            short_id,
                            current,
                            session.message_count,
                            session.total_input_tokens,
                            session.total_output_tokens
                        ));
                    }
                    ctx.add_system_message(output.join("\n"));
                }
            }
            Err(e) => {
                ctx.add_system_message(format!("Error listing sessions: {}", e));
            }
        }
        
        Ok(CommandResult::Continue)
    }
}

impl Command for SessionCommand {
    fn name(&self) -> &str {
        "session"
    }

    fn description(&self) -> &str {
        "Manage conversation sessions"
    }

    fn usage(&self) -> Vec<&str> {
        vec![
            "/session list",
            "/session new",
            "/session load <id>",
        ]
    }

    fn execute(&self, args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        let args = args.trim();
        
        if args == "list" {
            self.handle_list(ctx)
        } else if args == "new" {
            // TODO: Implement session creation (requires more context)
            ctx.add_system_message("Session creation not yet implemented in command system".to_string());
            Ok(CommandResult::Continue)
        } else if args.starts_with("load ") {
            // TODO: Implement session loading (requires more context)
            let session_id = args.strip_prefix("load ").unwrap();
            ctx.add_system_message(format!("Session loading not yet implemented: {}", session_id));
            Ok(CommandResult::Continue)
        } else {
            ctx.add_system_message("Usage: /session list, /session new, /session load <id>".to_string());
            Ok(CommandResult::Continue)
        }
    }
}

impl Default for SessionCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_command() {
        let cmd = SessionCommand::new();
        assert_eq!(cmd.name(), "session");
    }
}
