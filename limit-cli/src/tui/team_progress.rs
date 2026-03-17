//! TUI-side team progress state.
//!
//! Consumes [`TeamProgressEvent`]s from the async workflow and exposes
//! an immutable snapshot for the synchronous render loop.

use limit_agent::team::{
    TaskProgressInfo, TaskProgressStatus, TeamProgressEvent, WorkflowPhase, PHASE_COUNT,
};
use parking_lot::Mutex;
use std::time::Instant;
use tokio::sync::mpsc;

/// Immutable snapshot of team progress for rendering.
#[derive(Debug, Clone, Default)]
pub struct TeamProgressSnapshot {
    pub is_active: bool,
    pub current_phase: Option<WorkflowPhase>,
    pub phases_completed: usize,
    pub tasks: Vec<TaskProgressInfo>,
    pub started_at: Option<Instant>,
    pub finished: bool,
    pub success: bool,
    pub status_text: String,
}

/// Thread-safe state that receives events and produces snapshots.
pub struct TeamProgressState {
    inner: Mutex<TeamProgressSnapshot>,
}

impl TeamProgressState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(TeamProgressSnapshot::default()),
        }
    }

    /// Cheap clone of current state for the render loop.
    pub fn snapshot(&self) -> TeamProgressSnapshot {
        self.inner.lock().clone()
    }

    /// Apply an event from the workflow channel.
    pub fn apply(&self, event: TeamProgressEvent) {
        let mut state = self.inner.lock();
        match event {
            TeamProgressEvent::PhaseChanged { phase, completed } => {
                if !state.is_active {
                    state.is_active = true;
                    state.started_at = Some(Instant::now());
                }
                state.current_phase = Some(phase);
                state.phases_completed = completed;
            }
            TeamProgressEvent::TasksUpdate { tasks } => {
                state.tasks = tasks;
            }
            TeamProgressEvent::TaskStarted {
                task_id,
                agent_index,
            } => {
                if let Some(task) = state.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.status = TaskProgressStatus::InProgress;
                    task.agent_index = Some(agent_index);
                }
            }
            TeamProgressEvent::TaskCompleted { task_id, success } => {
                if let Some(task) = state.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.status = if success {
                        TaskProgressStatus::Completed
                    } else {
                        TaskProgressStatus::Failed
                    };
                }
            }
            TeamProgressEvent::Finished { success } => {
                state.finished = true;
                state.success = success;
                state.phases_completed = PHASE_COUNT;
            }
            TeamProgressEvent::StatusUpdate { message } => {
                state.status_text = message;
            }
        }
    }

    /// Reset state (after user sees final result).
    pub fn reset(&self) {
        *self.inner.lock() = TeamProgressSnapshot::default();
    }
}

impl Default for TeamProgressState {
    fn default() -> Self {
        Self::new()
    }
}

/// Drain pending events from a receiver and apply them to state.
pub fn drain_progress_events(
    rx: &parking_lot::Mutex<Option<mpsc::UnboundedReceiver<TeamProgressEvent>>>,
    state: &TeamProgressState,
) {
    let mut guard = rx.lock();
    if let Some(ref mut receiver) = *guard {
        while let Ok(event) = receiver.try_recv() {
            tracing::debug!(
                "[tui] team progress event received: {}",
                match &event {
                    TeamProgressEvent::PhaseChanged { phase, .. } => {
                        format!("PhaseChanged({:?})", phase)
                    }
                    TeamProgressEvent::TasksUpdate { tasks } => {
                        format!("TasksUpdate({} tasks)", tasks.len())
                    }
                    TeamProgressEvent::TaskStarted {
                        task_id,
                        agent_index,
                        ..
                    } => format!("TaskStarted({}@{})", task_id, agent_index),
                    TeamProgressEvent::TaskCompleted { task_id, success } => {
                        format!("TaskCompleted({}:{})", task_id, success)
                    }
                    TeamProgressEvent::StatusUpdate { message } => {
                        format!("StatusUpdate({:.50}…)", message)
                    }
                    TeamProgressEvent::Finished { success } => {
                        format!("Finished({})", success)
                    }
                }
            );
            state.apply(event);
        }
    }
}

impl TeamProgressSnapshot {
    /// Convert to the TUI widget snapshot type for rendering.
    pub fn to_tui_snapshot(
        &self,
    ) -> limit_tui::components::team_progress_types::TaskProgressSnapshot {
        use limit_tui::components::team_progress_types::{
            TaskProgressInfo as TuiTaskInfo, TaskProgressSnapshot as TuiSnapshot,
            TaskProgressStatus as TuiStatus, WorkflowPhase as TuiPhase,
        };

        TuiSnapshot {
            is_active: self.is_active,
            current_phase: self.current_phase.map(|p| match p {
                limit_agent::team::WorkflowPhase::PmAnalysis => TuiPhase::PmAnalysis,
                limit_agent::team::WorkflowPhase::TlPlan => TuiPhase::TlPlan,
                limit_agent::team::WorkflowPhase::TlBreakdown => TuiPhase::TlBreakdown,
                limit_agent::team::WorkflowPhase::JrExecution => TuiPhase::JrExecution,
                limit_agent::team::WorkflowPhase::TlValidation => TuiPhase::TlValidation,
                limit_agent::team::WorkflowPhase::PmDelivery => TuiPhase::PmDelivery,
            }),
            phases_completed: self.phases_completed,
            tasks: self
                .tasks
                .iter()
                .map(|t| TuiTaskInfo {
                    id: t.id.clone(),
                    description: t.description.clone(),
                    status: match t.status {
                        limit_agent::team::TaskProgressStatus::Pending => TuiStatus::Pending,
                        limit_agent::team::TaskProgressStatus::InProgress => TuiStatus::InProgress,
                        limit_agent::team::TaskProgressStatus::Completed => TuiStatus::Completed,
                        limit_agent::team::TaskProgressStatus::Failed => TuiStatus::Failed,
                    },
                    agent_index: t.agent_index,
                })
                .collect(),
            started_at: self.started_at,
            finished: self.finished,
            success: self.success,
            status_text: self.status_text.clone(),
        }
    }
}
