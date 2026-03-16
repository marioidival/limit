//! `/team` command — manage and execute tasks with multi-agent teams.
//!
//! Subcommands: `create`, `delete`, `list`, `status`, `start`, `history`.
//!
//! `create` registers a team with a placeholder provider.
//! `start` replaces the placeholder with a real provider from the active
//! [`AgentBridge`](crate::agent_bridge::AgentBridge) before executing.

use crate::error::CliError;
use crate::tui::commands::registry::{Command, CommandContext, CommandResult};
use limit_agent::team::{Team, TeamConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Stored team together with its configuration.
struct TeamEntry {
    team: Team,
    config: TeamConfig,
}

/// The `/team` command.
pub struct TeamCommand {
    teams: Arc<RwLock<HashMap<String, TeamEntry>>>,
}

impl Default for TeamCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl TeamCommand {
    pub fn new() -> Self {
        Self {
            teams: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// ── Command trait ─────────────────────────────────────────────────────

impl Command for TeamCommand {
    fn name(&self) -> &str {
        "team"
    }

    fn description(&self) -> &str {
        "Manage and execute tasks with multi-agent teams"
    }

    fn usage(&self) -> Vec<&str> {
        vec![
            "/team create --name <name> [--juniors N]",
            "/team list",
            "/team start --team <name> --task <description>",
            "/team status <name>",
            "/team history <name>",
            "/team delete <name>",
        ]
    }

    fn execute(&self, args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        let parts: Vec<&str> = args.split_whitespace().collect();

        if parts.is_empty() {
            ctx.add_system_message(
                "Usage: /team <subcommand>\n\
                 Subcommands: create, delete, list, status, start, history"
                    .to_string(),
            );
            return Ok(CommandResult::Continue);
        }

        match parts[0] {
            "create" => self.handle_create(&parts[1..], ctx),
            "delete" => self.handle_delete(parts.get(1).copied().unwrap_or(""), ctx),
            "list" => self.handle_list(ctx),
            "status" => self.handle_status(parts.get(1).copied().unwrap_or(""), ctx),
            "start" => self.handle_start(&parts[1..], ctx),
            "history" => self.handle_history(parts.get(1).copied().unwrap_or(""), ctx),
            other => {
                ctx.add_system_message(format!("Unknown subcommand: {}", other));
                Ok(CommandResult::Continue)
            }
        }
    }
}

// ── Subcommand handlers ───────────────────────────────────────────────

impl TeamCommand {
    fn handle_create(
        &self,
        args: &[&str],
        ctx: &mut CommandContext,
    ) -> Result<CommandResult, CliError> {
        let mut name = None;
        let mut juniors = 2usize;

        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--name" | "-n" => {
                    name = args.get(i + 1).map(|s| s.to_string());
                    i += 2;
                }
                "--juniors" | "-j" => {
                    juniors = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(2);
                    i += 2;
                }
                _ => i += 1,
            }
        }

        let name = match name {
            Some(n) => n,
            None => {
                ctx.add_system_message("Error: team name required. Use --name <name>".into());
                return Ok(CommandResult::Continue);
            }
        };

        let config = TeamConfig {
            num_juniors: juniors,
            ..TeamConfig::default()
        };

        let rt = tokio::runtime::Handle::current();
        let teams = self.teams.clone();
        let msg = rt.block_on(async {
            let mut teams = teams.write().await;
            if teams.contains_key(&name) {
                format!("⚠️  Team '{}' already exists", name)
            } else {
                let team = create_placeholder_team(&name, &config);
                teams.insert(name.clone(), TeamEntry { team, config });
                format!(
                    "✅ Team '{}' registered with {} junior agents\n\
                     Run /team start --team \"{}\" --task \"<description>\" to execute",
                    name, juniors, name
                )
            }
        });

        ctx.add_system_message(msg);
        Ok(CommandResult::Continue)
    }

    fn handle_delete(
        &self,
        name: &str,
        ctx: &mut CommandContext,
    ) -> Result<CommandResult, CliError> {
        if name.is_empty() {
            ctx.add_system_message("Error: team name required".into());
            return Ok(CommandResult::Continue);
        }

        let rt = tokio::runtime::Handle::current();
        let teams = self.teams.clone();
        let msg = rt.block_on(async {
            let mut teams = teams.write().await;
            if teams.remove(name).is_some() {
                format!("✅ Team '{}' deleted", name)
            } else {
                format!("⚠️  Team '{}' not found", name)
            }
        });

        ctx.add_system_message(msg);
        Ok(CommandResult::Continue)
    }

    fn handle_list(&self, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        let rt = tokio::runtime::Handle::current();
        let teams = self.teams.clone();
        let msg = rt.block_on(async {
            let teams = teams.read().await;
            if teams.is_empty() {
                "No teams created yet. Use /team create --name <name>".to_string()
            } else {
                let list = teams
                    .iter()
                    .map(|(n, e)| format!("- {} ({} juniors)", n, e.team.jrs.len()))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("Teams:\n{}", list)
            }
        });

        ctx.add_system_message(msg);
        Ok(CommandResult::Continue)
    }

    fn handle_status(
        &self,
        name: &str,
        ctx: &mut CommandContext,
    ) -> Result<CommandResult, CliError> {
        if name.is_empty() {
            ctx.add_system_message("Error: team name required".into());
            return Ok(CommandResult::Continue);
        }

        let rt = tokio::runtime::Handle::current();
        let teams = self.teams.clone();
        let msg = rt.block_on(async {
            let teams = teams.read().await;
            match teams.get(name) {
                Some(entry) => format!(
                    "Team: {}\nPM: ready\nTL: ready\nJuniors: {} agents\n\
                     Max parallel tasks: {}",
                    name,
                    entry.team.jrs.len(),
                    entry.config.max_parallel_tasks,
                ),
                None => format!("Team '{}' not found", name),
            }
        });

        ctx.add_system_message(msg);
        Ok(CommandResult::Continue)
    }

    fn handle_start(
        &self,
        args: &[&str],
        ctx: &mut CommandContext,
    ) -> Result<CommandResult, CliError> {
        let mut team_name = None;
        let mut task = None;

        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--team" | "-t" => {
                    team_name = args.get(i + 1).map(|s| s.to_string());
                    i += 2;
                }
                "--task" | "-k" => {
                    // Collect remaining args as the task description
                    task = Some(args[i + 1..].join(" "));
                    break;
                }
                _ => i += 1,
            }
        }

        let team_name = match team_name {
            Some(n) => n,
            None => {
                ctx.add_system_message(
                    "Error: --team <name> required\n\
                     Usage: /team start --team <name> --task <description>"
                        .into(),
                );
                return Ok(CommandResult::Continue);
            }
        };

        let task = match task {
            Some(t) if !t.trim().is_empty() => t,
            _ => {
                ctx.add_system_message(
                    "Error: --task <description> required\n\
                     Usage: /team start --team <name> --task <description>"
                        .into(),
                );
                return Ok(CommandResult::Continue);
            }
        };

        // We need the AgentBridge to get the real LLM provider and tools.
        // The bridge is accessible via CommandContext's state, which holds
        // a reference to the TuiBridge. However, CommandContext doesn't
        // expose it directly. We instead spawn the execution in the async
        // runtime and let the caller's event loop drive UI updates.
        //
        // For now, we run synchronously using block_on. The task will show
        // results as a system message when complete.

        ctx.add_system_message(format!("🚀 Team '{}' starting task...", team_name));

        let rt = tokio::runtime::Handle::current();
        let teams = self.teams.clone();
        let name_clone = team_name.clone();
        let task_clone = task.clone();

        let result = rt.block_on(async move {
            let mut teams = teams.write().await;

            let entry = match teams.get_mut(&name_clone) {
                Some(e) => e,
                None => {
                    return format!("⚠️  Team '{}' not found", name_clone);
                }
            };

            // Build a real provider and tool registry from the AgentBridge
            // that's already running in the TUI. Since we don't have direct
            // access, we re-create the provider from the config.
            let config = limit_llm::Config::load().map_err(|e| e.to_string());
            let provider = match &config {
                Ok(cfg) => {
                    limit_llm::ProviderFactory::create_provider(cfg).map_err(|e| e.to_string())
                }
                Err(e) => Err(e.clone()),
            };

            let real_provider: Box<dyn limit_llm::LlmProvider> = match provider {
                Ok(p) => p,
                Err(e) => {
                    return format!(
                        "⚠️  Failed to create LLM provider: {}\n\
                         Please check your config in ~/.limit/config.toml",
                        e
                    );
                }
            };

            // Re-create team with the real provider and proper tools
            let tool_registry = build_tool_registry();
            let tools = Arc::new(tool_registry);
            let new_team = Team::new(
                entry.team.name.clone(),
                real_provider,
                entry.config.clone(),
                tools,
            );

            match new_team {
                Ok(mut team) => {
                    let exec_result = team.execute(&task_clone).await;
                    match exec_result {
                        Ok(result) => {
                            // Replace placeholder with the real team
                            entry.team = team;

                            format!(
                                "✅ Team '{}' completed in {:.1}s\n\n{}\n\n📊 Events: {}",
                                name_clone,
                                result.duration.as_secs_f64(),
                                result.solution,
                                result.events.len(),
                            )
                        }
                        Err(e) => {
                            format!("❌ Team execution failed: {}", e)
                        }
                    }
                }
                Err(e) => format!("❌ Failed to create team: {}", e),
            }
        });

        ctx.add_system_message(result);
        Ok(CommandResult::Continue)
    }

    fn handle_history(
        &self,
        name: &str,
        ctx: &mut CommandContext,
    ) -> Result<CommandResult, CliError> {
        if name.is_empty() {
            ctx.add_system_message("Error: team name required".into());
            return Ok(CommandResult::Continue);
        }

        let rt = tokio::runtime::Handle::current();
        let teams = self.teams.clone();
        let msg = rt.block_on(async {
            let teams = teams.read().await;
            match teams.get(name) {
                Some(entry) => {
                    let events = entry.team.events().await;
                    if events.is_empty() {
                        "No history recorded yet. Run /team start first.".to_string()
                    } else {
                        events
                            .iter()
                            .map(|e| format!("[{}] {}: {}", e.role, e.action, e.content))
                            .collect::<Vec<_>>()
                            .join("\n\n")
                    }
                }
                None => format!("Team '{}' not found", name),
            }
        });

        ctx.add_system_message(msg);
        Ok(CommandResult::Continue)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Create a placeholder team (noop provider, empty tools) so that
/// `list` and `status` work before the user runs `/team start`.
fn create_placeholder_team(name: &str, config: &TeamConfig) -> Team {
    struct NoopProvider;

    #[async_trait::async_trait]
    impl limit_llm::LlmProvider for NoopProvider {
        async fn send(
            &self,
            _messages: Vec<limit_llm::Message>,
            _tools: Vec<limit_llm::Tool>,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn futures::Stream<
                            Item = Result<limit_llm::ProviderResponseChunk, limit_llm::LlmError>,
                        > + Send
                        + '_,
                >,
            >,
            limit_llm::LlmError,
        > {
            use futures::stream;
            Ok(Box::pin(stream::empty()))
        }

        fn provider_name(&self) -> &str {
            "noop"
        }

        fn model_name(&self) -> &str {
            "noop"
        }

        fn clone_box(&self) -> Box<dyn limit_llm::LlmProvider> {
            Box::new(NoopProvider)
        }
    }

    let provider: Box<dyn limit_llm::LlmProvider> = Box::new(NoopProvider);
    let tools = Arc::new(limit_agent::ToolRegistry::new());

    Team::new(name.to_string(), provider, config.clone(), tools)
        .expect("placeholder team creation should not fail")
}

/// Build a [`ToolRegistry`] with the same tools that the main agent uses.
fn build_tool_registry() -> limit_agent::ToolRegistry {
    use crate::tools::{
        AstGrepTool, BashTool, FileEditTool, FileReadTool, FileWriteTool, GitAddTool, GitCloneTool,
        GitCommitTool, GitDiffTool, GitLogTool, GitPullTool, GitPushTool, GitStatusTool, GrepTool,
        LspTool, WebFetchTool, WebSearchTool,
    };

    let mut registry = limit_agent::ToolRegistry::new();

    // File tools
    let _ = registry.register(FileReadTool::new());
    let _ = registry.register(FileWriteTool::new());
    let _ = registry.register(FileEditTool::new());

    // Bash
    let _ = registry.register(BashTool::new());

    // Git tools
    let _ = registry.register(GitStatusTool::new());
    let _ = registry.register(GitDiffTool::new());
    let _ = registry.register(GitLogTool::new());
    let _ = registry.register(GitAddTool::new());
    let _ = registry.register(GitCommitTool::new());
    let _ = registry.register(GitPushTool::new());
    let _ = registry.register(GitPullTool::new());
    let _ = registry.register(GitCloneTool::new());

    // Analysis tools
    let _ = registry.register(GrepTool::new());
    let _ = registry.register(AstGrepTool::new());
    let _ = registry.register(LspTool::new());

    // Web tools
    let _ = registry.register(WebSearchTool::new());
    let _ = registry.register(WebFetchTool::new());

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_command_name() {
        let cmd = TeamCommand::new();
        assert_eq!(cmd.name(), "team");
    }

    #[test]
    fn test_team_command_description() {
        let cmd = TeamCommand::new();
        assert!(!cmd.description().is_empty());
    }

    #[test]
    fn test_team_command_usage() {
        let cmd = TeamCommand::new();
        let usage = cmd.usage();
        assert!(!usage.is_empty());
        assert!(usage.iter().any(|u| u.contains("create")));
        assert!(usage.iter().any(|u| u.contains("start")));
    }

    #[test]
    fn test_placeholder_team_creation() {
        let config = TeamConfig::default();
        let team = create_placeholder_team("test", &config);
        assert_eq!(team.name, "test");
        assert_eq!(team.jrs.len(), 2);
    }

    #[test]
    fn test_placeholder_team_custom_juniors() {
        let config = TeamConfig {
            num_juniors: 5,
            max_parallel_tasks: 8,
        };
        let team = create_placeholder_team("test", &config);
        assert_eq!(team.jrs.len(), 5);
    }

    #[test]
    fn test_build_tool_registry() {
        let registry = build_tool_registry();
        let tools = registry.list();
        assert!(tools.len() >= 15);
        assert!(tools.contains(&"file_read".to_string()));
        assert!(tools.contains(&"bash".to_string()));
        assert!(tools.contains(&"git_commit".to_string()));
    }
}
