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
/// Robust to common LLM output variations:
/// - Case-insensitive `TASK:` / `DEPENDS_ON:` matching
/// - Markdown list prefixes (`1.`, `- `, `* `)
/// - `DEPENDS_ON:` inside CONTEXT blocks is ignored (treated as description text)
/// - Numeric dependency references (`DEPENDS_ON: 1, 2` → 1-based task index)
pub fn parse_tasks(text: &str) -> Vec<Task> {
    let mut tasks: Vec<Task> = Vec::new();
    let mut current_desc: Option<String> = None;
    let mut current_deps: Vec<String> = Vec::new();
    let mut in_context = false;

    for line in text.lines() {
        let trimmed = line.trim();

        // Track CONTEXT block boundaries (only outside code fences)
        if !trimmed.starts_with("```") && trimmed == "CONTEXT:" {
            in_context = true;
        }

        // Strip common markdown list prefixes: "1. ", "- ", "* "
        let after_digit = trimmed
            .strip_prefix(|c: char| c.is_ascii_digit())
            .unwrap_or(trimmed);
        let after_dot = after_digit.strip_prefix(". ").unwrap_or(after_digit);
        let after_dash = after_dot.strip_prefix("- ").unwrap_or(after_dot);
        let stripped = after_dash.strip_prefix("* ").unwrap_or(after_dash);

        // Strip markdown bold/inline code formatting from prefix
        let stripped = stripped.strip_prefix("**").unwrap_or(stripped);
        let stripped = stripped.strip_prefix("`").unwrap_or(stripped);

        let upper = stripped.to_uppercase();

        if upper.starts_with("TASK:") {
            // Finalize previous task if any
            if let Some(desc) = current_desc.take() {
                let mut task = Task::new(desc);
                task.depends_on = std::mem::take(&mut current_deps);
                tasks.push(task);
            }
            let desc = stripped[5..]
                .trim()
                .trim_start_matches(':')
                .trim()
                .to_string();
            current_desc = Some(desc);
            in_context = false;
        } else if upper.starts_with("DEPENDS_ON:") && current_desc.is_some() && !in_context {
            // DEPENDS_ON only parsed outside CONTEXT blocks
            let deps_str = stripped[11..].trim().trim_start_matches(':').trim();
            let deps: Vec<String> = deps_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            current_deps.extend(deps);
        } else if let Some(ref mut desc) = current_desc {
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

    // Warn when text looks like it has tasks but none parsed
    if tasks.is_empty() && text.contains("TASK") {
        tracing::warn!(
            "[team] TL breakdown contained 'TASK' but no tasks were parsed ({} chars)",
            text.len()
        );
    }

    resolve_dependencies(&mut tasks);
    tasks
}

/// Resolve `depends_on` entries from task descriptions to task IDs.
///
/// Supports both description matching and 1-based numeric indices.
fn resolve_dependencies(tasks: &mut [Task]) {
    let desc_to_id: std::collections::HashMap<String, String> = tasks
        .iter()
        .enumerate()
        .flat_map(|(i, t)| {
            let first_line = t
                .description
                .lines()
                .next()
                .unwrap_or(&t.description)
                .trim()
                .to_string();
            let mut entries = vec![(first_line.clone(), t.id.clone())];
            // Also index by 1-based number
            entries.push(((i + 1).to_string(), t.id.clone()));
            entries
        })
        .collect();

    for task in tasks.iter_mut() {
        task.depends_on = task
            .depends_on
            .iter()
            .filter_map(|dep| desc_to_id.get(dep.trim()).cloned())
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

    #[test]
    fn test_parse_tasks_case_insensitive() {
        let input = "Task: Create file A\nSome text\ntask: Create file B";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].description, "Create file A\nSome text");
        assert_eq!(tasks[1].description, "Create file B");
    }

    #[test]
    fn test_parse_tasks_depends_on_case_insensitive() {
        let input = "TASK: Create auth module\nTASK: Add login\ndepends_on: Create auth module";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[1].depends_on.len(), 1);
        assert_eq!(tasks[1].depends_on[0], tasks[0].id);
    }

    #[test]
    fn test_parse_tasks_markdown_list_prefix() {
        let input = "1. TASK: First task\n2. TASK: Second task\n3. TASK: Third task";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].description, "First task");
        assert_eq!(tasks[1].description, "Second task");
        assert_eq!(tasks[2].description, "Third task");
    }

    #[test]
    fn test_parse_tasks_dash_list_prefix() {
        let input = "- TASK: First task\n- TASK: Second task";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_parse_tasks_depends_on_inside_context_not_parsed() {
        let input = "TASK: Create file\nCONTEXT:\nSome context here\nDEPENDS_ON: should not parse\nTASK: Another task";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0]
            .description
            .contains("DEPENDS_ON: should not parse"));
        assert!(tasks[0].depends_on.is_empty());
    }

    #[test]
    fn test_parse_tasks_bold_or_code_block_task_prefix() {
        let input = "**TASK:** Create file\nSome details\n**TASK:** Another file";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].description.contains("Create file"));
    }

    #[test]
    fn test_parse_tasks_numbered_depends_on() {
        let input = "TASK: Create auth\nTASK: Create user\nTASK: Add login\nDEPENDS_ON: 1, 2";
        let tasks = parse_tasks(input);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[2].depends_on.len(), 2);
        assert_eq!(tasks[2].depends_on[0], tasks[0].id);
        assert_eq!(tasks[2].depends_on[1], tasks[1].id);
    }
}
