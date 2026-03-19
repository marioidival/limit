//! Team workflow — the pipeline that coordinates PM → TL → Jr agents.
//!
//! This module defines the [`TeamResult`] and [`WorkflowPhase`] types.
//! The actor-based system in [`crate::team::orchestrator_actor`] drives
//! the 6-phase workflow.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// The final result produced by a team execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResult {
    /// The PM's delivery summary for the user.
    pub solution: String,
    /// Wall-clock time for the full pipeline.
    pub duration: Duration,
    /// All events recorded during execution.
    pub events: Vec<crate::team::history::TeamEvent>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team::messages::{extract_modified_files, truncate};
    use crate::team::orchestrator::TaskResult;

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
