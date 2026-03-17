//! `/team` command — manage and execute tasks with multi-agent teams.
//!
//! Subcommands: `create`, `delete`, `list`, `status`, `start`, `history`.
//!
//! `create` registers a team with a placeholder provider.
//! `start` replaces the placeholder with a real provider from the active
//! [`AgentBridge`](crate::agent_bridge::AgentBridge) before executing.
//!
//! Teams are persisted to `~/.limit/teams/` as JSON snapshots.

use crate::error::CliError;
use crate::tui::commands::registry::{Command, CommandContext, CommandResult};
use limit_agent::team::{EventLevel, Team, TeamConfig, TeamSnapshot, TeamStore};
use limit_tui::components::Message;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Stored team together with its configuration.
struct TeamEntry {
    #[allow(dead_code)]
    team: Team,
    config: TeamConfig,
}

/// The `/team` command.
pub struct TeamCommand {
    /// Placeholder teams for metadata (config, junios count).
    teams: Mutex<HashMap<String, TeamEntry>>,
    /// Execution histories indexed by team name (populated after team runs).
    execution_histories: Arc<Mutex<HashMap<String, limit_agent::team::TeamHistory>>>,
    store: Mutex<TeamStore>,
}

impl Default for TeamCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl TeamCommand {
    pub fn new() -> Self {
        let store = TeamStore::default_dir().unwrap_or_else(|e| {
            tracing::warn!("Failed to create team store: {}. Using temp dir.", e);
            let tmp = std::env::temp_dir().join("limit-teams");
            std::fs::create_dir_all(&tmp).ok();
            TeamStore::new(tmp).expect("temp dir should work")
        });
        Self {
            teams: Mutex::new(HashMap::new()),
            execution_histories: Arc::new(Mutex::new(HashMap::new())),
            store: Mutex::new(store),
        }
    }

    /// Load teams from disk into memory.
    fn load_from_store(&self, ctx: &mut CommandContext) {
        let store = self.store.lock();
        let names = match store.list() {
            Ok(n) => n,
            Err(e) => {
                ctx.add_system_message(format!("⚠️  Failed to list teams: {}", e));
                return;
            }
        };

        let mut teams = self.teams.lock();
        for name in &names {
            if teams.contains_key(name) {
                continue; // already loaded
            }
            match store.load(name) {
                Ok(Some(snap)) => {
                    let team = create_placeholder_team(&snap.name, &snap.config);
                    teams.insert(
                        name.clone(),
                        TeamEntry {
                            team,
                            config: snap.config,
                        },
                    );
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("Failed to load team '{}': {}", name, e);
                }
            }
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
            max_parallel_tasks: 4,
            enable_streaming: false,
            roles: Default::default(),
        };

        let mut teams = self.teams.lock();
        if teams.contains_key(&name) {
            ctx.add_system_message(format!("⚠️  Team '{}' already exists", name));
        } else {
            let team = create_placeholder_team(&name, &config);

            // Persist to disk
            if let Err(e) = self
                .store
                .lock()
                .save(&TeamSnapshot::new(&name, config.clone()))
            {
                tracing::warn!("Failed to persist team '{}': {}", name, e);
            }

            teams.insert(name.clone(), TeamEntry { team, config });
            ctx.add_system_message(format!(
                "✅ Team '{}' registered with {} junior agents\n\
                 Run /team start --team \"{}\" --task \"<description>\" to execute",
                name, juniors, name
            ));
        }

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

        // Remove from disk
        if let Err(e) = self.store.lock().delete(name) {
            tracing::warn!("Failed to delete team '{}' from disk: {}", name, e);
        }

        let mut teams = self.teams.lock();
        if teams.remove(name).is_some() {
            ctx.add_system_message(format!("✅ Team '{}' deleted", name));
        } else {
            ctx.add_system_message(format!("⚠️  Team '{}' not found", name));
        }

        Ok(CommandResult::Continue)
    }

    fn handle_list(&self, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        // Load persisted teams first
        self.load_from_store(ctx);

        let teams = self.teams.lock();
        if teams.is_empty() {
            ctx.add_system_message("No teams created yet. Use /team create --name <name>".into());
        } else {
            let list = teams
                .iter()
                .map(|(n, e)| format!("- {} ({} juniors)", n, e.team.jrs.len()))
                .collect::<Vec<_>>()
                .join("\n");
            ctx.add_system_message(format!("Teams:\n{}", list));
        }

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

        let teams = self.teams.lock();
        match teams.get(name) {
            Some(entry) => {
                ctx.add_system_message(format!(
                    "Team: {}\nPM: ready\nTL: ready\nJuniors: {} agents\n\
                     Max parallel tasks: {}",
                    name,
                    entry.team.jrs.len(),
                    entry.config.max_parallel_tasks,
                ));
            }
            None => {
                ctx.add_system_message(format!("Team '{}' not found", name));
            }
        }

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

        ctx.add_system_message(format!("🚀 Team '{}' starting task...", team_name));

        // Create progress channel and set receiver for TUI rendering
        let (progress_tx, progress_rx) = mpsc::unbounded_channel();
        *ctx.team_progress_rx.lock() = Some(progress_rx);

        // Prepare cloned values for the background thread
        let chat_view = ctx.chat_view.clone();
        let messages = ctx.messages.clone();
        let name_clone = team_name;
        let task_clone = task;
        let execution_histories = Arc::clone(&self.execution_histories);

        // Validate team exists before spawning background work
        {
            let teams = self.teams.lock();
            if !teams.contains_key(&name_clone) {
                ctx.add_system_message(format!("⚠️  Team '{}' not found", name_clone));
                return Ok(CommandResult::Continue);
            }
        }

        // Spawn team execution in a background thread.
        //
        // This creates a dedicated tokio runtime for team execution. While creating
        // a runtime per execution has overhead, it's necessary because:
        // 1. The TUI event loop is synchronous
        // 2. We need async execution without blocking the UI
        // 3. Team execution is infrequent and long-running, so overhead is acceptable
        let name_for_monitor = name_clone.clone();
        let spawn_result = std::thread::Builder::new()
            .name(format!("team-{}-executor", name_clone))
            .spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        if let Ok(mut cv) = chat_view.lock() {
                            cv.add_message(Message::system(format!(
                                "❌ Failed to create runtime: {}",
                                e
                            )));
                        }
                        return;
                    }
                };

                rt.block_on(async move {
                let config_result = limit_llm::Config::load();
                let provider = match &config_result {
                    Ok(cfg) => limit_llm::ProviderFactory::create_provider(cfg),
                    Err(e) => Err(limit_llm::LlmError::ConfigError(e.to_string())),
                };

                let real_provider: Box<dyn limit_llm::LlmProvider> = match provider {
                    Ok(p) => p,
                    Err(e) => {
                        if let Ok(mut cv) = chat_view.lock() {
                            cv.add_message(Message::system(format!(
                                "⚠️  Failed to create LLM provider: {}\n\
                             Please check your config in ~/.limit/config.toml",
                                e
                            )));
                        }
                        return;
                    }
                };

                // Build team config from [team] section in config.toml
                let team_section = config_result
                    .as_ref()
                    .ok()
                    .and_then(|cfg| cfg.team_raw.as_ref())
                    .map(limit_agent::team::TeamSection::from_raw)
                    .unwrap_or_default();
                let team_config = TeamConfig::from_section(&team_section);

                // Re-create team with real provider, config from file, and proper tools
                let tool_registry = build_tool_registry();
                let tools = Arc::new(tool_registry);

                let new_team = Team::new(name_clone.clone(), real_provider, team_config.clone(), tools);

                match new_team {
                    Ok(mut team) => {
                        let result = team.execute(&task_clone, Some(progress_tx)).await;

                        match result {
                            Ok(result) => {
                                // Update the execution history for this team
                                let history_clone = (*team.history.read().await).clone();
                                execution_histories.lock().insert(name_clone.clone(), history_clone);

                                let mut summary = format!(
                                    "✅ Team '{}' completed in {:.1}s",
                                    name_clone,
                                    result.duration.as_secs_f64(),
                                );

                                if result.total_tasks > 0 {
                                    summary.push_str(&format!(
                                        "\n📦 Tasks: {}/{} succeeded",
                                        result.total_tasks - result.failed_tasks,
                                        result.total_tasks,
                                    ));
                                }

                                if result.failed_tasks > 0 {
                                    summary.push_str(&format!(
                                        "\n⚠️  {} task(s) failed — check /team history for details",
                                        result.failed_tasks
                                    ));
                                }

                                if !result.files_modified.is_empty() {
                                    summary.push_str(&format!(
                                        "\n📁 Files: {}",
                                        result.files_modified.join(", ")
                                    ));
                                }

                                summary.push_str(&format!(
                                    "\n📊 Events: {}",
                                    result.events.len(),
                                ));

                                summary.push_str(&format!("\n\n{}", result.solution));

                                if let Ok(mut cv) = chat_view.lock() {
                                    cv.add_message(Message::system(summary.clone()));
                                }

                                // Also add to conversation history so the single-agent
                                // has context about what the team did.
                                if let Ok(mut msgs) = messages.lock() {
                                    msgs.push(limit_llm::Message {
                                        role: limit_llm::Role::System,
                                        content: Some(format!(
                                            "[Team '{}' completed]\n{}",
                                            name_clone, summary
                                        )),
                                        tool_calls: None,
                                        tool_call_id: None,
                                    });
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("{}", e);
                                let hint = if err_msg.contains("Rate limit")
                                    || err_msg.contains("429")
                                {
                                    "\n💡 Tip: Rate limited — try again in a moment or use a team config with a cheaper model for Jr agents."
                                } else if err_msg.contains("API key") {
                                    "\n💡 Tip: Check your API key in ~/.limit/config.toml"
                                } else if err_msg.contains("timeout") {
                                    "\n💡 Tip: Request timed out — try breaking the task into smaller pieces."
                                } else {
                                    ""
                                };

                                if let Ok(mut cv) = chat_view.lock() {
                                    cv.add_message(Message::system(format!(
                                        "❌ Team execution failed: {}{}",
                                        err_msg, hint
                                    )));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if let Ok(mut cv) = chat_view.lock() {
                            cv.add_message(Message::system(format!(
                                "❌ Failed to create team: {}. Check your config in ~/.limit/config.toml",
                                e
                            )));
                        }
                    }
                }
                });
            });

        let handle = spawn_result.map_err(|e| {
            tracing::error!("Failed to spawn team executor thread: {}", e);
            CliError::IoError(std::io::Error::other(e))
        })?;

        // Spawn a panic monitor so thread crashes are reported to the user
        // instead of being silently swallowed (the old code dropped the JoinHandle).
        {
            let monitor_chat = ctx.chat_view.clone();
            let monitor_name = name_for_monitor;
            let _ = std::thread::Builder::new()
                .name(format!("team-{}-panic-monitor", monitor_name))
                .spawn(move || {
                    if let Err(panic_payload) = handle.join() {
                        let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                            format!("❌ Team '{}' crashed: {}", monitor_name, s)
                        } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                            format!("❌ Team '{}' crashed: {}", monitor_name, s)
                        } else {
                            format!("❌ Team '{}' thread panicked", monitor_name)
                        };
                        if let Ok(mut cv) = monitor_chat.lock() {
                            cv.add_message(Message::system(msg));
                        }
                    }
                });
        }

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

        // Read from execution_histories (populated after team runs)
        let events_result = {
            let histories = self.execution_histories.lock();
            histories.get(name).map(|history| history.events().to_vec())
        };

        match events_result {
            Some(events) => {
                if events.is_empty() {
                    ctx.add_system_message(
                        "No history recorded yet. Run /team start first.".into(),
                    );
                } else {
                    let errors: Vec<_> = events
                        .iter()
                        .filter(|e| e.level == EventLevel::Error)
                        .collect();
                    let warnings: Vec<_> = events
                        .iter()
                        .filter(|e| e.level == EventLevel::Warn)
                        .collect();

                    let mut log = String::new();

                    // Summary header
                    log.push_str(&format!(
                        "📋 Team '{}' history ({} events",
                        name,
                        events.len()
                    ));
                    if !errors.is_empty() {
                        log.push_str(&format!(", {} errors", errors.len()));
                    }
                    if !warnings.is_empty() {
                        log.push_str(&format!(", {} warnings", warnings.len()));
                    }
                    log.push_str("):\n\n");

                    for e in &events {
                        log.push_str(&format!("[{}] {}: {}\n\n", e.role, e.action, e.content));
                    }

                    if !errors.is_empty() {
                        log.push_str("⚠️  Errors:\n");
                        for e in errors {
                            log.push_str(&format!(
                                "  - [{}] {}: {}\n",
                                e.role, e.action, e.content
                            ));
                        }
                    }

                    ctx.add_system_message(log);
                }
            }
            None => {
                ctx.add_system_message(format!(
                    "Team '{}' has no history. Run /team start first.",
                    name
                ));
            }
        }

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
    use crate::agent_bridge::AgentBridge;
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

    // Set proper LLM schemas so agents can generate correct tool-call arguments.
    // Without these, tools get a generic `{"type": "object"}` schema and the LLM
    // cannot determine the required parameters.
    for name in registry.list() {
        let (desc, params) = AgentBridge::get_tool_schema(&name);
        registry.set_schema(&name, desc, params);
    }

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
            enable_streaming: true,
            roles: Default::default(),
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
