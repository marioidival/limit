//! Team workflow — the pipeline that coordinates PM → TL → Jr agents.
//!
//! This module defines the high-level [`TeamWorkflow`] enum that drives
//! the team from request analysis through to delivery.

use crate::error::AgentError;
use crate::team::agent::TeamAgent;
use crate::team::history::{TeamEvent, TeamHistory};
use crate::team::orchestrator::{parse_tasks, Task, TaskResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// The final result produced by a team execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResult {
    /// The PM's delivery summary for the user.
    pub solution: String,
    /// Wall-clock time for the full pipeline.
    pub duration: Duration,
    /// All events recorded during execution.
    pub events: Vec<TeamEvent>,
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
    let analysis = pm
        .prompt(&format!(
            "User request:\n{user_request}\n\nAnalyze this request and identify what needs to be done."
        ))
        .await?;
    log_event(history, "PM", "analysis", &analysis).await;

    // ── Phase 2: TL technical plan ────────────────────────────────────
    tracing::info!("[team] TL — creating technical plan");
    let plan = tl
        .prompt(&format!(
            "PM analysis:\n{analysis}\n\nCreate a technical plan to implement this."
        ))
        .await?;
    log_event(history, "TL", "plan", &plan).await;

    // ── Phase 3: TL task breakdown ────────────────────────────────────
    tracing::info!("[team] TL — breaking down tasks");
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
        });
    }

    // ── Phase 4: Jr parallel execution ────────────────────────────────
    tracing::info!(
        "[team] Jr — executing {} tasks (max {max_parallel} parallel)",
        tasks.len()
    );
    let task_results = execute_tasks_parallel(jrs, &tasks, max_parallel).await;
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

    // ── Phase 5: TL validation ────────────────────────────────────────
    tracing::info!("[team] TL — validating results");
    let validation = tl
        .prompt(&format!(
            "Task results:\n{results_summary}\n\n\
             Validate the implementation. Note any issues or improvements needed."
        ))
        .await?;
    log_event(history, "TL", "validation", &validation).await;

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
    })
}

/// Execute tasks across Jr agents, distributing round-robin.
async fn execute_tasks_parallel(
    jrs: &mut [TeamAgent],
    tasks: &[Task],
    _max_parallel: usize,
) -> Vec<TaskResult> {
    if jrs.is_empty() || tasks.is_empty() {
        return Vec::new();
    }

    let mut task_results = Vec::with_capacity(tasks.len());

    for (i, task) in tasks.iter().enumerate() {
        let jr_idx = i % jrs.len();
        let description = task.description.clone();
        let task_id = task.id.clone();
        let jr = &mut jrs[jr_idx];

        match jr
            .prompt(&format!(
                "Execute this task:\n{description}\n\nUse tools as needed."
            ))
            .await
        {
            Ok(output) => task_results.push(TaskResult {
                task_id,
                output,
                success: true,
            }),
            Err(e) => task_results.push(TaskResult {
                task_id,
                output: format!("Task failed: {e}"),
                success: false,
            }),
        }
    }

    task_results
}

/// Helper to record an event into shared history.
async fn log_event(history: &Arc<RwLock<TeamHistory>>, role: &str, action: &str, content: &str) {
    let mut h = history.write().await;
    h.add_event(TeamEvent {
        timestamp: chrono::Utc::now(),
        role: role.to_string(),
        action: action.to_string(),
        content: content.to_string(),
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
}
