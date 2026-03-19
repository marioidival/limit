//! OrchestratorActor — state machine driving the 6-phase team workflow.
//!
//! Replaces the monolithic `execute_workflow()` with message-based
//! coordination. Each agent call becomes an `ActorRef::send()` + `oneshot::recv()`.
//! Jr tasks run in parallel via `buffer_unordered` without `Arc<Mutex>`.
//! Tasks with dependencies are executed in topological order.

use super::actor::ActorRef;
use super::messages::{
    extract_modified_files, send_progress, truncate, TeamMessage, MAX_CONTEXT_CHARS, MAX_TASKS,
    PHASE_TIMEOUT_SECS,
};
use crate::error::AgentError;
use crate::team::history::{EventLevel, TeamEvent, TeamHistory};
use crate::team::orchestrator::{parse_tasks, Task, TaskResult};
use crate::team::progress::{TaskProgressInfo, TaskProgressStatus, TeamProgressEvent};
use crate::team::workflow::{TeamResult, WorkflowPhase};
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::sync::{mpsc, oneshot, RwLock};

/// The orchestrator drives the 6-phase workflow by sending messages
/// to agent actors and collecting replies via oneshot channels.
pub struct OrchestratorActor {
    pm: ActorRef<TeamMessage>,
    tl: ActorRef<TeamMessage>,
    jrs: Vec<ActorRef<TeamMessage>>,
    max_parallel: usize,
    history: Arc<RwLock<TeamHistory>>,
    progress_tx: Option<mpsc::UnboundedSender<TeamProgressEvent>>,
    result_tx: oneshot::Sender<Result<TeamResult, AgentError>>,
    user_request: String,
    token_input: Arc<AtomicU64>,
    token_output: Arc<AtomicU64>,
    /// Nesting depth for this team (0 = top-level, 1 = child, etc.).
    #[allow(dead_code)]
    nesting: u32,
}

impl OrchestratorActor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pm: ActorRef<TeamMessage>,
        tl: ActorRef<TeamMessage>,
        jrs: Vec<ActorRef<TeamMessage>>,
        max_parallel: usize,
        history: Arc<RwLock<TeamHistory>>,
        progress_tx: Option<mpsc::UnboundedSender<TeamProgressEvent>>,
        result_tx: oneshot::Sender<Result<TeamResult, AgentError>>,
        user_request: String,
        token_input: Arc<AtomicU64>,
        token_output: Arc<AtomicU64>,
    ) -> Self {
        Self {
            pm,
            tl,
            jrs,
            max_parallel,
            history,
            progress_tx,
            result_tx,
            user_request,
            token_input,
            token_output,
            nesting: 0,
        }
    }

    /// Send a message and await the reply via oneshot.
    async fn ask_pm(
        &self,
        msg: TeamMessage,
        rx: oneshot::Receiver<Result<String, AgentError>>,
    ) -> Result<String, AgentError> {
        let start = std::time::Instant::now();
        self.pm
            .send(msg)
            .await
            .map_err(|e| AgentError::ActorError(format!("PM mailbox closed: {e}")))?;
        let result = tokio::time::timeout(std::time::Duration::from_secs(PHASE_TIMEOUT_SECS), rx)
            .await
            .map_err(|_| AgentError::PhaseTimeout {
                phase: "PM".to_string(),
                seconds: PHASE_TIMEOUT_SECS,
            })?
            .map_err(|_| AgentError::ActorError("PM reply channel dropped".into()))?;
        tracing::info!(
            "[orchestrator] PM replied in {:?} ({} chars)",
            start.elapsed(),
            result.as_ref().map(|s| s.len()).unwrap_or(0)
        );
        result
    }

    async fn ask_tl(
        &self,
        msg: TeamMessage,
        rx: oneshot::Receiver<Result<String, AgentError>>,
    ) -> Result<String, AgentError> {
        let start = std::time::Instant::now();
        self.tl
            .send(msg)
            .await
            .map_err(|e| AgentError::ActorError(format!("TL mailbox closed: {e}")))?;
        let result = tokio::time::timeout(std::time::Duration::from_secs(PHASE_TIMEOUT_SECS), rx)
            .await
            .map_err(|_| AgentError::PhaseTimeout {
                phase: "TL".to_string(),
                seconds: PHASE_TIMEOUT_SECS,
            })?
            .map_err(|_| AgentError::ActorError("TL reply channel dropped".into()))?;
        tracing::info!(
            "[orchestrator] TL replied in {:?} ({} chars)",
            start.elapsed(),
            result.as_ref().map(|s| s.len()).unwrap_or(0)
        );
        result
    }

    /// Execute the full 6-phase workflow.
    pub async fn run_workflow(mut self) {
        let start = std::time::Instant::now();
        let result = self.execute().await;
        match &result {
            Ok(r) => {
                tracing::info!(
                    "[orchestrator] workflow completed in {:?} — {} tasks ({} failed), {} files modified, {}in/{}out tokens",
                    start.elapsed(),
                    r.total_tasks,
                    r.failed_tasks,
                    r.files_modified.len(),
                    self.token_input.load(Ordering::Relaxed),
                    self.token_output.load(Ordering::Relaxed),
                );
            }
            Err(e) => {
                tracing::error!(
                    "[orchestrator] workflow failed after {:?}: {}",
                    start.elapsed(),
                    e
                );
            }
        }
        let _ = self.result_tx.send(result);
    }

    async fn log_event(&self, role: &str, action: &str, content: &str) {
        let mut h = self.history.write().await;
        h.add_event(TeamEvent {
            timestamp: chrono::Utc::now(),
            role: role.to_string(),
            action: action.to_string(),
            content: content.to_string(),
            level: EventLevel::default(),
        });
    }

    async fn log_phase(&self, phase: WorkflowPhase) {
        let mut h = self.history.write().await;
        h.add_event(TeamEvent {
            timestamp: chrono::Utc::now(),
            role: "system".to_string(),
            action: format!("phase:{:?}", phase),
            content: format!("entered phase: {}", phase),
            level: EventLevel::Info,
        });
    }

    async fn execute(&mut self) -> Result<TeamResult, AgentError> {
        let start = Instant::now();

        // ── Phase 1: PM analysis ──────────────────────────────────────
        tracing::info!("[team] PM — analyzing request");
        self.log_phase(WorkflowPhase::PmAnalysis).await;
        send_progress(
            &self.progress_tx,
            TeamProgressEvent::PhaseChanged {
                phase: WorkflowPhase::PmAnalysis,
                completed: 0,
                nesting: 0,
            },
        );

        let (tx, rx) = oneshot::channel();
        let analysis = self
            .ask_pm(
                TeamMessage::PmAnalyze {
                    request: self.user_request.clone(),
                    reply: tx,
                },
                rx,
            )
            .await?;

        send_progress(
            &self.progress_tx,
            TeamProgressEvent::StatusUpdate {
                message: truncate(&analysis, 200),
                nesting: 0,
            },
        );
        self.log_event("PM", "analysis", &analysis).await;

        // ── Phase 2: TL technical plan ───────────────────────────────
        tracing::info!("[team] TL — creating technical plan");
        self.log_phase(WorkflowPhase::TlPlan).await;
        send_progress(
            &self.progress_tx,
            TeamProgressEvent::PhaseChanged {
                phase: WorkflowPhase::TlPlan,
                completed: 1,
                nesting: 0,
            },
        );

        let (tx, rx) = oneshot::channel();
        let analysis_clone = analysis.clone();
        let plan = self
            .ask_tl(
                TeamMessage::TlPlan {
                    analysis,
                    reply: tx,
                },
                rx,
            )
            .await?;

        // Guard against empty/exploratory TL plans
        let plan_for_breakdown = if plan.len() < 200 || plan.to_lowercase().contains("explore") {
            tracing::warn!(
                "[team] TL plan too short ({}) or exploratory — using PM analysis for breakdown",
                plan.len()
            );
            analysis_clone
        } else {
            plan.clone()
        };

        send_progress(
            &self.progress_tx,
            TeamProgressEvent::StatusUpdate {
                message: truncate(&plan, 200),
                nesting: 0,
            },
        );
        self.log_event("TL", "plan", &plan).await;

        // ── Phase 3: TL task breakdown ───────────────────────────────
        tracing::info!("[team] TL — breaking down tasks");
        self.log_phase(WorkflowPhase::TlBreakdown).await;
        send_progress(
            &self.progress_tx,
            TeamProgressEvent::PhaseChanged {
                phase: WorkflowPhase::TlBreakdown,
                completed: 2,
                nesting: 0,
            },
        );

        let (tx, rx) = oneshot::channel();
        let tasks_text = self
            .ask_tl(
                TeamMessage::TlBreakdown {
                    plan: plan_for_breakdown,
                    reply: tx,
                },
                rx,
            )
            .await?;

        send_progress(
            &self.progress_tx,
            TeamProgressEvent::StatusUpdate {
                message: truncate(&tasks_text, 200),
                nesting: 0,
            },
        );

        let mut tasks: Vec<Task> = parse_tasks(&tasks_text);
        let original_count = tasks.len();
        tasks.truncate(MAX_TASKS);
        if tasks.len() < original_count {
            tracing::warn!(
                "[team] TL produced {} tasks, truncated to {}",
                original_count,
                tasks.len()
            );
        }

        // Truncate CONTEXT blocks in task descriptions
        for task in &mut tasks {
            task.description = truncate_context(&task.description, MAX_CONTEXT_CHARS);
        }

        self.log_event(
            "TL",
            "tasks",
            &format!("{} tasks (of {} parsed)", tasks.len(), original_count),
        )
        .await;

        // Send initial task list to TUI
        send_progress(
            &self.progress_tx,
            TeamProgressEvent::TasksUpdate {
                tasks: tasks
                    .iter()
                    .map(|t| TaskProgressInfo {
                        id: t.id.clone(),
                        description: t.description.clone(),
                        status: TaskProgressStatus::Pending,
                        agent_index: None,
                        sub_status: String::new(),
                    })
                    .collect(),
                nesting: 0,
            },
        );

        // ── No tasks: deliver plan as-is ─────────────────────────────
        if tasks.is_empty() {
            tracing::warn!("[team] TL produced no parseable tasks — delivering plan as-is");

            let (tx, rx) = oneshot::channel();
            let delivery = self
                .ask_pm(TeamMessage::PmDeliverNoTasks { plan, reply: tx }, rx)
                .await?;
            self.log_event("PM", "delivery", &delivery).await;

            send_progress(
                &self.progress_tx,
                TeamProgressEvent::Finished {
                    success: true,
                    nesting: 0,
                },
            );

            return Ok(TeamResult {
                solution: delivery,
                duration: start.elapsed(),
                events: self.history.read().await.events().to_vec(),
                total_retries: 0,
                failed_tasks: 0,
                total_tasks: 0,
                files_modified: vec![],
                tokens_input: 0,
                tokens_output: 0,
            });
        }

        // ── Phase 4: Jr parallel execution ───────────────────────────
        self.log_phase(WorkflowPhase::JrExecution).await;
        send_progress(
            &self.progress_tx,
            TeamProgressEvent::PhaseChanged {
                phase: WorkflowPhase::JrExecution,
                completed: 3,
                nesting: 0,
            },
        );
        tracing::info!(
            "[team] Jr — executing {} tasks (max {} parallel)",
            tasks.len(),
            self.max_parallel
        );

        let task_results = self.execute_tasks_parallel(&tasks, self.max_parallel).await;

        let failed_tasks = task_results.iter().filter(|r| !r.success).count();
        let total_tasks = task_results.len();
        let files_modified = extract_modified_files(&task_results);

        // ── Build command suggestion (TL-suggested) ──────────────────
        tracing::info!("[team] asking TL for build verification command");
        let (tx, rx) = oneshot::channel();
        let build_cmd_response = self
            .ask_tl(
                TeamMessage::TlSuggestBuildCommand {
                    files_modified: files_modified.clone(),
                    reply: tx,
                },
                rx,
            )
            .await?;

        let build_cmd = build_cmd_response.trim().to_string();
        if build_cmd.is_empty() || build_cmd == "NONE" {
            tracing::info!("[team] TL suggested no build command");
        } else {
            tracing::info!("[team] TL suggested build command: {}", build_cmd);
        }

        let results_summary: String = task_results
            .iter()
            .map(|r| {
                if r.success {
                    format!("[OK] {}", r.output)
                } else {
                    format!("[FAIL] {}", r.output)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.log_event("Jr", "execution", &results_summary).await;

        // ── Phase 5: TL validation ───────────────────────────────────
        self.log_phase(WorkflowPhase::TlValidation).await;
        send_progress(
            &self.progress_tx,
            TeamProgressEvent::PhaseChanged {
                phase: WorkflowPhase::TlValidation,
                completed: 4,
                nesting: 0,
            },
        );
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

        let (tx, rx) = oneshot::channel();
        let validation = self
            .ask_tl(
                TeamMessage::TlValidate {
                    results_summary: results_summary.clone(),
                    files_list,
                    build_cmd: build_cmd.clone(),
                    reply: tx,
                },
                rx,
            )
            .await?;

        let validation_failures = parse_validation_failures(&validation);
        if validation_failures > 0 {
            tracing::warn!("[team] TL validation found {} FAIL(s)", validation_failures);
        }

        send_progress(
            &self.progress_tx,
            TeamProgressEvent::StatusUpdate {
                message: truncate(&validation, 200),
                nesting: 0,
            },
        );
        self.log_event("TL", "validation", &validation).await;

        // ── Phase 6: PM delivery ─────────────────────────────────────
        self.log_phase(WorkflowPhase::PmDelivery).await;
        send_progress(
            &self.progress_tx,
            TeamProgressEvent::PhaseChanged {
                phase: WorkflowPhase::PmDelivery,
                completed: 5,
                nesting: 0,
            },
        );
        tracing::info!("[team] PM — preparing delivery");

        // Build failure context for PM delivery
        let failure_context = if validation_failures > 0 {
            format!(
                "\n\n**WARNING: {} task(s) FAILED validation.** These tasks need to be re-done.",
                validation_failures
            )
        } else {
            String::new()
        };

        let (tx, rx) = oneshot::channel();
        let delivery = self
            .ask_pm(
                TeamMessage::PmDeliver {
                    validation: format!("{}{}", validation, failure_context),
                    results_summary,
                    files_modified: files_modified.clone(),
                    reply: tx,
                },
                rx,
            )
            .await?;

        send_progress(
            &self.progress_tx,
            TeamProgressEvent::StatusUpdate {
                message: truncate(&delivery, 200),
                nesting: 0,
            },
        );
        self.log_event("PM", "delivery", &delivery).await;

        let total_failures = failed_tasks + validation_failures;

        send_progress(
            &self.progress_tx,
            TeamProgressEvent::Finished {
                success: total_failures == 0,
                nesting: 0,
            },
        );

        Ok(TeamResult {
            solution: delivery,
            duration: start.elapsed(),
            events: self.history.read().await.events().to_vec(),
            total_retries: 0,
            failed_tasks: total_failures,
            total_tasks,
            files_modified,
            tokens_input: 0,
            tokens_output: 0,
        })
    }

    /// Execute tasks with topological dependency ordering.
    ///
    /// Tasks without dependencies run in parallel (up to `max_parallel`).
    /// Tasks with dependencies wait until all their deps complete.
    /// Failed tasks (including tool-limit hits) are retried once.
    async fn execute_tasks_parallel(&self, tasks: &[Task], max_parallel: usize) -> Vec<TaskResult> {
        if self.jrs.is_empty() || tasks.is_empty() {
            return Vec::new();
        }

        let num_jrs = self.jrs.len();
        let jrs = self.jrs.clone();
        let progress_tx = self.progress_tx.clone();

        // Build a lookup map: task_id -> (task, original_index)
        let task_map: HashMap<String, (Task, usize)> = tasks
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id.clone(), (t.clone(), i)))
            .collect();

        let mut completed: HashSet<String> = HashSet::new();
        let mut remaining: HashSet<String> = tasks.iter().map(|t| t.id.clone()).collect();
        let mut results: HashMap<String, TaskResult> = HashMap::new();

        while !remaining.is_empty() {
            // Find tasks whose dependencies are all satisfied
            let ready: Vec<(Task, usize)> = remaining
                .iter()
                .filter_map(|id| {
                    let (task, idx) = task_map.get(id)?;
                    let deps_met = task.depends_on.iter().all(|dep| completed.contains(dep));
                    deps_met.then(|| (task.clone(), *idx))
                })
                .collect();

            if ready.is_empty() {
                // Deadlock — remaining tasks have unresolvable dependencies
                tracing::warn!(
                    "[team] deadlock detected: {} tasks with unmet dependencies",
                    remaining.len()
                );
                for id in &remaining {
                    results.insert(
                        id.clone(),
                        TaskResult {
                            task_id: id.clone(),
                            output: "Task skipped: unresolvable dependencies".into(),
                            success: false,
                            hit_tool_limit: false,
                            files_modified: vec![],
                        },
                    );
                }
                break;
            }

            // Execute ready tasks in parallel (up to max_parallel)
            use futures::stream::{self, StreamExt};

            let batch_size = ready.len();
            tracing::info!(
                "[orchestrator] executing batch of {} task(s) ({} remaining)",
                batch_size,
                remaining.len()
            );
            let batch_start = std::time::Instant::now();
            let batch_results: Vec<TaskResult> =
                stream::iter(ready.into_iter().enumerate())
                    .map(|(batch_i, (task, orig_idx))| {
                        let _ = batch_i; // used for logging if needed
                        let jrs = jrs.clone();
                        let progress_tx = progress_tx.clone();
                        async move {
                            let jr_idx = orig_idx % num_jrs;
                            let task_id = task.id.clone();

                            send_progress(
                                &progress_tx,
                                TeamProgressEvent::TaskStarted {
                                    task_id: task_id.clone(),
                                    agent_index: jr_idx,
                                nesting: 0,
                                },
                            );

                            let task_result = {
                                let (tx, rx) = oneshot::channel();
                                if let Err(e) = jrs[jr_idx]
                                    .send(TeamMessage::JrExecute {
                                        task: task.clone(),
                                        reply: tx,
                                        progress_tx: self.progress_tx.clone(),
                                    })
                                    .await
                                {
                                    TaskResult {
                                        task_id: task_id.clone(),
                                        output: format!("Jr mailbox closed: {e}"),
                                        success: false,
                                        hit_tool_limit: false,
                                        files_modified: vec![],
                                    }
                                } else {
                                    match rx.await {
                                        Ok(result) => result,
                                        Err(_) => TaskResult {
                                            task_id: task_id.clone(),
                                            output: "Jr reply channel dropped".into(),
                                            success: false,
                                            hit_tool_limit: false,
                                            files_modified: vec![],
                                        },
                                    }
                                }
                            };

                            let success = task_result.success;
                            if !success {
                                tracing::warn!(
                                    "[team] Jr[{}] task failed on first attempt. Retrying...",
                                    jr_idx
                                );

                                // Retry once, with context from previous attempt if tool-limited
                                let retry_hint = if task_result.hit_tool_limit {
                                    format!(
                                        "\n\nPrevious attempt was interrupted. Continue from where you left off: {}",
                                        task_result.output
                                    )
                                } else {
                                    String::new()
                                };

                                let (tx, rx) = oneshot::channel();
                                let retry_task = Task {
                                    id: task_id.clone(),
                                    description: format!("{}{}", task.description, retry_hint),
                                    status: task.status.clone(),
                                    depends_on: task.depends_on.clone(),
                                };
                                let retry_result =
                                    if let Err(e) = jrs[jr_idx]
                                        .send(TeamMessage::JrExecute {
                                            task: retry_task,
                                            reply: tx,
                                            progress_tx: self.progress_tx.clone(),
                                        })
                                        .await
                                    {
                                        TaskResult {
                                            task_id: task_id.clone(),
                                            output: format!("Jr mailbox closed on retry: {e}"),
                                            success: false,
                                            hit_tool_limit: false,
                                            files_modified: vec![],
                                        }
                                    } else {
                                        match rx.await {
                                            Ok(result) => result,
                                            Err(_) => TaskResult {
                                                task_id: task_id.clone(),
                                                output: "Jr reply channel dropped on retry"
                                                    .into(),
                                                success: false,
                                                hit_tool_limit: false,
                                                files_modified: vec![],
                                            },
                                        }
                                    };

                                send_progress(
                                    &progress_tx,
                                    TeamProgressEvent::TaskCompleted {
                                        task_id: task_id.clone(),
                                        success: retry_result.success,
                                    nesting: 0,
                                    },
                                );
                                retry_result
                            } else {
                                send_progress(
                                    &progress_tx,
                                    TeamProgressEvent::TaskCompleted {
                                        task_id: task_id.clone(),
                                        success,
                                    nesting: 0,
                                    },
                                );
                                task_result
                            }
                        }
                    })
                    .buffer_unordered(max_parallel.max(1).min(batch_size))
                    .collect()
                    .await;

            tracing::info!(
                "[orchestrator] batch completed in {:?} ({} results)",
                batch_start.elapsed(),
                batch_results.len()
            );

            // Mark completed and collect results
            for r in &batch_results {
                completed.insert(r.task_id.clone());
                remaining.remove(&r.task_id);
                results.insert(r.task_id.clone(), r.clone());
            }
        }

        // Return results in original task order
        tasks.iter().filter_map(|t| results.remove(&t.id)).collect()
    }
}

/// Truncate CONTEXT blocks within a task description to `max_chars`.
fn truncate_context(description: &str, max_chars: usize) -> String {
    if description.len() <= max_chars {
        return description.to_string();
    }

    // Find CONTEXT: blocks and truncate their content
    let context_marker = "\nCONTEXT:\n";
    if let Some(pos) = description.find(context_marker) {
        let prefix = &description[..pos + context_marker.len()];
        let content = &description[pos + context_marker.len()..];

        let remaining = max_chars.saturating_sub(prefix.len());
        if remaining == 0 {
            return description[..pos].to_string();
        }

        // Truncate at a character boundary
        let end = content
            .char_indices()
            .take_while(|(i, _)| *i < remaining)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(remaining.min(content.len()));

        format!("{}{}...(truncated)", prefix, &content[..end])
    } else {
        description.to_string()
    }
}

/// Count **FAIL** entries in TL validation output.
fn parse_validation_failures(validation: &str) -> usize {
    validation
        .lines()
        .filter(|l| l.contains("Status:") && l.contains("**FAIL**"))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_context_no_context() {
        let desc = "Create a file with some content";
        assert_eq!(truncate_context(desc, 100), desc);
    }

    #[test]
    fn test_truncate_context_short() {
        let desc = "TASK: Do something\nCONTEXT:\nshort content";
        assert_eq!(truncate_context(desc, 100), desc);
    }

    #[test]
    fn test_truncate_context_long() {
        let long_content = "x".repeat(10000);
        let desc = format!("TASK: Do something\nCONTEXT:\n{}", long_content);
        let result = truncate_context(&desc, 200);
        assert!(result.contains("..."));
        assert!(result.len() < desc.len());
    }

    #[test]
    fn test_truncate_context_preserves_prefix() {
        let desc = "TASK: Create auth module\nCONTEXT:\n```rust\npub mod auth;\n```";
        let result = truncate_context(desc, 100);
        assert!(result.starts_with("TASK: Create auth module\nCONTEXT:\n"));
    }

    #[test]
    fn test_parse_validation_failures_none() {
        let v = "## Task: 1\n- Status: **PASS**\n- Reason: all good";
        assert_eq!(parse_validation_failures(v), 0);
    }

    #[test]
    fn test_parse_validation_failures_some() {
        let v = "## Task: 1\n- Status: **FAIL**\n- Reason: bad\n\n## Task: 2\n- Status: **PASS**\n- Reason: ok\n\n## Task: 3\n- Status: **FAIL**\n- Reason: also bad";
        assert_eq!(parse_validation_failures(v), 2);
    }
}
