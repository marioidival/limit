//! Team progress event types for TUI integration.
//!
//! These types carry structured progress information from the async team
//! workflow to the synchronous TUI render loop via `mpsc` channels.

use super::workflow::WorkflowPhase;

/// Number of phases in the team workflow.
pub const PHASE_COUNT: usize = 6;

/// Events emitted by the team workflow for TUI rendering.
#[derive(Debug, Clone)]
pub enum TeamProgressEvent {
    /// A new workflow phase has been entered.
    PhaseChanged {
        phase: WorkflowPhase,
        completed: usize,
    },
    /// Full task list updated (after TL breakdown).
    TasksUpdate { tasks: Vec<TaskProgressInfo> },
    /// A task has started executing.
    TaskStarted { task_id: String, agent_index: usize },
    /// A task has completed (success or failure).
    TaskCompleted { task_id: String, success: bool },
    /// Status text shown after each phase completes (truncated agent output).
    StatusUpdate { message: String },
    /// The entire team workflow has finished.
    Finished { success: bool },
}

/// Per-task progress information displayed in the TUI.
#[derive(Debug, Clone)]
pub struct TaskProgressInfo {
    pub id: String,
    pub description: String,
    pub status: TaskProgressStatus,
    pub agent_index: Option<usize>,
}

/// Status of a single task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskProgressStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl WorkflowPhase {
    /// 0-based index of this phase within the workflow.
    pub const fn index(self) -> usize {
        match self {
            Self::PmAnalysis => 0,
            Self::TlPlan => 1,
            Self::TlBreakdown => 2,
            Self::JrExecution => 3,
            Self::TlValidation => 4,
            Self::PmDelivery => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_indices() {
        assert_eq!(WorkflowPhase::PmAnalysis.index(), 0);
        assert_eq!(WorkflowPhase::TlPlan.index(), 1);
        assert_eq!(WorkflowPhase::TlBreakdown.index(), 2);
        assert_eq!(WorkflowPhase::JrExecution.index(), 3);
        assert_eq!(WorkflowPhase::TlValidation.index(), 4);
        assert_eq!(WorkflowPhase::PmDelivery.index(), 5);
    }

    #[test]
    fn test_phase_count() {
        assert_eq!(PHASE_COUNT, 6);
    }

    #[test]
    fn test_task_progress_status_eq() {
        assert_eq!(TaskProgressStatus::Pending, TaskProgressStatus::Pending);
        assert_ne!(TaskProgressStatus::Pending, TaskProgressStatus::InProgress);
    }
}
