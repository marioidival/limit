//! Branching commands for tree-based sessions
//!
//! Provides functionality to branch from specific entries and list all branches.

use super::CommandContext;
use crate::error::CliError;

/// Branch from a specific entry in the session
///
/// This moves the leaf pointer to the specified entry, creating a new branch.
/// Subsequent appends will be children of this entry.
pub fn branch_from(ctx: &mut CommandContext, entry_id: &str) -> Result<String, CliError> {
    let session_manager = ctx
        .session_manager
        .lock()
        .map_err(|e| CliError::ConfigError(format!("Failed to lock session manager: {}", e)))?;

    let mut tree = session_manager.load_tree_session(&ctx.session_id)?;
    let new_leaf = tree.branch_from(entry_id)?;

    // Save updated tree
    session_manager.save_tree_session(&ctx.session_id, &tree)?;

    Ok(new_leaf)
}

/// List all branches (paths from root to leaves)
///
/// Returns information about all leaf nodes in the session tree,
/// including their depth (message count) and whether they're the current branch.
pub fn list_branches(ctx: &CommandContext) -> Result<Vec<BranchInfo>, CliError> {
    let session_manager = ctx
        .session_manager
        .lock()
        .map_err(|e| CliError::ConfigError(format!("Failed to lock session manager: {}", e)))?;

    let tree = session_manager.load_tree_session(&ctx.session_id)?;

    // Find all leaf nodes
    let entries = tree.entries();
    let parent_ids: std::collections::HashSet<_> = entries
        .iter()
        .filter_map(|e| e.parent_id.as_ref())
        .cloned()
        .collect();

    let leaves: Vec<_> = entries
        .iter()
        .filter(|e| !parent_ids.contains(&e.id))
        .collect();

    let branches: Vec<BranchInfo> = leaves
        .iter()
        .map(|leaf| {
            let depth = count_depth(&tree, &leaf.id);
            BranchInfo {
                leaf_id: leaf.id.clone(),
                depth,
                is_current: leaf.id == tree.leaf_id(),
            }
        })
        .collect();

    Ok(branches)
}

/// Count the depth of a leaf node (number of messages from root to leaf)
fn count_depth(tree: &crate::session_tree::SessionTree, leaf_id: &str) -> usize {
    let context = tree.build_context(leaf_id).unwrap_or_default();
    context.len()
}

/// Information about a branch in the session tree
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// ID of the leaf entry
    pub leaf_id: String,
    /// Number of messages from root to this leaf
    pub depth: usize,
    /// Whether this is the current active branch
    pub is_current: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionManager;
    use crate::session_tree::{SerializableMessage, SessionEntry, SessionEntryType};
    use limit_llm::{Message, Role};
    use tempfile::tempdir;

    /// Helper function to create a test entry
    fn create_test_entry(id: &str, parent_id: Option<&str>, content: &str) -> SessionEntry {
        SessionEntry {
            id: id.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            entry_type: SessionEntryType::Message {
                message: SerializableMessage::from(Message {
                    role: Role::User,
                    content: Some(limit_llm::MessageContent::text(content)),
                    tool_calls: None,
                    tool_call_id: None,
                    cache_control: None,
                }),
            },
        }
    }

    /// Helper function to create a test tree with branches
    /// Tree structure:
    ///   root -> A -> B
    ///          \
    ///           -> C
    fn create_test_tree_with_branches(session_manager: &SessionManager, session_id: &str) {
        let root = create_test_entry("root", None, "root content");
        let a = create_test_entry("a", Some("root"), "a content");
        let b = create_test_entry("b", Some("a"), "b content");

        session_manager
            .append_tree_entry(session_id, &root)
            .unwrap();
        session_manager.append_tree_entry(session_id, &a).unwrap();
        session_manager.append_tree_entry(session_id, &b).unwrap();
    }

    #[test]
    fn test_count_depth() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("session.db");
        let sessions_dir = dir.path().join("sessions");

        let session_manager = SessionManager::with_paths(db_path, sessions_dir).unwrap();
        let session_id = session_manager.create_new_session().unwrap();
        session_manager
            .create_tree_session(&session_id, "/test".to_string())
            .unwrap();

        create_test_tree_with_branches(&session_manager, &session_id);

        let tree = session_manager.load_tree_session(&session_id).unwrap();

        // root depth = 1
        assert_eq!(count_depth(&tree, "root"), 1);
        // a depth = 2 (root -> a)
        assert_eq!(count_depth(&tree, "a"), 2);
        // b depth = 3 (root -> a -> b)
        assert_eq!(count_depth(&tree, "b"), 3);
    }

    #[test]
    fn test_branch_from() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("session.db");
        let sessions_dir = dir.path().join("sessions");

        let session_manager = SessionManager::with_paths(db_path, sessions_dir).unwrap();
        let session_id = session_manager.create_new_session().unwrap();
        session_manager
            .create_tree_session(&session_id, "/test".to_string())
            .unwrap();

        create_test_tree_with_branches(&session_manager, &session_id);

        let mut tree = session_manager.load_tree_session(&session_id).unwrap();
        let branch_id = tree.branch_from("a").unwrap();
        assert_eq!(branch_id, "a");

        let new_entry = create_test_entry("new", Some("a"), "new content");
        tree.append(new_entry).unwrap();

        let context = tree.build_context("new").unwrap();
        assert_eq!(context.len(), 3);
    }

    #[test]
    fn test_list_branches() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("session.db");
        let sessions_dir = dir.path().join("sessions");

        let session_manager = SessionManager::with_paths(db_path, sessions_dir).unwrap();
        let session_id = session_manager.create_new_session().unwrap();
        session_manager
            .create_tree_session(&session_id, "/test".to_string())
            .unwrap();

        create_test_tree_with_branches(&session_manager, &session_id);

        // Add another branch from root
        let d = create_test_entry("d", Some("root"), "d content");
        session_manager.append_tree_entry(&session_id, &d).unwrap();

        let tree = session_manager.load_tree_session(&session_id).unwrap();

        let entries = tree.entries();
        let parent_ids: std::collections::HashSet<_> = entries
            .iter()
            .filter_map(|e| e.parent_id.as_ref())
            .cloned()
            .collect();

        let leaves: Vec<_> = entries
            .iter()
            .filter(|e| !parent_ids.contains(&e.id))
            .collect();

        assert_eq!(leaves.len(), 2);

        let leaf_ids: Vec<_> = leaves.iter().map(|l| l.id.as_str()).collect();
        assert!(leaf_ids.contains(&"b"));
        assert!(leaf_ids.contains(&"d"));
    }

    #[test]
    fn test_branch_info() {
        let info = BranchInfo {
            leaf_id: "test-branch".to_string(),
            depth: 5,
            is_current: true,
        };

        assert_eq!(info.leaf_id, "test-branch");
        assert_eq!(info.depth, 5);
        assert!(info.is_current);
    }
}
