//! `/team` command — manage and execute tasks with multi-agent teams.
//!
//! Subcommands: `create`, `delete`, `list`, `status`, `start`, `history`.
//!
//! Teams are persisted to `~/.limit/team.db` (SQLite). Config lives in
//! `config.toml` as the single source of truth for role settings.

use crate::error::CliError;
use crate::tui::commands::registry::{Command, CommandContext, CommandResult};
use limit_agent::team::{EventLevel, Team, TeamConfig, TeamDb};
use limit_tui::components::Message;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;

/// The `/team` command.
pub struct TeamCommand {
    db: Mutex<TeamDb>,
}

impl Default for TeamCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl TeamCommand {
    pub fn new() -> Self {
        let db = TeamDb::open_default().unwrap_or_else(|e| {
            tracing::warn!("Failed to open team db: {}. Using temp.", e);
            let tmp = std::env::temp_dir().join("limit-team.db");
            TeamDb::open(&tmp).expect("temp db should work")
        });
        Self { db: Mutex::new(db) }
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

        let db = self.db.lock();
        if db
            .team_exists(&name)
            .map_err(|e| CliError::Other(e.to_string()))?
        {
            ctx.add_system_message(format!("⚠️  Team '{}' already exists", name));
        } else {
            db.insert_team(&name)
                .map_err(|e| CliError::Other(e.to_string()))?;

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

        let db = self.db.lock();
        match db.delete_team(name) {
            Ok(true) => ctx.add_system_message(format!("✅ Team '{}' deleted", name)),
            Ok(false) => ctx.add_system_message(format!("⚠️  Team '{}' not found", name)),
            Err(e) => {
                tracing::warn!("Failed to delete team '{}': {}", name, e);
                ctx.add_system_message(format!("⚠️  Failed to delete team '{}'", name));
            }
        }

        Ok(CommandResult::Continue)
    }

    fn handle_list(&self, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        let db = self.db.lock();
        let teams = db
            .list_teams()
            .map_err(|e| CliError::Other(e.to_string()))?;

        if teams.is_empty() {
            ctx.add_system_message("No teams created yet. Use /team create --name <name>".into());
        } else {
            let list = teams
                .iter()
                .map(|n| format!("- {}", n))
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

        let db = self.db.lock();

        if !db
            .team_exists(name)
            .map_err(|e| CliError::Other(e.to_string()))?
        {
            ctx.add_system_message(format!("Team '{}' not found", name));
            return Ok(CommandResult::Continue);
        }

        // Build config from [team] section in config.toml (same source as /team start)
        let config = load_team_config();
        let juniors = config.num_juniors;

        ctx.add_system_message(format!(
            "Team: {}\nPM: ready\nTL: ready\nJuniors: {} agents\n\
             Max parallel tasks: {}",
            name, juniors, config.max_parallel_tasks,
        ));

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

        // Validate team exists
        {
            let db = self.db.lock();
            if !db
                .team_exists(&team_name)
                .map_err(|e| CliError::Other(e.to_string()))?
            {
                ctx.add_system_message(format!("⚠️  Team '{}' not found", team_name));
                return Ok(CommandResult::Continue);
            }
        }

        ctx.add_system_message(format!("🚀 Team '{}' starting task...", team_name));

        // Create progress channel and set receiver for TUI rendering
        ctx.team_progress.reset();
        let (progress_tx, progress_rx) = mpsc::unbounded_channel();
        *ctx.team_progress_rx.lock() = Some(progress_rx);

        // Prepare cloned values for the background thread
        let chat_view = ctx.chat_view.clone();
        let messages = ctx.messages.clone();
        let name_clone = team_name;
        let task_clone = task;

        // We need a TeamDb handle for the background thread.
        // Since TeamDb uses internal Mutex, we clone the Arc (but we use a
        // raw pointer approach: open a second connection to the same db file).
        let db_path = limit_agent::team::default_db_path().expect("db path should exist");

        // Spawn team execution in a background thread.
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
                    .and_then(|cfg| cfg.team.as_ref())
                    .map(limit_agent::team::TeamSection::from_raw)
                    .unwrap_or_default();
                let team_config = TeamConfig::from_section(&team_section);

                // Re-create team with real provider, config from file, and proper tools
                let tools = build_tool_registry();

                let providers = config_result
                    .as_ref()
                    .map(|c| c.providers.clone())
                    .unwrap_or_default();
                let new_team = Team::new(name_clone.clone(), real_provider, team_config.clone(), tools, providers);

                match new_team {
                    Ok(mut team) => {
                        let result = team.execute(&task_clone, Some(progress_tx)).await;

                        // Open a separate db connection for persistence (thread-safe)
                        let persist_db = match TeamDb::open(&db_path) {
                            Ok(db) => db,
                            Err(e) => {
                                tracing::warn!("Failed to open db for persistence: {}", e);
                                if let Ok(mut cv) = chat_view.lock() {
                                    cv.add_message(Message::system(
                                        "⚠️  Run completed but could not persist results".into(),
                                    ));
                                }
                                return;
                            }
                        };

                        match result {
                            Ok(result) => {
                                // Persist run result to SQLite
                                if let Err(e) = persist_db.persist_run_result(&name_clone, &task_clone, &result) {
                                    tracing::warn!("Failed to persist run result: {}", e);
                                }

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

                                // Also add to conversation history
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

                                // Persist error run
                                if let Err(pe) = persist_db.persist_run_error(&name_clone, &task_clone, std::time::Duration::from_secs(0), &err_msg) {
                                    tracing::warn!("Failed to persist run error: {}", pe);
                                }

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

        // Spawn a panic monitor
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

        let db = self.db.lock();

        match db.get_latest_run_events(name) {
            Ok(Some((events, summary))) => {
                if events.is_empty() {
                    ctx.add_system_message(format!(
                        "Team '{}' has a run but no events were recorded.",
                        name
                    ));
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

                    log.push_str(&format!("📋 Team '{}' — last run\n", name));
                    log.push_str(&format!("   Task: {}\n", summary.task));
                    log.push_str(&format!("   Status: {}\n", summary.status));

                    if let Some(ms) = summary.duration_ms {
                        log.push_str(&format!("   Duration: {:.1}s\n", ms as f64 / 1000.0));
                    }
                    if summary.tokens_input > 0 || summary.tokens_output > 0 {
                        log.push_str(&format!(
                            "   Tokens: {} in / {} out\n",
                            summary.tokens_input, summary.tokens_output
                        ));
                    }

                    log.push_str(&format!("   Events ({}", events.len()));
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
            Ok(None) => {
                ctx.add_system_message(format!(
                    "Team '{}' has no run history. Run /team start first.",
                    name
                ));
            }
            Err(e) => {
                ctx.add_system_message(format!("⚠️  Failed to load history: {}", e));
            }
        }

        Ok(CommandResult::Continue)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Load team config from `[team]` section in `config.toml`.
fn load_team_config() -> TeamConfig {
    let section = limit_llm::Config::load()
        .ok()
        .and_then(|cfg| {
            cfg.team
                .map(|v| limit_agent::team::TeamSection::from_raw(&v))
        })
        .unwrap_or_default();
    TeamConfig::from_section(&section)
}

/// Build a base [`ToolRegistry`] with all standard tools (no `team_start`).
fn build_base_registry() -> limit_agent::ToolRegistry {
    use crate::agent_bridge::AgentBridge;
    use crate::tools::{
        AstGrepTool, BashTool, FileEditTool, FileReadTool, FileWriteTool, GitAddTool, GitCloneTool,
        GitCommitTool, GitDiffTool, GitLogTool, GitPullTool, GitPushTool, GitStatusTool, GrepTool,
        LspTool, WebFetchTool, WebSearchTool,
    };

    let registry = limit_agent::ToolRegistry::new();

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
    for name in registry.list() {
        let (desc, params) = AgentBridge::get_tool_schema(&name);
        registry.set_schema(&name, desc, params);
    }

    registry
}

/// Build a [`ToolRegistry`] wrapped in `Arc`, including `team_start` for recursive teams.
fn build_tool_registry() -> Arc<limit_agent::ToolRegistry> {
    use crate::agent_bridge::AgentBridge;
    use crate::tools::TeamStartTool;

    let registry = build_base_registry();

    // Wrap in Arc — TeamStartTool holds a reference for recursive child teams.
    let registry = Arc::new(registry);

    // Register team_start (needs Arc, interior mutability allows this).
    registry.register_arc(Arc::new(TeamStartTool::new(
        registry.clone(),
        2, // default max_recursion_depth
    )));
    let (desc, params) = AgentBridge::get_tool_schema("team_start");
    registry.set_schema("team_start", desc, params);

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
    fn test_build_tool_registry() {
        let registry = build_tool_registry();
        let tools = registry.list();
        assert!(tools.len() >= 16);
        assert!(tools.contains(&"file_read".to_string()));
        assert!(tools.contains(&"bash".to_string()));
        assert!(tools.contains(&"git_commit".to_string()));
        assert!(tools.contains(&"team_start".to_string()));
    }
}
