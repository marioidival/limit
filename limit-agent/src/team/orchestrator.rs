//! Task orchestration — breaking down plans and executing tasks.
//!
//! Defines the [`Task`] and [`TaskResult`] types used by the [`Team`](super::Team)
//! during the execution phase.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single unit of work assigned to a Junior developer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier.
    pub id: String,
    /// Human-readable description of what to do.
    pub description: String,
    /// Current status.
    pub status: TaskStatus,
}

impl Task {
    /// Create a new pending task with the given description.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.into(),
            status: TaskStatus::Pending,
        }
    }
}

/// Lifecycle status of a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    /// Not yet started.
    Pending,
    /// Currently being executed.
    InProgress,
    /// Successfully completed.
    Completed,
    /// Failed to complete.
    Failed,
}

/// The outcome of executing a single task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Which task this result belongs to.
    pub task_id: String,
    /// The agent's output / summary.
    pub output: String,
    /// Whether the task succeeded.
    pub success: bool,
}

/// Parse `TASK: <description>` lines from a TL response into a list of [`Task`]s.
pub fn parse_tasks(text: &str) -> Vec<Task> {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("TASK:")
        })
        .map(|line| {
            let desc = line.trim().trim_start_matches("TASK:").trim().to_string();
            Task::new(desc)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_new() {
        let task = Task::new("Write tests");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(!task.id.is_empty());
        assert_eq!(task.description, "Write tests");
    }

    #[test]
    fn test_task_serialization() {
        let task = Task::new("Create file");
        let json = serde_json::to_string(&task).unwrap();
        let deserialized: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(task.id, deserialized.id);
        assert_eq!(task.description, deserialized.description);
    }

    #[test]
    fn test_task_status_eq() {
        assert_eq!(TaskStatus::Pending, TaskStatus::Pending);
        assert_ne!(TaskStatus::Pending, TaskStatus::Completed);
    }

    #[test]
    fn test_parse_tasks_empty() {
        let tasks = parse_tasks("No tasks here");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_parse_tasks_multiple() {
        let input = "TASK: Create file A\nSome text\nTASK: Create file B\nTASK: Run tests";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].description, "Create file A");
        assert_eq!(tasks[1].description, "Create file B");
        assert_eq!(tasks[2].description, "Run tests");
    }

    #[test]
    fn test_parse_tasks_with_whitespace() {
        let input = "  TASK:   Trim this description  \nTASK:  And this one";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].description, "Trim this description");
        assert_eq!(tasks[1].description, "And this one");
    }
}
