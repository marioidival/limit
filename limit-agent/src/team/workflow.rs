//! Team workflow — the pipeline that coordinates PM → TL → Jr agents.
//!
//! This module defines the [`TeamResult`] and [`WorkflowPhase`] types
//! and the legacy [`execute_workflow`] function (deprecated in favor
//! of the actor-based system in [`crate::team::orchestrator_actor`]).

use crate::error::AgentError;
use crate::team::agent::TeamAgent;
use crate::team::history::{EventLevel, TeamEvent, TeamHistory};
use crate::team::messages::{extract_modified_files, send_progress, truncate};
use crate::team::orchestrator::{parse_tasks, Task, TaskResult};
use crate::team::progress::{TaskProgressInfo, TaskProgressStatus, TeamProgressEvent};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};

/// Maximum conversation history per agent to prevent unbounded growth.
const MAX_HISTORY_PER_AGENT: usize = 50;
/// Maximum tasks from TL breakdown. Prevents over-decomposition.
const MAX_TASKS: usize = 10;

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
    /// Total input tokens across all phases.
    pub tokens_input: u64,
    /// Total output tokens across all phases.
    pub tokens_output: u64,
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
///
/// **Deprecated**: Use the actor-based system via [`Team::execute`](super::Team::execute) instead.
#[deprecated(note = "Use the actor-based Team::execute() instead")]
pub async fn execute_workflow(
    pm: &mut TeamAgent,
    tl: &mut TeamAgent,
    jrs: &mut [TeamAgent],
    user_request: &str,
    history: &Arc<RwLock<TeamHistory>>,
    max_parallel: usize,
    progress_tx: Option<mpsc::UnboundedSender<TeamProgressEvent>>,
) -> Result<TeamResult, AgentError> {
    let start = Instant::now();

    // ── Phase 1: PM analysis ──────────────────────────────────────────
    let _pm_span = tracing::info_span!("pm_analysis").entered();
    tracing::info!("[team] PM — analyzing request");
    log_phase(history, WorkflowPhase::PmAnalysis).await;
    send_progress(
        &progress_tx,
        TeamProgressEvent::PhaseChanged {
            phase: WorkflowPhase::PmAnalysis,
            completed: 0,
        },
    );
    let analysis = pm
        .prompt(&format!(
            "User request:\n{user_request}\n\nAnalyze this request and identify what needs to be done."
        ))
        .await?
        .text;
    send_progress(
        &progress_tx,
        TeamProgressEvent::StatusUpdate {
            message: truncate(&analysis, 100),
        },
    );
    log_event(history, "PM", "analysis", &analysis).await;
    drop(_pm_span);

    // ── Phase 2: TL technical plan ────────────────────────────────────
    let _tl_span = tracing::info_span!("tl_plan").entered();
    tracing::info!("[team] TL — creating technical plan");
    log_phase(history, WorkflowPhase::TlPlan).await;
    send_progress(
        &progress_tx,
        TeamProgressEvent::PhaseChanged {
            phase: WorkflowPhase::TlPlan,
            completed: 1,
        },
    );
    let plan = tl
        .prompt(&format!(
            "PM analysis:\n{analysis}\n\nCreate a technical plan to implement this."
        ))
        .await?
        .text;
    send_progress(
        &progress_tx,
        TeamProgressEvent::StatusUpdate {
            message: truncate(&plan, 100),
        },
    );
    log_event(history, "TL", "plan", &plan).await;
    drop(_tl_span);

    // ── Phase 3: TL task breakdown ────────────────────────────────────
    let _breakdown_span = tracing::info_span!("tl_breakdown").entered();
    tracing::info!("[team] TL — breaking down tasks");
    log_phase(history, WorkflowPhase::TlBreakdown).await;
    send_progress(
        &progress_tx,
        TeamProgressEvent::PhaseChanged {
            phase: WorkflowPhase::TlBreakdown,
            completed: 2,
        },
    );
    let tasks = tl
        .prompt(&format!(
            "Technical plan:\n{plan}\n\nBreak this down into at most {MAX_TASKS} specific, \
             executable tasks. Each task should be self-contained and independently \
             completable by a junior developer. Combine small steps into single tasks. \
             Format each task on its own line as:\nTASK: <description>"
        ))
        .await?
        .text;
    send_progress(
        &progress_tx,
        TeamProgressEvent::StatusUpdate {
            message: truncate(&tasks, 100),
        },
    );
    let mut tasks: Vec<Task> = parse_tasks(&tasks);
    let original_count = tasks.len();
    tasks.truncate(MAX_TASKS);
    if tasks.len() < original_count {
        tracing::warn!(
            "[team] TL produced {} tasks, truncated to {}",
            original_count,
            tasks.len()
        );
    }
    log_event(
        history,
        "TL",
        "tasks",
        &format!("{} tasks (of {} parsed)", tasks.len(), original_count),
    )
    .await;
    drop(_breakdown_span);

    // Send initial task list to TUI
    send_progress(
        &progress_tx,
        TeamProgressEvent::TasksUpdate {
            tasks: tasks
                .iter()
                .map(|t| TaskProgressInfo {
                    id: t.id.clone(),
                    description: t.description.clone(),
                    status: TaskProgressStatus::Pending,
                    agent_index: None,
                })
                .collect(),
        },
    );

    if tasks.is_empty() {
        tracing::warn!("[team] TL produced no parseable tasks — delivering plan as-is");
        let _delivery_span = tracing::info_span!("pm_delivery_no_tasks").entered();
        let delivery = pm
            .prompt(&format!(
                "The Tech Lead produced a plan but no specific tasks. Here is the plan:\n\n{plan}\n\n\
                 Summarize this for the user and suggest next steps."
            ))
            .await?
            .text;
        log_event(history, "PM", "delivery", &delivery).await;

        send_progress(&progress_tx, TeamProgressEvent::Finished { success: true });

        return Ok(TeamResult {
            solution: delivery,
            duration: start.elapsed(),
            events: history.read().await.events().to_vec(),
            total_retries: 0,
            failed_tasks: 0,
            total_tasks: 0,
            files_modified: vec![],
            tokens_input: 0,
            tokens_output: 0,
        });
    }

    // ── Phase 4: Jr parallel execution ────────────────────────────────
    let _jr_span = tracing::info_span!("jr_execution", tasks = tasks.len()).entered();
    log_phase(history, WorkflowPhase::JrExecution).await;
    send_progress(
        &progress_tx,
        TeamProgressEvent::PhaseChanged {
            phase: WorkflowPhase::JrExecution,
            completed: 3,
        },
    );
    tracing::info!(
        "[team] Jr — executing {} tasks (max {max_parallel} parallel)",
        tasks.len()
    );
    let retry_tracker = RetryTracker::new();
    let task_results =
        execute_tasks_parallel(jrs, &tasks, max_parallel, &retry_tracker, &progress_tx).await;

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
    drop(_jr_span);

    log_phase(history, WorkflowPhase::TlValidation).await;
    send_progress(
        &progress_tx,
        TeamProgressEvent::PhaseChanged {
            phase: WorkflowPhase::TlValidation,
            completed: 4,
        },
    );
    // ── Phase 5: TL validation ────────────────────────────────────────
    let _validation_span = tracing::info_span!("tl_validation").entered();
    tracing::info!("[team] TL — validating results");
    let files_list = if files_modified.is_empty() {
        "No file paths were extracted from task results.".to_string()
    } else {
        format!(
            "Files reported as modified:\n{}",
            files_modified
                .iter()
                .map(|f| format!("- {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let validation = tl
        .prompt(&format!(
            "Task results:\n{results_summary}\n\n\
             {files_list}\n\n\
             Validate the implementation based on the task results above. \
             Do NOT use tools — just analyze the results and note any issues."
        ))
        .await?
        .text;
    send_progress(
        &progress_tx,
        TeamProgressEvent::StatusUpdate {
            message: truncate(&validation, 100),
        },
    );
    log_event(history, "TL", "validation", &validation).await;
    drop(_validation_span);

    log_phase(history, WorkflowPhase::PmDelivery).await;
    send_progress(
        &progress_tx,
        TeamProgressEvent::PhaseChanged {
            phase: WorkflowPhase::PmDelivery,
            completed: 5,
        },
    );
    // ── Phase 6: PM delivery ──────────────────────────────────────────
    let _delivery_span = tracing::info_span!("pm_delivery").entered();
    tracing::info!("[team] PM — preparing delivery");
    let delivery = pm
        .prompt(&format!(
            "Technical solution validated by TL:\n{validation}\n\n\
             Task results:\n{results_summary}\n\n\
             Files modified:\n{}\n\n\
             Summarize what was done: list the files created/modified and the outcome. \
             Only suggest follow-up actions if something is genuinely incomplete or broken. \
             Do NOT suggest improvements, refactors, or enhancements.",
            files_modified
                .iter()
                .map(|f| format!("- {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
        .await?
        .text;
    send_progress(
        &progress_tx,
        TeamProgressEvent::StatusUpdate {
            message: truncate(&delivery, 100),
        },
    );
    log_event(history, "PM", "delivery", &delivery).await;

    send_progress(&progress_tx, TeamProgressEvent::Finished { success: true });

    Ok(TeamResult {
        solution: delivery,
        duration: start.elapsed(),
        events: history.read().await.events().to_vec(),
        total_retries,
        failed_tasks,
        total_tasks,
        files_modified,
        tokens_input: 0,
        tokens_output: 0,
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
    progress_tx: &Option<mpsc::UnboundedSender<TeamProgressEvent>>,
) -> Vec<TaskResult> {
    if jrs.is_empty() || tasks.is_empty() {
        return Vec::new();
    }

    use futures::stream::{self, StreamExt};

    let owned_tasks: Vec<(String, String)> = tasks
        .iter()
        .map(|t| (t.id.clone(), t.description.clone()))
        .collect();
    let num_jrs = jrs.len();

    let retry_counter = retry_tracker.clone_counter();

    let progress_tx = progress_tx.clone();

    // Wrap each Jr agent in its own Mutex to allow parallel execution.
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
            let progress_tx = progress_tx.clone();
            async move {
                let jr_idx = i % num_jrs;

                send_progress(
                    &progress_tx,
                    TeamProgressEvent::TaskStarted {
                        task_id: task_id.clone(),
                        agent_index: jr_idx,
                    },
                );

                let prompt_text = format!(
                    "Execute this task:\n{description}\n\n\
                     Do the work efficiently. Use the minimum number of tool calls needed. \
                     Do not verify or re-read files after writing them — trust the tool results."
                );

                let result = {
                    let jr = jrs[jr_idx].clone();
                    let mut jr_lock = jr.lock().await;
                    jr_lock.prompt(&prompt_text).await
                };

                let task_result = match result {
                    Ok(pr) => {
                        let success = true;
                        send_progress(
                            &progress_tx,
                            TeamProgressEvent::TaskCompleted {
                                task_id: task_id.clone(),
                                success,
                            },
                        );
                        TaskResult {
                            task_id,
                            output: pr.text,
                            success,
                            hit_tool_limit: pr.hit_tool_limit,
                            files_modified: pr.files_modified,
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "[team] Jr[{}] task failed on first attempt: {}. Retrying...",
                            jr_idx,
                            e
                        );
                        retry_counter.fetch_add(1, Ordering::Relaxed);

                        let retry_result = {
                            let jr = jrs[jr_idx].clone();
                            let mut jr_lock = jr.lock().await;
                            jr_lock.prompt(&prompt_text).await
                        };
                        match retry_result {
                            Ok(pr) => {
                                send_progress(
                                    &progress_tx,
                                    TeamProgressEvent::TaskCompleted {
                                        task_id: task_id.clone(),
                                        success: true,
                                    },
                                );
                                TaskResult {
                                    task_id,
                                    output: pr.text,
                                    success: true,
                                    hit_tool_limit: pr.hit_tool_limit,
                                    files_modified: pr.files_modified,
                                }
                            }
                            Err(retry_err) => {
                                tracing::error!(
                                    "[team] Jr[{}] task failed after retry: {}",
                                    jr_idx,
                                    retry_err
                                );
                                send_progress(
                                    &progress_tx,
                                    TeamProgressEvent::TaskCompleted {
                                        task_id: task_id.clone(),
                                        success: false,
                                    },
                                );
                                TaskResult {
                                    task_id,
                                    output: format!(
                                        "Task failed after retry. Last error: {}",
                                        retry_err
                                    ),
                                    success: false,
                                    hit_tool_limit: false,
                                    files_modified: vec![],
                                }
                            }
                        }
                    }
                };

                task_result
            }
        })
        .buffer_unordered(max_parallel)
        .collect()
        .await;

    results
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
            tokens_input: 100,
            tokens_output: 200,
        };
        assert_eq!(result.total_retries, 2);
        assert_eq!(result.failed_tasks, 1);
        assert_eq!(result.total_tasks, 5);
        assert_eq!(result.tokens_input, 100);
        assert_eq!(result.tokens_output, 200);
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
            tokens_input: 0,
            tokens_output: 0,
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
                hit_tool_limit: false,
                files_modified: vec!["src/main.rs".into()],
            },
            TaskResult {
                task_id: "2".into(),
                output: r#"{"path": "src/lib.rs", "content": "..."}"#.into(),
                success: true,
                hit_tool_limit: false,
                files_modified: vec!["src/lib.rs".into()],
            },
            TaskResult {
                task_id: "3".into(),
                output: "No files modified".into(),
                success: true,
                hit_tool_limit: false,
                files_modified: vec![],
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
                hit_tool_limit: false,
                files_modified: vec!["src/main.rs".into()],
            },
            TaskResult {
                task_id: "2".into(),
                output: r#"{"path": "src/main.rs"}"#.into(),
                success: true,
                hit_tool_limit: false,
                files_modified: vec!["src/main.rs".into()],
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

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 100), "hello");
    }

    #[test]
    fn test_truncate_exact() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let s = "abcdefghij".repeat(12);
        let result = truncate(&s, 50);
        assert!(result.ends_with("..."));
        assert!(result.len() < s.len());
    }

    #[test]
    fn test_truncate_breaks_at_newline() {
        let s = "line one\nline two\nline three";
        let result = truncate(s, 20);
        assert!(result.contains("line one"));
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate("", 100), "");
    }
}
