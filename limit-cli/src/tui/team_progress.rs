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

/// How long to show the finished panel before auto-hiding.
const FINISHED_DISPLAY_SECS: u64 = 5;

/// Immutable snapshot of team progress for rendering.
#[derive(Debug, Clone, Default)]
pub struct TeamProgressSnapshot {
    pub is_active: bool,
    pub current_phase: Option<WorkflowPhase>,
    pub phases_completed: usize,
    pub tasks: Vec<TaskProgressInfo>,
    pub started_at: Option<Instant>,
    pub finished: bool,
    pub finished_at: Option<Instant>,
    pub success: bool,
    pub status_text: String,
    pub spinner_tick: usize,
    pub finish_summary: String,
    pub task_scroll_offset: usize,
    pub task_list_expanded: bool,
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
    /// Auto-hides the finished panel after `FINISHED_DISPLAY_SECS`.
    pub fn snapshot(&self) -> TeamProgressSnapshot {
        let snap = self.inner.lock().clone();
        if snap.finished {
            if let Some(finished_at) = snap.finished_at {
                if finished_at.elapsed().as_secs() >= FINISHED_DISPLAY_SECS {
                    // Auto-dismiss: clear the entire state so the panel disappears.
                    *self.inner.lock() = TeamProgressSnapshot::default();
                    return TeamProgressSnapshot::default();
                }
            }
        }
        snap.tick_spinner()
    }

    /// Apply an event from the workflow channel.
    pub fn apply(&self, event: TeamProgressEvent) {
        let mut state = self.inner.lock();
        match event {
            TeamProgressEvent::PhaseChanged {
                phase, completed, ..
            } => {
                if !state.is_active {
                    state.is_active = true;
                    state.started_at = Some(Instant::now());
                }
                state.current_phase = Some(phase);
                state.phases_completed = completed;
            }
            TeamProgressEvent::TasksUpdate { tasks, .. } => {
                state.tasks = tasks;
            }
            TeamProgressEvent::TaskStarted {
                task_id,
                agent_index,
                ..
            } => {
                if let Some(task) = state.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.status = TaskProgressStatus::InProgress;
                    task.agent_index = Some(agent_index);
                }
            }
            TeamProgressEvent::TaskCompleted {
                task_id, success, ..
            } => {
                if let Some(task) = state.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.status = if success {
                        TaskProgressStatus::Completed
                    } else {
                        TaskProgressStatus::Failed
                    };
                }
            }
            TeamProgressEvent::TaskSubStatusUpdate {
                task_id, message, ..
            } => {
                if let Some(task) = state.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.sub_status = message;
                }
            }
            TeamProgressEvent::Finished { success, .. } => {
                state.finished = true;
                state.success = success;
                state.phases_completed = PHASE_COUNT;
                state.finished_at = Some(Instant::now());

                // Compute finish summary
                if let Some(started_at) = state.started_at {
                    let elapsed = started_at.elapsed().as_secs();
                    let elapsed_str = if elapsed < 60 {
                        format!("{}s", elapsed)
                    } else {
                        let mins = elapsed / 60;
                        let secs = elapsed % 60;
                        format!("{}m{}s", mins, secs)
                    };

                    let completed = state
                        .tasks
                        .iter()
                        .filter(|t| t.status == TaskProgressStatus::Completed)
                        .count();
                    let failed = state
                        .tasks
                        .iter()
                        .filter(|t| t.status == TaskProgressStatus::Failed)
                        .count();
                    let total = state.tasks.len();

                    if success {
                        state.finish_summary = format!(
                            "✅ Done in {} — {}/{} tasks completed",
                            elapsed_str, completed, total
                        );
                    } else {
                        state.finish_summary = format!(
                            "⚠️ Done in {} — {}/{} tasks failed",
                            elapsed_str, failed, total
                        );
                    }
                }
            }
            TeamProgressEvent::StatusUpdate { message, .. } => {
                state.status_text = message;
            }
        }
    }

    /// Reset state (after user sees final result).
    pub fn reset(&self) {
        *self.inner.lock() = TeamProgressSnapshot::default();
    }

    /// Adjust task scroll offset.
    pub fn adjust_scroll(&self, delta: isize) {
        let mut snap = self.inner.lock();
        let new_offset = snap.task_scroll_offset as isize + delta;
        if new_offset >= 0 {
            snap.task_scroll_offset = new_offset as usize;
        }
    }

    /// Toggle task list expansion.
    pub fn toggle_expand(&self) {
        let mut snap = self.inner.lock();
        snap.task_list_expanded = !snap.task_list_expanded;
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
                    TeamProgressEvent::TasksUpdate { tasks, .. } => {
                        format!("TasksUpdate({} tasks)", tasks.len())
                    }
                    TeamProgressEvent::TaskStarted {
                        task_id,
                        agent_index,
                        ..
                    } => format!("TaskStarted({}@{})", task_id, agent_index),
                    TeamProgressEvent::TaskCompleted {
                        task_id, success, ..
                    } => {
                        format!("TaskCompleted({}:{})", task_id, success)
                    }
                    TeamProgressEvent::TaskSubStatusUpdate {
                        task_id, message, ..
                    } => {
                        format!("TaskSubStatusUpdate({}: {:.30}…)", task_id, message)
                    }
                    TeamProgressEvent::StatusUpdate { message, .. } => {
                        format!("StatusUpdate({:.50}…)", message)
                    }
                    TeamProgressEvent::Finished { success, .. } => {
                        format!("Finished({})", success)
                    }
                }
            );
            state.apply(event);
        }
    }
}

impl TeamProgressSnapshot {
    /// Increment the spinner tick and return a new snapshot.
    fn tick_spinner(&self) -> TeamProgressSnapshot {
        let mut snap = self.clone();
        snap.spinner_tick = (snap.spinner_tick + 1) % 10;
        snap
    }

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
                    sub_status: t.sub_status.clone(),
                })
                .collect(),
            started_at: self.started_at,
            finished: self.finished,
            success: self.success,
            status_text: self.status_text.clone(),
            spinner_frame: self.spinner_tick,
            finish_summary: self.finish_summary.clone(),
            task_scroll_offset: self.task_scroll_offset,
            task_list_expanded: self.task_list_expanded,
        }
    }
}
