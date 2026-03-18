// Message protocol for the actor-based team system.
//!
//! Defines all message types flowing between actors, orchestrator commands,
//! and supervisor lifecycle messages. Also contains shared workflow helpers
//! used by both the legacy workflow and the new actor system.

use crate::error::AgentError;
use crate::team::orchestrator::{Task, TaskResult};
use crate::team::progress::TeamProgressEvent;
use tokio::sync::{mpsc, oneshot};

/// Messages sent from the Orchestrator to agent actors.
///
/// Each variant carries a `oneshot::Sender` for the request-response
/// pattern — the actor sends its reply back through the channel.
pub enum TeamMessage {
    // PM
    PmAnalyze {
        request: String,
        reply: oneshot::Sender<Result<String, AgentError>>,
    },
    PmDeliver {
        validation: String,
        results_summary: String,
        files_modified: Vec<String>,
        reply: oneshot::Sender<Result<String, AgentError>>,
    },
    PmDeliverNoTasks {
        plan: String,
        reply: oneshot::Sender<Result<String, AgentError>>,
    },
    // TL
    TlPlan {
        analysis: String,
        reply: oneshot::Sender<Result<String, AgentError>>,
    },
    TlBreakdown {
        plan: String,
        reply: oneshot::Sender<Result<String, AgentError>>,
    },
    TlValidate {
        results_summary: String,
        files_list: String,
        build_cmd: String,
        reply: oneshot::Sender<Result<String, AgentError>>,
    },
    TlSuggestBuildCommand {
        files_modified: Vec<String>,
        reply: oneshot::Sender<Result<String, AgentError>>,
    },
    // Jr
    JrExecute {
        task: Task,
        reply: oneshot::Sender<TaskResult>,
        progress_tx: Option<mpsc::UnboundedSender<TeamProgressEvent>>,
    },
    // Lifecycle (matched by AgentActor::handle; sent via task abort in practice)
    #[allow(dead_code)]
    Shutdown,
}

/// Maximum tasks from TL breakdown.
pub const MAX_TASKS: usize = 10;

/// Maximum conversation history per agent.
pub const MAX_HISTORY_PER_AGENT: usize = 50;

/// Maximum characters for CONTEXT blocks in task descriptions.
pub const MAX_CONTEXT_CHARS: usize = 4000;

// ── Shared helpers (used by both legacy workflow and actor system) ──

/// Send a progress event to the TUI (non-blocking, ignores send errors).
pub fn send_progress(
    tx: &Option<mpsc::UnboundedSender<TeamProgressEvent>>,
    event: TeamProgressEvent,
) {
    if let Some(tx) = tx {
        tracing::debug!(
            "[team] sending progress event: {}",
            match &event {
                crate::team::progress::TeamProgressEvent::PhaseChanged { phase, .. } => {
                    format!("PhaseChanged({:?})", phase)
                }
                crate::team::progress::TeamProgressEvent::TasksUpdate { tasks, .. } => {
                    format!("TasksUpdate({} tasks)", tasks.len())
                }
                crate::team::progress::TeamProgressEvent::TaskStarted {
                    task_id,
                    agent_index,
                    ..
                } => format!("TaskStarted({}@{})", task_id, agent_index),
                crate::team::progress::TeamProgressEvent::TaskCompleted {
                    task_id,
                    success,
                    ..
                } => format!("TaskCompleted({}:{})", task_id, success),
                crate::team::progress::TeamProgressEvent::StatusUpdate { message, .. } => {
                    format!("StatusUpdate({:.50}…)", message)
                }
                crate::team::progress::TeamProgressEvent::TaskSubStatusUpdate {
                    task_id,
                    message,
                    ..
                } => format!("TaskSubStatusUpdate({}: {:.50}…)", task_id, message),
                crate::team::progress::TeamProgressEvent::Finished { success, .. } => {
                    format!("Finished({})", success)
                }
            }
        );
        let _ = tx.send(event);
    }
}

/// Truncate a string to `max` bytes, breaking at the last newline or space within limit.
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = s[..max]
        .char_indices()
        .last()
        .map(|(i, _)| i)
        .unwrap_or(max);
    if let Some(nl) = s[..end].rfind('\n') {
        end = nl;
    } else if let Some(sp) = s[..end].rfind(' ') {
        end = sp;
    }
    format!("{}...", s[..end].trim_end())
}

/// Extract file paths from Jr task results.
/// Uses the `files_modified` field tracked during tool execution.
pub fn extract_modified_files(results: &[TaskResult]) -> Vec<String> {
    let mut files: Vec<String> = results
        .iter()
        .flat_map(|r| r.files_modified.clone())
        .collect();
    files.sort();
    files.dedup();
    files
}
