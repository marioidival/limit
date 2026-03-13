//! Session management commands
//!
//! Handles /session list, /session new, /session load

use super::{Command, CommandContext, CommandResult};
use crate::error::CliError;

/// Session command - manages sessions
pub struct SessionCommand {
    // No state needed - all context is in CommandContext
}

impl SessionCommand {
    pub fn new() -> Self {
        Self {}
    }

    fn handle_list(&self, ctx: &CommandContext) -> Result<CommandResult, CliError> {
        let session_manager = ctx.session_manager.lock().unwrap();
        let current_session_id = ctx.session_id.clone();

        match session_manager.list_sessions() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    ctx.add_system_message("No sessions found.".to_string());
                } else {
                    let mut output = vec!["Sessions (most recent first):".to_string()];
                    for (i, session) in sessions.iter().enumerate() {
                        let current = if session.id == current_session_id {
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

    fn handle_new(&self, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        // Save current session first
        let save_result = {
            let session_manager = ctx.session_manager.lock().unwrap();
            let messages = ctx.messages.lock().unwrap().clone();
            let input_tokens = *ctx.total_input_tokens.lock().unwrap();
            let output_tokens = *ctx.total_output_tokens.lock().unwrap();

            session_manager.save_session(&ctx.session_id, &messages, input_tokens, output_tokens)
        };

        if let Err(e) = save_result {
            tracing::error!("Failed to save current session: {}", e);
            ctx.add_system_message(format!("⚠ Warning: Failed to save current session: {}", e));
        }

        // Create new session
        let new_session_id = {
            let session_manager = ctx.session_manager.lock().map_err(|e| {
                CliError::ConfigError(format!("Failed to acquire session manager lock: {}", e))
            })?;

            session_manager
                .create_new_session()
                .map_err(|e| CliError::ConfigError(format!("Failed to create session: {}", e)))?
        };

        // Update session ID in context
        ctx.session_id = new_session_id.clone();

        // Clear messages and reset token counts
        ctx.messages.lock().unwrap().clear();
        *ctx.total_input_tokens.lock().unwrap() = 0;
        *ctx.total_output_tokens.lock().unwrap() = 0;

        // Clear chat view
        ctx.chat_view.lock().unwrap().clear();

        tracing::info!("Created new session: {}", new_session_id);

        // Add system message
        let session_short_id = if new_session_id.len() > 8 {
            &new_session_id[new_session_id.len().saturating_sub(8)..]
        } else {
            &new_session_id
        };
        ctx.add_system_message(format!("🆕 New session created: {}", session_short_id));

        Ok(CommandResult::NewSession)
    }

    fn handle_load(
        &self,
        session_id: &str,
        ctx: &mut CommandContext,
    ) -> Result<CommandResult, CliError> {
        tracing::info!("Session load command detected for session: {}", session_id);

        // Save current session first
        let save_result = {
            let session_manager = ctx.session_manager.lock().unwrap();
            let messages = ctx.messages.lock().unwrap().clone();
            let input_tokens = *ctx.total_input_tokens.lock().unwrap();
            let output_tokens = *ctx.total_output_tokens.lock().unwrap();

            session_manager.save_session(&ctx.session_id, &messages, input_tokens, output_tokens)
        };

        if let Err(e) = save_result {
            tracing::error!("Failed to save current session: {}", e);
            ctx.add_system_message(format!("⚠ Warning: Failed to save current session: {}", e));
        }

        // Find session ID from partial match
        let (full_session_id, session_info, messages) = {
            let session_manager = ctx.session_manager.lock().map_err(|e| {
                CliError::ConfigError(format!("Failed to acquire session manager lock: {}", e))
            })?;

            let sessions = session_manager
                .list_sessions()
                .map_err(|e| CliError::ConfigError(format!("Failed to list sessions: {}", e)))?;

            let matched_session = if session_id.len() >= 8 {
                // Try exact match first
                sessions
                    .iter()
                    .find(|s| s.id == session_id)
                    // Then try prefix match
                    .or_else(|| sessions.iter().find(|s| s.id.starts_with(session_id)))
            } else {
                // Try prefix match for short IDs
                sessions.iter().find(|s| s.id.starts_with(session_id))
            };

            match matched_session {
                Some(info) => {
                    let full_id = info.id.clone();
                    let msgs = session_manager.load_session(&full_id).map_err(|e| {
                        CliError::ConfigError(format!("Failed to load session {}: {}", full_id, e))
                    })?;
                    (full_id, info.clone(), msgs)
                }
                None => {
                    ctx.add_system_message(format!("❌ Session not found: {}", session_id));
                    return Ok(CommandResult::Continue);
                }
            }
        };

        // Update context with loaded session data
        ctx.session_id = full_session_id.clone();
        *ctx.total_input_tokens.lock().unwrap() = session_info.total_input_tokens;
        *ctx.total_output_tokens.lock().unwrap() = session_info.total_output_tokens;
        *ctx.messages.lock().unwrap() = messages.clone();

        // Clear chat view and reload messages
        {
            let mut chat = ctx.chat_view.lock().unwrap();
            chat.clear();

            for msg in &messages {
                match msg.role {
                    limit_llm::Role::User => {
                        let content = msg.content.as_deref().unwrap_or("");
                        let chat_msg = limit_tui::components::Message::user(content.to_string());
                        chat.add_message(chat_msg);
                    }
                    limit_llm::Role::Assistant => {
                        let content = msg.content.as_deref().unwrap_or("");
                        let chat_msg =
                            limit_tui::components::Message::assistant(content.to_string());
                        chat.add_message(chat_msg);
                    }
                    _ => {}
                }
            }
        }

        tracing::info!(
            "Loaded session: {} ({} messages)",
            full_session_id,
            messages.len()
        );

        // Add system message
        let session_short_id = if full_session_id.len() > 8 {
            &full_session_id[full_session_id.len().saturating_sub(8)..]
        } else {
            &full_session_id
        };
        ctx.add_system_message(format!(
            "📂 Loaded session: {} ({} messages, {} in tokens, {} out tokens)",
            session_short_id,
            messages.len(),
            session_info.total_input_tokens,
            session_info.total_output_tokens
        ));

        Ok(CommandResult::LoadSession(full_session_id))
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
        vec!["/session list", "/session new", "/session load <id>"]
    }

    fn execute(&self, args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        let args = args.trim();

        if args == "list" {
            self.handle_list(ctx)
        } else if args == "new" {
            self.handle_new(ctx)
        } else if args.starts_with("load ") {
            let session_id = args.strip_prefix("load ").unwrap();
            self.handle_load(session_id, ctx)
        } else {
            ctx.add_system_message(
                "Usage: /session list, /session new, /session load <id>".to_string(),
            );
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
