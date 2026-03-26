use limit_llm::{CacheControl, Message, Role, ToolCall};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

/// Unique 8-character hex ID for session entries
pub type EntryId = String;

/// A single entry in the session tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    /// 8-char hex ID
    pub id: EntryId,
    /// Parent entry ID (None for root)
    pub parent_id: Option<EntryId>,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// The entry content
    #[serde(flatten)]
    pub entry_type: SessionEntryType,
}

/// Types of entries in the session tree
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEntryType {
    /// Session metadata (first entry in file)
    Session { version: u32, cwd: String },
    /// User/assistant/tool message
    Message { message: SerializableMessage },
    /// Compaction summary (replaces old messages)
    Compaction {
        summary: String,
        first_kept_id: EntryId,
    },
    /// Branch context when switching branches
    BranchSummary { from_id: EntryId, summary: String },
}

/// Message that can be serialized to JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl From<Message> for SerializableMessage {
    fn from(msg: Message) -> Self {
        Self {
            role: match msg.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
                Role::System => "system".to_string(),
                Role::Tool => "tool".to_string(),
            },
            content: msg.content.map(|c| c.to_text()),
            tool_calls: msg.tool_calls,
            tool_call_id: msg.tool_call_id,
            cache_control: msg.cache_control,
        }
    }
}

impl From<SerializableMessage> for Message {
    fn from(msg: SerializableMessage) -> Self {
        Self {
            role: match msg.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "system" => Role::System,
                "tool" => Role::Tool,
                _ => Role::User,
            },
            content: msg.content.map(limit_llm::MessageContent::text),
            tool_calls: msg.tool_calls,
            tool_call_id: msg.tool_call_id,
            cache_control: msg.cache_control,
        }
    }
}

/// Generate a random 8-char hex ID
pub fn generate_entry_id() -> EntryId {
    let mut rng = rand::rng();
    format!("{:08x}", rng.random::<u32>())
}

/// In-memory tree structure for session entries
pub struct SessionTree {
    /// All entries indexed by ID
    entries: HashMap<EntryId, SessionEntry>,
    /// Current leaf entry ID
    leaf_id: EntryId,
    /// Session metadata
    session_id: String,
    /// Working directory when session was created
    cwd: String,
}

impl SessionTree {
    /// Create a new empty session tree
    pub fn new(cwd: String) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        Self {
            entries: HashMap::new(),
            leaf_id: String::new(),
            session_id,
            cwd,
        }
    }

    /// Load from existing entries
    pub fn from_entries(
        entries: Vec<SessionEntry>,
        session_id: String,
        cwd: String,
    ) -> Result<Self, SessionTreeError> {
        let mut by_id: HashMap<EntryId, SessionEntry> = HashMap::new();
        let mut leaf_id = String::new();

        for entry in entries {
            leaf_id = entry.id.clone();
            by_id.insert(entry.id.clone(), entry);
        }

        Ok(Self {
            entries: by_id,
            leaf_id,
            session_id,
            cwd,
        })
    }

    /// Append a new entry as child of current leaf
    pub fn append(&mut self, entry: SessionEntry) -> Result<(), SessionTreeError> {
        let id = entry.id.clone();

        if self.entries.is_empty() {
            if entry.parent_id.is_some() {
                return Err(SessionTreeError::InvalidParent {
                    expected: "none (first entry)".to_string(),
                    got: entry.parent_id.clone(),
                });
            }
        } else if entry.parent_id.as_ref() != Some(&self.leaf_id) {
            return Err(SessionTreeError::InvalidParent {
                expected: self.leaf_id.clone(),
                got: entry.parent_id.clone(),
            });
        }

        self.entries.insert(id.clone(), entry);
        self.leaf_id = id;
        Ok(())
    }

    /// Build message context from leaf to root
    pub fn build_context(&self, leaf_id: &str) -> Result<Vec<Message>, SessionTreeError> {
        let mut path = Vec::new();
        let mut current_id = Some(leaf_id.to_string());

        while let Some(id) = current_id {
            let entry = self
                .entries
                .get(&id)
                .ok_or(SessionTreeError::EntryNotFound(id))?;

            // Handle compaction: stop at first_kept_id
            if let SessionEntryType::Compaction { first_kept_id, .. } = &entry.entry_type {
                current_id = Some(first_kept_id.clone());
                path.push(entry.clone());
                continue;
            }

            current_id = entry.parent_id.clone();
            path.push(entry.clone());
        }

        path.reverse();

        let messages: Vec<Message> = path
            .into_iter()
            .filter_map(|entry| match entry.entry_type {
                SessionEntryType::Message { message } => Some(Message::from(message)),
                SessionEntryType::Compaction { summary, .. } => Some(Message {
                    role: Role::User,
                    content: Some(limit_llm::MessageContent::text(format!(
                        "<summary>\n{}\n</summary>",
                        summary
                    ))),
                    tool_calls: None,
                    tool_call_id: None,
                    cache_control: None,
                }),
                SessionEntryType::Session { .. } => None,
                SessionEntryType::BranchSummary { .. } => None,
            })
            .collect();

        Ok(messages)
    }

    /// Create a branch from a specific entry
    pub fn branch_from(&mut self, entry_id: &str) -> Result<EntryId, SessionTreeError> {
        if !self.entries.contains_key(entry_id) {
            return Err(SessionTreeError::EntryNotFound(entry_id.to_string()));
        }

        self.leaf_id = entry_id.to_string();
        Ok(entry_id.to_string())
    }

    /// Get current leaf ID
    pub fn leaf_id(&self) -> &str {
        &self.leaf_id
    }

    /// Get all entries
    pub fn entries(&self) -> Vec<&SessionEntry> {
        self.entries.values().collect()
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Save tree to JSONL file
    pub fn save_to_file(&self, path: &Path) -> Result<(), SessionTreeError> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let header = SessionEntry {
            id: self.session_id.clone(),
            parent_id: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            entry_type: SessionEntryType::Session {
                version: 1,
                cwd: self.cwd.clone(),
            },
        };
        writeln!(writer, "{}", serde_json::to_string(&header)?)?;

        let sorted = self.sort_entries()?;
        for entry in sorted {
            writeln!(writer, "{}", serde_json::to_string(&entry)?)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Load tree from JSONL file
    pub fn load_from_file(path: &Path) -> Result<Self, SessionTreeError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut entries = Vec::new();
        let mut session_id = String::new();
        let mut cwd = String::new();

        for line in reader.lines() {
            let line: String = line?;
            if line.trim().is_empty() {
                continue;
            }

            let entry: SessionEntry = serde_json::from_str(&line)?;

            if let SessionEntryType::Session { version: _, cwd: c } = &entry.entry_type {
                session_id = entry.id.clone();
                cwd = c.clone();
            } else {
                entries.push(entry);
            }
        }

        Self::from_entries(entries, session_id, cwd)
    }

    /// Append a single entry to file (for incremental saves)
    pub fn append_to_file(
        &self,
        path: &Path,
        entry: &SessionEntry,
    ) -> Result<(), SessionTreeError> {
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;

        writeln!(file, "{}", serde_json::to_string(entry)?)?;
        Ok(())
    }

    fn sort_entries(&self) -> Result<Vec<SessionEntry>, SessionTreeError> {
        let mut sorted = Vec::new();
        let mut visited: std::collections::HashSet<EntryId> = std::collections::HashSet::new();

        let roots: Vec<_> = self
            .entries
            .values()
            .filter(|e| e.parent_id.is_none())
            .collect();

        for root in roots {
            self.sort_dfs(root, &mut sorted, &mut visited)?;
        }

        Ok(sorted)
    }

    fn sort_dfs(
        &self,
        entry: &SessionEntry,
        sorted: &mut Vec<SessionEntry>,
        visited: &mut std::collections::HashSet<EntryId>,
    ) -> Result<(), SessionTreeError> {
        if visited.contains(&entry.id) {
            return Ok(());
        }

        visited.insert(entry.id.clone());
        sorted.push(entry.clone());

        for child in self.entries.values() {
            if child.parent_id.as_ref() == Some(&entry.id) {
                self.sort_dfs(child, sorted, visited)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SessionTreeError {
    #[error("Entry not found: {0}")]
    EntryNotFound(String),
    #[error("Invalid parent: expected {expected:?}, got {got:?}")]
    InvalidParent {
        expected: String,
        got: Option<String>,
    },
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_entry_serialization() {
        let entry = SessionEntry {
            id: "a1b2c3d4".to_string(),
            parent_id: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            entry_type: SessionEntryType::Message {
                message: SerializableMessage {
                    role: "user".to_string(),
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    cache_control: None,
                },
            },
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"id\":\"a1b2c3d4\""));
        assert!(json.contains("\"type\":\"message\""));

        let parsed: SessionEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, entry.id);
    }

    #[test]
    fn test_build_context_linear() {
        let mut tree = SessionTree::new("/test".to_string());

        let msg1 = SessionEntry {
            id: "a1b2c3d4".to_string(),
            parent_id: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            entry_type: SessionEntryType::Message {
                message: SerializableMessage::from(Message {
                    role: Role::User,
                    content: Some(limit_llm::MessageContent::text("Hello")),
                    tool_calls: None,
                    tool_call_id: None,
                    cache_control: None,
                }),
            },
        };

        let msg2 = SessionEntry {
            id: "b2c3d4e5".to_string(),
            parent_id: Some("a1b2c3d4".to_string()),
            timestamp: "2024-01-01T00:01:00Z".to_string(),
            entry_type: SessionEntryType::Message {
                message: SerializableMessage::from(Message {
                    role: Role::Assistant,
                    content: Some(limit_llm::MessageContent::text("Hi!")),
                    tool_calls: None,
                    tool_call_id: None,
                    cache_control: None,
                }),
            },
        };

        tree.append(msg1).unwrap();
        tree.append(msg2).unwrap();

        let messages = tree.build_context("b2c3d4e5").unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content.as_ref().unwrap().to_text(), "Hello");
        assert_eq!(messages[1].content.as_ref().unwrap().to_text(), "Hi!");
    }

    #[test]
    fn test_build_context_with_branching() {
        let mut tree = SessionTree::new("/test".to_string());

        // Root -> A -> B
        //         \
        //          -> C (branch from A)
        let root = create_test_entry("root", None, "root content");
        let a = create_test_entry("a", Some("root"), "a content");
        let b = create_test_entry("b", Some("a"), "b content");
        let c = create_test_entry("c", Some("a"), "c content");

        // Build main path: root -> A -> B
        tree.append(root).unwrap();
        tree.append(a).unwrap();
        tree.append(b).unwrap();

        // Create branch from A to C
        tree.branch_from("a").unwrap();
        tree.append(c).unwrap();

        // Build context from B: root -> A -> B
        let context_b = tree.build_context("b").unwrap();
        assert_eq!(context_b.len(), 3);

        // Build context from C: root -> A -> C
        let context_c = tree.build_context("c").unwrap();
        assert_eq!(context_c.len(), 3);
        assert_eq!(
            context_c[2].content.as_ref().unwrap().to_text(),
            "c content"
        );
    }

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

    #[test]
    fn test_jsonl_roundtrip() {
        let mut tree = SessionTree::new("/test".to_string());

        let entry1 = create_test_entry("a1b2c3d4", None, "first");
        let entry2 = create_test_entry("b2c3d4e5", Some("a1b2c3d4"), "second");

        tree.append(entry1).unwrap();
        tree.append(entry2).unwrap();

        let file = tempfile::NamedTempFile::new().unwrap();
        tree.save_to_file(file.path()).unwrap();

        let loaded = SessionTree::load_from_file(file.path()).unwrap();

        assert_eq!(loaded.leaf_id(), "b2c3d4e5");
        assert_eq!(loaded.entries().len(), 2);

        let context = loaded.build_context("b2c3d4e5").unwrap();
        assert_eq!(context.len(), 2);
    }

    #[test]
    fn test_jsonl_format() {
        let mut tree = SessionTree::new("/test".to_string());
        tree.append(create_test_entry("a1b2c3d4", None, "test"))
            .unwrap();

        let file = tempfile::NamedTempFile::new().unwrap();
        tree.save_to_file(file.path()).unwrap();

        let content = std::fs::read_to_string(file.path()).unwrap();

        for line in content.lines() {
            if !line.is_empty() {
                serde_json::from_str::<serde_json::Value>(line).expect("Line should be valid JSON");
            }
        }
    }
}
