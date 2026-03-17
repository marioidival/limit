//! Multi-agent team system.
//!
//! A **team** coordinates specialized agents (PM, TL, Jr) to work on a
//! complex user request through a structured workflow:
//!
//! 1. **PM** analyzes the request
//! 2. **TL** creates a technical plan and breaks it into tasks
//! 3. **Jr** agents execute the tasks (potentially in parallel)
//! 4. **TL** validates the results
//! 5. **PM** delivers a summary to the user
//!
//! Internally, each agent runs as an independent actor with its own
//! tokio task and mailbox. The [`OrchestratorActor`](orchestrator_actor::OrchestratorActor)
//! drives the 6-phase workflow by sending messages to actors and
//! collecting replies via oneshot channels.
//!
//! # Example
//!
//! ```rust,no_run
//! use limit_agent::team::{Team, TeamConfig};
//! use limit_agent::ToolRegistry;
//! use std::sync::Arc;
//! # fn make_provider() -> Box<dyn limit_llm::LlmProvider> { unimplemented!() }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = make_provider();
//! let tools = Arc::new(ToolRegistry::new());
//! let config = TeamConfig::default();
//!
//! let mut team = Team::new("my-team".into(), provider, config, tools)?;
//! let result = team.execute("Add JWT authentication", None).await?;
//! println!("Solution:\n{}", result.solution);
//! # Ok(())
//! # }
//! ```

mod actor;
mod agent;
mod agent_actor;
mod history;
mod messages;
mod orchestrator;
mod orchestrator_actor;
mod persistence;
mod progress;
mod role;
mod supervisor;
pub mod workflow;

pub use agent::{PromptResult, TeamAgent};
pub use history::{EventLevel, TeamEvent, TeamHistory};
pub use orchestrator::{parse_tasks, Task, TaskResult, TaskStatus};
pub use persistence::{TeamSnapshot, TeamStore};
pub use progress::{TaskProgressInfo, TaskProgressStatus, TeamProgressEvent, PHASE_COUNT};
pub use role::{Role, RoleConfig, TeamRolesSection, TeamSection};
#[allow(deprecated)]
pub use workflow::{execute_workflow, TeamResult, WorkflowPhase};

use crate::error::AgentError;
use crate::registry::ToolRegistry;
use crate::team::actor::{spawn, ActorRef};
use crate::team::agent_actor::AgentActor;
use crate::team::messages::TeamMessage;
use crate::team::orchestrator_actor::OrchestratorActor;
use limit_llm::LlmProvider;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};

/// Configuration for creating a [`Team`].
///
/// Mirrors the `[team]` section in `config.toml`. Prefer using
/// [`TeamConfig::from_section`] to build from a parsed [`TeamSection`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    /// Number of Junior developer agents to spawn.
    pub num_juniors: usize,
    /// Maximum number of tasks to execute concurrently.
    pub max_parallel_tasks: usize,
    /// Enable streaming output.
    pub enable_streaming: bool,
    /// Per-role overrides (model, tool whitelist, max_tokens).
    pub roles: TeamRolesSection,
}

impl Default for TeamConfig {
    fn default() -> Self {
        Self {
            num_juniors: 2,
            max_parallel_tasks: 4,
            enable_streaming: true,
            roles: TeamRolesSection::default(),
        }
    }
}

impl TeamConfig {
    /// Build a `TeamConfig` from a parsed [`TeamSection`].
    pub fn from_section(section: &TeamSection) -> Self {
        Self {
            num_juniors: section.default_juniors,
            max_parallel_tasks: section.max_parallel_tasks,
            enable_streaming: section.enable_streaming,
            roles: section.roles.clone(),
        }
    }
}

/// A named team containing a PM, TL, and one or more Jr agents.
///
/// Agents are rebuilt per [`execute`](Team::execute) call from the stored
/// provider and tools, ensuring a clean state each run.
pub struct Team {
    /// Human-readable name for this team.
    pub name: String,
    /// Product Manager agent.
    pub pm: TeamAgent,
    /// Tech Lead agent.
    pub tl: TeamAgent,
    /// Junior developer agents.
    pub jrs: Vec<TeamAgent>,
    /// LLM provider (stored for rebuilding agents per execute call).
    provider: Box<dyn LlmProvider>,
    /// Tool registry (shared across all agents).
    tools: Arc<ToolRegistry>,
    /// Shared event log.
    pub history: Arc<RwLock<TeamHistory>>,
    /// Configuration used when creating this team.
    pub config: TeamConfig,
}

impl Team {
    /// Create a new team with the given name, provider, and tools.
    ///
    /// All agents share the same LLM provider but have their own
    /// conversation history, role-specific system prompts, and
    /// tool whitelists from `config.roles`.
    pub fn new(
        name: String,
        provider: Box<dyn LlmProvider>,
        config: TeamConfig,
        tools: Arc<ToolRegistry>,
    ) -> Result<Self, AgentError> {
        let pm_tools = config.roles.pm.tools.clone();
        let tl_tools = config.roles.tl.tools.clone();
        let jr_tools = config.roles.jr.tools.clone();

        let pm =
            TeamAgent::with_allowed_tools(Role::PM, provider.clone_box(), tools.clone(), pm_tools);
        let tl =
            TeamAgent::with_allowed_tools(Role::TL, provider.clone_box(), tools.clone(), tl_tools);

        let jrs = (0..config.num_juniors)
            .map(|_| {
                TeamAgent::with_allowed_tools(
                    Role::Jr,
                    provider.clone_box(),
                    tools.clone(),
                    jr_tools.clone(),
                )
            })
            .collect();

        Ok(Self {
            name,
            pm,
            tl,
            jrs,
            provider,
            tools,
            history: Arc::new(RwLock::new(TeamHistory::new())),
            config,
        })
    }

    /// Execute a user request through the full team workflow.
    ///
    /// Spawns agent actors (PM, TL, Jrs), an orchestrator, and a supervisor.
    /// Returns a [`TeamResult`] containing the PM's delivery summary,
    /// wall-clock duration, and all recorded events.
    pub async fn execute(
        &mut self,
        user_request: &str,
        progress_tx: Option<mpsc::UnboundedSender<TeamProgressEvent>>,
    ) -> Result<TeamResult, AgentError> {
        // Shared token counters across all agents
        let token_input = Arc::new(AtomicU64::new(0));
        let token_output = Arc::new(AtomicU64::new(0));

        // 1. Build fresh agents from stored config
        let pm_provider = self.provider.clone_box();
        let tl_provider = self.provider.clone_box();
        let jr_provider: Box<dyn LlmProvider> = match self.config.roles.jr.max_tokens {
            Some(m) => self.provider.with_max_tokens(m),
            None => self.provider.clone_box(),
        };

        let pm = TeamAgent::with_allowed_tools(
            Role::PM,
            pm_provider,
            self.tools.clone(),
            self.config.roles.pm.tools.clone(),
        );
        let tl = TeamAgent::with_allowed_tools(
            Role::TL,
            tl_provider,
            self.tools.clone(),
            self.config.roles.tl.tools.clone(),
        );
        let jrs: Vec<TeamAgent> = (0..self.config.num_juniors)
            .map(|_| {
                TeamAgent::with_allowed_tools(
                    Role::Jr,
                    jr_provider.clone_box(),
                    self.tools.clone(),
                    self.config.roles.jr.tools.clone(),
                )
            })
            .collect();

        // 2. Spawn agent actors
        let pm_actor = AgentActor::new(
            Role::PM,
            pm,
            self.history.clone(),
            progress_tx.clone(),
            token_input.clone(),
            token_output.clone(),
        );
        let (pm_ref, pm_handle) = spawn(pm_actor, 32);

        let tl_actor = AgentActor::new(
            Role::TL,
            tl,
            self.history.clone(),
            progress_tx.clone(),
            token_input.clone(),
            token_output.clone(),
        );
        let (tl_ref, tl_handle) = spawn(tl_actor, 32);

        let mut jr_refs: Vec<ActorRef<TeamMessage>> = Vec::with_capacity(jrs.len());
        let mut jr_handles: Vec<tokio::task::JoinHandle<()>> = Vec::with_capacity(jrs.len());
        for jr in jrs {
            let jr_actor = AgentActor::new(
                Role::Jr,
                jr,
                self.history.clone(),
                progress_tx.clone(),
                token_input.clone(),
                token_output.clone(),
            );
            let (jr_ref, jr_handle) = spawn(jr_actor, 32);
            jr_refs.push(jr_ref);
            jr_handles.push(jr_handle);
        }

        // 3. Create oneshot for final result
        let (result_tx, result_rx) = oneshot::channel();

        // 4. Spawn orchestrator
        let orchestrator = OrchestratorActor::new(
            pm_ref,
            tl_ref,
            jr_refs,
            self.config.max_parallel_tasks,
            self.history.clone(),
            progress_tx,
            result_tx,
            user_request.to_string(),
        );

        let orchestrator_handle = tokio::spawn(async move {
            orchestrator.run_workflow().await;
        });

        // 5. Await the result
        let result = result_rx.await.map_err(|_| {
            // The orchestrator likely panicked. Try to extract the panic message.
            use futures::future::FutureExt;
            if let Some(Err(join_err)) = orchestrator_handle.now_or_never() {
                if let Ok(panic_payload) = join_err.try_into_panic() {
                    let msg: String = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    tracing::error!("[team] orchestrator PANICKED: {}", msg);
                }
            }
            AgentError::ActorError("orchestrator dropped result channel".into())
        })??;

        // 6. Graceful shutdown — abort all actor tasks
        // (orchestrator_handle was consumed by the error handler above,
        //  but it has already completed by this point)
        pm_handle.abort();
        tl_handle.abort();
        for h in jr_handles {
            h.abort();
        }

        let mut result = result;

        // Accumulate token counts from shared counters
        result.tokens_input = token_input.load(std::sync::atomic::Ordering::Relaxed);
        result.tokens_output = token_output.load(std::sync::atomic::Ordering::Relaxed);

        Ok(result)
    }

    /// Read the team's event history.
    pub async fn events(&self) -> Vec<TeamEvent> {
        self.history.read().await.events().to_vec()
    }

    /// Clear all event history and reset all agent conversations.
    pub async fn reset(&mut self) {
        self.pm.clear_history();
        self.tl.clear_history();
        for jr in &mut self.jrs {
            jr.clear_history();
        }
        self.history.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_config_default() {
        let config = TeamConfig::default();
        assert_eq!(config.num_juniors, 2);
        assert_eq!(config.max_parallel_tasks, 4);
        assert!(config.enable_streaming);
    }

    #[test]
    fn test_team_config_from_section() {
        let section = TeamSection {
            default_juniors: 5,
            max_parallel_tasks: 10,
            enable_streaming: false,
            roles: TeamRolesSection::default(),
        };
        let config = TeamConfig::from_section(&section);
        assert_eq!(config.num_juniors, 5);
        assert_eq!(config.max_parallel_tasks, 10);
    }

    #[test]
    fn test_team_config_clone() {
        let config = TeamConfig::default();
        let _ = config.clone();
    }

    #[test]
    fn test_role_reexports() {
        let _ = Role::PM;
        let _ = Role::TL;
        let _ = Role::Jr;
    }

    #[test]
    fn test_parse_tasks_reexport() {
        let tasks = parse_tasks("TASK: do stuff");
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn test_team_section_reexport() {
        let _ = TeamSection::default();
        let _ = RoleConfig::default();
    }
}
