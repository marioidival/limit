//! Types for team progress panel rendering.
//!
//! These are self-contained types used by the widget. Conversion from
//! `limit_agent` types happens in `limit-cli`.

/// Number of phases in the team workflow.
pub const PHASE_COUNT: usize = 6;

/// Status of a single task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskProgressStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Per-task progress information.
#[derive(Debug, Clone)]
pub struct TaskProgressInfo {
    pub id: String,
    pub description: String,
    pub status: TaskProgressStatus,
    pub agent_index: Option<usize>,
    pub sub_status: String,
}

/// Phase of the team workflow.
#[derive(Debug, Clone, Copy)]
pub enum WorkflowPhase {
    PmAnalysis,
    TlPlan,
    TlBreakdown,
    JrExecution,
    TlValidation,
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

/// Immutable snapshot of team progress for rendering.
#[derive(Debug, Clone, Default)]
pub struct TaskProgressSnapshot {
    pub is_active: bool,
    pub current_phase: Option<WorkflowPhase>,
    pub phases_completed: usize,
    pub tasks: Vec<TaskProgressInfo>,
    pub started_at: Option<std::time::Instant>,
    pub finished: bool,
    pub success: bool,
    pub status_text: String,
    pub spinner_frame: usize,
    pub finish_summary: String,
    pub task_scroll_offset: usize,
    pub task_list_expanded: bool,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub streaming_text: String,
}
