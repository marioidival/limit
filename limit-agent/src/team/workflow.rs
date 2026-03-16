//! Team workflow — the pipeline that coordinates PM → TL → Jr agents.
//!
//! This module defines the high-level [`TeamWorkflow`] enum that drives
//! the team from request analysis through to delivery.

use crate::error::AgentError;
use crate::team::agent::TeamAgent;
use crate::team::history::{EventLevel, TeamEvent, TeamHistory};
use crate::team::orchestrator::{parse_tasks, Task, TaskResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Maximum conversation history per agent to prevent unbounded growth.
const MAX_HISTORY_PER_AGENT: usize = 50;

/// Compiled regex for extracting file paths (lazy-initialized once).
static FILE_PATH_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

/// Track retry statistics during workflow execution (thread-safe for parallel tasks).
#[derive(Debug)]
struct RetryTracker {
    count: Arc<AtomicUsize>,
}

impl RetryTracker {
    fn new() -> Self {
        Self {
            count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn total(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    fn clone_counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.count)
    }
}

/// The final result produced by a team execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResult {
    /// The PM's delivery summary for the user.
    pub solution: String,
    /// Wall-clock time for the full pipeline.
    pub duration: Duration,
    /// All events recorded during execution.
    pub events: Vec<TeamEvent>,
    /// Number of retried LLM calls across all agents.
    pub total_retries: usize,
    /// Number of tasks that failed.
    pub failed_tasks: usize,
    /// Total number of tasks executed.
    pub total_tasks: usize,
    /// Files modified during execution (extracted from tool call results).
    pub files_modified: Vec<String>,
}

/// Named phases of the team workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowPhase {
    /// PM is analyzing the request.
    PmAnalysis,
    /// TL is creating a technical plan.
    TlPlan,
    /// TL is breaking the plan into tasks.
    TlBreakdown,
    /// Jr agents are executing tasks.
    JrExecution,
    /// TL is validating results.
    TlValidation,
    /// PM is preparing the delivery summary.
    PmDelivery,
}

impl std::fmt::Display for WorkflowPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PmAnalysis => write!(f, "PM analysis"),
            Self::TlPlan => write!(f, "TL plan"),
            Self::TlBreakdown => write!(f, "TL breakdown"),
            Self::JrExecution => write!(f, "Jr execution"),
            Self::TlValidation => write!(f, "TL validation"),
            Self::PmDelivery => write!(f, "PM delivery"),
        }
    }
}

/// Drive the full team workflow: PM analysis → TL plan → breakdown → Jr execution → TL validation → PM delivery.
///
/// Returns a [`TeamResult`] with the solution, duration, events, and
/// execution statistics (retries, failures).
pub async fn execute_workflow(
    pm: &mut TeamAgent,
    tl: &mut TeamAgent,
    jrs: &mut [TeamAgent],
    user_request: &str,
    history: &Arc<RwLock<TeamHistory>>,
    max_parallel: usize,
) -> Result<TeamResult, AgentError> {
    let start = Instant::now();

    // ── Phase 1: PM analysis ──────────────────────────────────────────
    tracing::info!("[team] PM — analyzing request");
    log_phase(history, WorkflowPhase::PmAnalysis).await;
    let analysis = pm
        .prompt(&format!(
            "User request:\n{user_request}\n\nAnalyze this request and identify what needs to be done."
        ))
        .await?;
    log_event(history, "PM", "analysis", &analysis).await;

    // ── Phase 2: TL technical plan ────────────────────────────────────
    tracing::info!("[team] TL — creating technical plan");
    log_phase(history, WorkflowPhase::TlPlan).await;
    let plan = tl
        .prompt(&format!(
            "PM analysis:\n{analysis}\n\nCreate a technical plan to implement this."
        ))
        .await?;
    log_event(history, "TL", "plan", &plan).await;

    // ── Phase 3: TL task breakdown ────────────────────────────────────
    tracing::info!("[team] TL — breaking down tasks");
    log_phase(history, WorkflowPhase::TlBreakdown).await;
    let tasks = tl
        .prompt(&format!(
            "Technical plan:\n{plan}\n\nBreak this down into specific, executable tasks. \
             Format each task on its own line as:\nTASK: <description>"
        ))
        .await?;
    let tasks: Vec<Task> = parse_tasks(&tasks);
    log_event(
        history,
        "TL",
        "tasks",
        &format!("{} tasks parsed", tasks.len()),
    )
    .await;

    if tasks.is_empty() {
        tracing::warn!("[team] TL produced no parseable tasks — delivering plan as-is");
        let delivery = pm
            .prompt(&format!(
                "The Tech Lead produced a plan but no specific tasks. Here is the plan:\n\n{plan}\n\n\
                 Summarize this for the user and suggest next steps."
            ))
            .await?;
        log_event(history, "PM", "delivery", &delivery).await;

        return Ok(TeamResult {
            solution: delivery,
            duration: start.elapsed(),
            events: history.read().await.events().to_vec(),
            total_retries: 0,
            failed_tasks: 0,
            total_tasks: 0,
            files_modified: vec![],
        });
    }

    // ── Phase 4: Jr parallel execution ────────────────────────────────
    log_phase(history, WorkflowPhase::JrExecution).await;
    tracing::info!(
        "[team] Jr — executing {} tasks (max {max_parallel} parallel)",
        tasks.len()
    );
    let retry_tracker = RetryTracker::new();
    let task_results = execute_tasks_parallel(jrs, &tasks, max_parallel, &retry_tracker).await;

    // Trim Jr agent histories to prevent unbounded growth
    for jr in jrs.iter_mut() {
        jr.trim_history(MAX_HISTORY_PER_AGENT);
    }

    let failed_tasks = task_results.iter().filter(|r| !r.success).count();
    let total_tasks = task_results.len();
    let total_retries = retry_tracker.total();

    // Extract modified files from Jr tool-call results
    let files_modified = extract_modified_files(&task_results);

    let results_summary: String = task_results
        .iter()
        .map(|r| {
            if r.success {
                format!("[✅] {}", r.output)
            } else {
                format!("[❌] {}", r.output)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    log_event(history, "Jr", "execution", &results_summary).await;

    log_phase(history, WorkflowPhase::TlValidation).await;
    // ── Phase 5: TL validation ────────────────────────────────────────
    tracing::info!("[team] TL — validating results");
    let validation = tl
        .prompt(&format!(
            "Task results:\n{results_summary}\n\n\
             Validate the implementation. Note any issues or improvements needed."
        ))
        .await?;
    log_event(history, "TL", "validation", &validation).await;

    log_phase(history, WorkflowPhase::PmDelivery).await;
    // ── Phase 6: PM delivery ──────────────────────────────────────────
    tracing::info!("[team] PM — preparing delivery");
    let delivery = pm
        .prompt(&format!(
            "Technical solution validated by TL:\n{validation}\n\n\
             Summarize the solution for the user in a clear, concise way."
        ))
        .await?;
    log_event(history, "PM", "delivery", &delivery).await;

    Ok(TeamResult {
        solution: delivery,
        duration: start.elapsed(),
        events: history.read().await.events().to_vec(),
        total_retries,
        failed_tasks,
        total_tasks,
        files_modified,
    })
}

/// Execute tasks across Jr agents using `futures::stream::buffer_unordered`.
///
/// Tasks are distributed round-robin across available Jr agents.
/// Failed tasks are retried once before being marked as failed.
///
/// Uses per-agent mutexes to allow true parallel execution of tasks
/// assigned to different Jr agents.
async fn execute_tasks_parallel(
    jrs: &mut [TeamAgent],
    tasks: &[Task],
    max_parallel: usize,
    retry_tracker: &RetryTracker,
) -> Vec<TaskResult> {
    if jrs.is_empty() || tasks.is_empty() {
        return Vec::new();
    }

    use futures::stream::{self, StreamExt};

    // We need to own the tasks for the async closure
    let owned_tasks: Vec<(String, String)> = tasks
        .iter()
        .map(|t| (t.id.clone(), t.description.clone()))
        .collect();
    let num_jrs = jrs.len();

    // Clone the retry counter for use in async tasks
    let retry_counter = retry_tracker.clone_counter();

    // Wrap each Jr agent in its own Mutex to allow parallel execution.
    // This avoids the single-mutex bottleneck that would serialize all tasks.
    let jrs: Vec<Arc<tokio::sync::Mutex<TeamAgent>>> = jrs
        .iter_mut()
        .map(|jr| {
            Arc::new(tokio::sync::Mutex::new(std::mem::replace(
                jr,
                TeamAgent::placeholder(),
            )))
        })
        .collect();

    let results: Vec<TaskResult> = stream::iter(owned_tasks.into_iter().enumerate())
        .map(|(i, (task_id, description))| {
            let jrs = jrs.clone();
            let retry_counter = retry_counter.clone();
            async move {
                let jr_idx = i % num_jrs;
                let prompt_text =
                    format!("Execute this task:\n{description}\n\nUse tools as needed.");

                let result = {
                    let jr = jrs[jr_idx].clone();
                    let mut jr_lock = jr.lock().await;
                    jr_lock.prompt(&prompt_text).await
                };

                match result {
                    Ok(output) => TaskResult {
                        task_id,
                        output,
                        success: true,
                    },
                    Err(e) => {
                        tracing::warn!(
                            "[team] Jr[{}] task failed on first attempt: {}. Retrying...",
                            jr_idx,
                            e
                        );
                        // Track the retry
                        retry_counter.fetch_add(1, Ordering::Relaxed);

                        let retry_result = {
                            let jr = jrs[jr_idx].clone();
                            let mut jr_lock = jr.lock().await;
                            jr_lock.prompt(&prompt_text).await
                        };
                        match retry_result {
                            Ok(output) => TaskResult {
                                task_id,
                                output,
                                success: true,
                            },
                            Err(retry_err) => {
                                tracing::error!(
                                    "[team] Jr[{}] task failed after retry: {}",
                                    jr_idx,
                                    retry_err
                                );
                                TaskResult {
                                    task_id,
                                    output: format!(
                                        "Task failed after retry. Last error: {}",
                                        retry_err
                                    ),
                                    success: false,
                                }
                            }
                        }
                    }
                }
            }
        })
        .buffer_unordered(max_parallel)
        .collect()
        .await;

    results
}

/// Extract file paths from Jr task outputs by looking for common
/// file-path patterns in tool-call results.
fn extract_modified_files(results: &[TaskResult]) -> Vec<String> {
    let re = FILE_PATH_REGEX
        .get_or_init(|| Regex::new(r#""path"\s*:\s*"([^"]+)""#).expect("invalid file path regex"));
    let mut files = Vec::new();

    for r in results {
        for cap in re.captures_iter(&r.output) {
            files.push(cap[1].to_string());
        }
    }

    files.sort();
    files.dedup();
    files
}

/// Helper to record an event into shared history.
async fn log_event(history: &Arc<RwLock<TeamHistory>>, role: &str, action: &str, content: &str) {
    let mut h = history.write().await;
    h.add_event(TeamEvent {
        timestamp: chrono::Utc::now(),
        role: role.to_string(),
        action: action.to_string(),
        content: content.to_string(),
        level: EventLevel::default(),
    });
}

/// Log a workflow-phase transition event.
async fn log_phase(history: &Arc<RwLock<TeamHistory>>, phase: WorkflowPhase) {
    let mut h = history.write().await;
    h.add_event(TeamEvent {
        timestamp: chrono::Utc::now(),
        role: "system".to_string(),
        action: format!("phase:{:?}", phase),
        content: format!("entered phase: {}", phase),
        level: EventLevel::Info,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_phase_display() {
        assert_eq!(format!("{}", WorkflowPhase::PmAnalysis), "PM analysis");
        assert_eq!(format!("{}", WorkflowPhase::JrExecution), "Jr execution");
    }

    #[test]
    fn test_workflow_phase_eq() {
        assert_eq!(WorkflowPhase::PmAnalysis, WorkflowPhase::PmAnalysis);
        assert_ne!(WorkflowPhase::PmAnalysis, WorkflowPhase::TlPlan);
    }

    #[test]
    fn test_workflow_phase_serialization() {
        let phase = WorkflowPhase::TlPlan;
        let json = serde_json::to_string(&phase).unwrap();
        let deserialized: WorkflowPhase = serde_json::from_str(&json).unwrap();
        assert_eq!(phase, deserialized);
    }

    #[test]
    fn test_team_result_stats() {
        let result = TeamResult {
            solution: "done".into(),
            duration: Duration::from_secs(10),
            events: vec![],
            total_retries: 2,
            failed_tasks: 1,
            total_tasks: 5,
            files_modified: vec![],
        };
        assert_eq!(result.total_retries, 2);
        assert_eq!(result.failed_tasks, 1);
        assert_eq!(result.total_tasks, 5);
    }

    #[test]
    fn test_team_result_serialization() {
        let result = TeamResult {
            solution: "done".into(),
            duration: Duration::from_secs(1),
            events: vec![],
            total_retries: 0,
            failed_tasks: 0,
            total_tasks: 0,
            files_modified: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: TeamResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.solution, deserialized.solution);
        assert_eq!(result.duration, deserialized.duration);
    }

    #[test]
    fn test_extract_modified_files() {
        let results = vec![
            TaskResult {
                task_id: "1".into(),
                output: r#"Created file: {"path": "src/main.rs"}"#.into(),
                success: true,
            },
            TaskResult {
                task_id: "2".into(),
                output: r#"{"path": "src/lib.rs", "content": "..."}"#.into(),
                success: true,
            },
            TaskResult {
                task_id: "3".into(),
                output: "No files modified".into(),
                success: true,
            },
        ];
        let files = extract_modified_files(&results);
        assert_eq!(files, vec!["src/lib.rs", "src/main.rs"]);
    }

    #[test]
    fn test_extract_modified_files_dedup() {
        let results = vec![
            TaskResult {
                task_id: "1".into(),
                output: r#"{"path": "src/main.rs"}"#.into(),
                success: true,
            },
            TaskResult {
                task_id: "2".into(),
                output: r#"{"path": "src/main.rs"}"#.into(),
                success: true,
            },
        ];
        let files = extract_modified_files(&results);
        assert_eq!(files, vec!["src/main.rs"]);
    }

    #[test]
    fn test_extract_modified_files_empty() {
        let results: Vec<TaskResult> = vec![];
        let files = extract_modified_files(&results);
        assert!(files.is_empty());
    }
}
