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
    /// Human-readable description of what to do (may include CONTEXT block).
    pub description: String,
    /// Current status.
    pub status: TaskStatus,
    /// Task IDs this task depends on (must complete first).
    pub depends_on: Vec<String>,
}

impl Task {
    /// Create a new pending task with the given description.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.into(),
            status: TaskStatus::Pending,
            depends_on: Vec::new(),
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
    /// Whether the agent hit the tool-call limit.
    pub hit_tool_limit: bool,
    /// Files modified by file_write or file_edit tool calls.
    pub files_modified: Vec<String>,
}

/// Parse `TASK: <description>` lines from a TL response into a list of [`Task`]s.
///
/// Supports multi-line descriptions (CONTEXT blocks) and `DEPENDS_ON:` directives.
///
/// Format:
/// ```text
/// TASK: <description>
/// CONTEXT:
/// <multi-line content>
/// TASK: <another description>
/// DEPENDS_ON: <task description to depend on>
/// ```
pub fn parse_tasks(text: &str) -> Vec<Task> {
    let mut tasks: Vec<Task> = Vec::new();
    let mut current_desc: Option<String> = None;
    let mut current_deps: Vec<String> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("TASK:") {
            // Finalize previous task if any
            if let Some(desc) = current_desc.take() {
                let mut task = Task::new(desc);
                task.depends_on = std::mem::take(&mut current_deps);
                tasks.push(task);
            }
            let desc = trimmed.trim_start_matches("TASK:").trim().to_string();
            current_desc = Some(desc);
        } else if trimmed.starts_with("DEPENDS_ON:") && current_desc.is_some() {
            // DEPENDS_ON applies to the currently-being-built task
            let deps: Vec<String> = trimmed
                .trim_start_matches("DEPENDS_ON:")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            current_deps.extend(deps);
        } else if let Some(ref mut desc) = current_desc {
            // Continuation line (e.g. CONTEXT block) — append to description
            desc.push('\n');
            desc.push_str(trimmed);
        }
    }

    // Finalize last task
    if let Some(desc) = current_desc.take() {
        let mut task = Task::new(desc);
        task.depends_on = std::mem::take(&mut current_deps);
        tasks.push(task);
    }

    // Resolve dependency descriptions to task IDs
    resolve_dependencies(&mut tasks);
    tasks
}

/// Resolve `depends_on` entries from task descriptions to task IDs.
fn resolve_dependencies(tasks: &mut [Task]) {
    // Build a description → id map
    let desc_to_id: std::collections::HashMap<String, String> = tasks
        .iter()
        .map(|t| {
            // Use the first line of the description as the key
            let key = t
                .description
                .lines()
                .next()
                .unwrap_or(&t.description)
                .trim();
            (key.to_string(), t.id.clone())
        })
        .collect();

    for task in tasks.iter_mut() {
        task.depends_on = task
            .depends_on
            .iter()
            .filter_map(|desc| desc_to_id.get(desc.trim()).cloned())
            .collect();
    }
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
        assert!(task.depends_on.is_empty());
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
        // "Some text" is a continuation line appended to task A
        assert_eq!(tasks[0].description, "Create file A\nSome text");
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

    #[test]
    fn test_parse_tasks_with_context() {
        let input = "TASK: Create auth module\nCONTEXT:\n```rust\npub mod auth;\n```\nTASK: Add login endpoint";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].description.contains("CONTEXT:"));
        assert!(tasks[0].description.contains("pub mod auth;"));
        assert_eq!(tasks[1].description, "Add login endpoint");
    }

    #[test]
    fn test_parse_tasks_with_dependencies() {
        let input = "TASK: Create README.es.md\nTASK: Update README.md links\nDEPENDS_ON: Create README.es.md";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].depends_on.is_empty());
        assert_eq!(tasks[1].depends_on.len(), 1);
        assert_eq!(tasks[1].depends_on[0], tasks[0].id);
    }

    #[test]
    fn test_parse_tasks_with_multiple_dependencies() {
        let input = "TASK: Create auth module\nTASK: Create user module\nTASK: Add login endpoint\nDEPENDS_ON: Create auth module, Create user module";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].depends_on.is_empty());
        assert!(tasks[1].depends_on.is_empty());
        assert_eq!(tasks[2].depends_on.len(), 2);
    }

    #[test]
    fn test_parse_tasks_unresolved_dependency() {
        let input = "TASK: Create file\nTASK: Update file\nDEPENDS_ON: Nonexistent task";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 2);
        // Unresolved dependency is dropped
        assert!(tasks[1].depends_on.is_empty());
    }
}
