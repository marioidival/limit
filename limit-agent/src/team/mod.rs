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
//! let result = team.execute("Add JWT authentication").await?;
//! println!("Solution:\n{}", result.solution);
//! # Ok(())
//! # }
//! ```

mod agent;
mod history;
mod orchestrator;
mod persistence;
mod role;
mod workflow;

pub use agent::TeamAgent;
pub use history::{EventLevel, TeamEvent, TeamHistory};
pub use orchestrator::{parse_tasks, Task, TaskResult, TaskStatus};
pub use persistence::{TeamSnapshot, TeamStore};
pub use role::{Role, RoleConfig, TeamRolesSection, TeamSection};
pub use workflow::{execute_workflow, TeamResult, WorkflowPhase};

use crate::error::AgentError;
use crate::registry::ToolRegistry;
use limit_llm::LlmProvider;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

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
    /// Per-role overrides (model, tool whitelist).
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
pub struct Team {
    /// Human-readable name for this team.
    pub name: String,
    /// Product Manager agent.
    pub pm: TeamAgent,
    /// Tech Lead agent.
    pub tl: TeamAgent,
    /// Junior developer agents.
    pub jrs: Vec<TeamAgent>,
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
            history: Arc::new(RwLock::new(TeamHistory::new())),
            config,
        })
    }

    /// Execute a user request through the full team workflow.
    ///
    /// Returns a [`TeamResult`] containing the PM's delivery summary,
    /// wall-clock duration, and all recorded events.
    pub async fn execute(&mut self, user_request: &str) -> Result<TeamResult, AgentError> {
        execute_workflow(
            &mut self.pm,
            &mut self.tl,
            &mut self.jrs,
            user_request,
            &self.history,
            self.config.max_parallel_tasks,
        )
        .await
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
