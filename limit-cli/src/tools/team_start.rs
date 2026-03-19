use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::registry::ToolRegistry;
use limit_agent::team::{Team, TeamConfig};
use limit_agent::Tool;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Tool that spawns a child team to handle a sub-task.
///
/// Jr agents (and other roles) can call `team_start` to delegate work
/// to a full PM → TL → Jr workflow. Recursion is bounded by
/// `max_depth` to prevent unbounded nesting.
pub struct TeamStartTool {
    /// Shared tool registry — child teams' Jrs also get `team_start`.
    registry: Arc<ToolRegistry>,
    /// Current nesting depth (shared across concurrent calls).
    depth: Arc<AtomicUsize>,
    /// Maximum allowed nesting depth.
    max_depth: usize,
}

impl TeamStartTool {
    pub fn new(registry: Arc<ToolRegistry>, max_depth: usize) -> Self {
        Self {
            registry,
            depth: Arc::new(AtomicUsize::new(0)),
            max_depth,
        }
    }

    /// Create with a shared depth counter (used by child teams).
    pub fn with_shared_depth(
        registry: Arc<ToolRegistry>,
        depth: Arc<AtomicUsize>,
        max_depth: usize,
    ) -> Self {
        Self {
            registry,
            depth,
            max_depth,
        }
    }
}

#[async_trait]
impl Tool for TeamStartTool {
    fn name(&self) -> &str {
        "team_start"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        // Atomically check-and-increment depth using CAS
        let mut current = self.depth.load(Ordering::Relaxed);
        loop {
            if current >= self.max_depth {
                return Ok(json!({
                    "success": false,
                    "error": format!(
                        "Maximum recursion depth ({}) reached. Cannot spawn more child teams.",
                        self.max_depth
                    )
                }));
            }
            match self.depth.compare_exchange_weak(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }

        // Parse args
        let task = args
            .get("task")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        if task.is_empty() {
            self.depth.fetch_sub(1, Ordering::SeqCst);
            return Ok(json!({
                "success": false,
                "error": "Missing required parameter: task (string description of what the child team should do)"
            }));
        }

        let juniors_override = args
            .get("juniors")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);
        let max_parallel_override = args
            .get("max_parallel")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        let result = self
            .run_child_team(task, juniors_override, max_parallel_override)
            .await;

        // Decrement depth
        self.depth.fetch_sub(1, Ordering::SeqCst);

        result
    }
}

impl TeamStartTool {
    async fn run_child_team(
        &self,
        task: &str,
        juniors_override: Option<usize>,
        max_parallel_override: Option<usize>,
    ) -> Result<Value, AgentError> {
        // Load config from file
        let config_result = limit_llm::Config::load();

        let provider: Box<dyn limit_llm::LlmProvider> = match &config_result {
            Ok(cfg) => limit_llm::ProviderFactory::create_provider(cfg)
                .map_err(|e| AgentError::ToolError(format!("Failed to create provider: {}", e)))?,
            Err(e) => {
                return Ok(json!({
                    "success": false,
                    "error": format!("Failed to load config: {}", e)
                }));
            }
        };

        // Build team config
        let team_section = config_result
            .as_ref()
            .ok()
            .and_then(|cfg| cfg.team.as_ref())
            .map(limit_agent::team::TeamSection::from_raw)
            .unwrap_or_default();
        let mut team_config = TeamConfig::from_section(&team_section);

        // Override depth for child team
        let remaining_depth = self
            .max_depth
            .saturating_sub(self.depth.load(Ordering::Relaxed));
        team_config.max_recursion_depth = remaining_depth;

        // Apply optional overrides
        if let Some(juniors) = juniors_override {
            team_config.num_juniors = juniors;
        }
        if let Some(max_parallel) = max_parallel_override {
            team_config.max_parallel_tasks = max_parallel;
        }

        // Get providers map for per-role provider overrides
        let providers = config_result
            .as_ref()
            .map(|c| c.providers.clone())
            .unwrap_or_default();

        // Create child team
        let mut team = Team::new(
            format!("child-team-L{}", self.depth.load(Ordering::Relaxed)),
            provider,
            team_config,
            self.registry.clone(),
            providers,
        )
        .map_err(|e| AgentError::ToolError(format!("Failed to create child team: {}", e)))?;

        // Execute
        let start = std::time::Instant::now();
        let result = team
            .execute(task, None)
            .await
            .map_err(|e| AgentError::ToolError(format!("Child team execution failed: {}", e)))?;
        let duration_secs = start.elapsed().as_secs_f64();

        Ok(json!({
            "success": true,
            "solution": result.solution,
            "files_modified": result.files_modified,
            "duration_secs": duration_secs,
            "tasks_total": result.total_tasks,
            "tasks_failed": result.failed_tasks,
            "tokens_input": result.tokens_input,
            "tokens_output": result.tokens_output,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_start_tool_name() {
        let registry = Arc::new(ToolRegistry::new());
        let tool = TeamStartTool::new(registry, 2);
        assert_eq!(tool.name(), "team_start");
    }

    #[test]
    fn test_team_start_missing_task() {
        let registry = Arc::new(ToolRegistry::new());
        let tool = TeamStartTool::new(registry, 2);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(tool.execute(json!({})));
        let val = result.unwrap();
        assert_eq!(val["success"], false);
        assert!(val["error"].as_str().unwrap().contains("task"));
    }

    #[test]
    fn test_team_start_empty_task() {
        let registry = Arc::new(ToolRegistry::new());
        let tool = TeamStartTool::new(registry, 2);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(tool.execute(json!({"task": "  "})));
        let val = result.unwrap();
        assert_eq!(val["success"], false);
    }

    #[test]
    fn test_shared_depth() {
        let registry = Arc::new(ToolRegistry::new());
        let depth = Arc::new(AtomicUsize::new(1));
        let tool = TeamStartTool::with_shared_depth(registry, depth.clone(), 2);

        let rt = tokio::runtime::Runtime::new().unwrap();
        // Depth is already 1, max is 2, so it should still work (1 < 2)
        let result = rt.block_on(tool.execute(json!({"task": "test"})));
        // Will fail because no config, but should NOT fail on depth check
        let val = result.unwrap();
        // Either success (unlikely without real config) or a config error
        if val["success"] == false {
            let err = val["error"].as_str().unwrap_or("");
            assert!(!err.contains("recursion depth"));
        }
    }
}
