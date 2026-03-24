use limit_llm::{CacheControl, Message, Role, ToolCall};
use rand::Rng;
use serde::{Deserialize, Serialize};

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
            content: msg.content,
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
            content: msg.content,
            tool_calls: msg.tool_calls,
            tool_call_id: msg.tool_call_id,
            cache_control: msg.cache_control,
        }
    }
}

/// Generate a random 8-char hex ID
pub fn generate_entry_id() -> EntryId {
    let mut rng = rand::thread_rng();
    format!("{:08x}", rng.gen::<u32>())
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
}
