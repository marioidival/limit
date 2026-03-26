use crate::error::LlmError;
use crate::types::{Message, MessageContent};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CURRENT_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    version: u32,
    messages: Vec<PersistedMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedMessage {
    role: PersistedRole,
    content: Option<String>,
    tool_calls: Option<Vec<crate::types::ToolCall>>,
    tool_call_id: Option<String>,
    cache_control: Option<crate::types::CacheControl>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum PersistedRole {
    User,
    Assistant,
    System,
    Tool,
}

impl From<PersistedRole> for crate::types::Role {
    fn from(role: PersistedRole) -> Self {
        match role {
            PersistedRole::User => crate::types::Role::User,
            PersistedRole::Assistant => crate::types::Role::Assistant,
            PersistedRole::System => crate::types::Role::System,
            PersistedRole::Tool => crate::types::Role::Tool,
        }
    }
}

impl From<crate::types::Role> for PersistedRole {
    fn from(role: crate::types::Role) -> Self {
        match role {
            crate::types::Role::User => PersistedRole::User,
            crate::types::Role::Assistant => PersistedRole::Assistant,
            crate::types::Role::System => PersistedRole::System,
            crate::types::Role::Tool => PersistedRole::Tool,
        }
    }
}

impl From<PersistedMessage> for Message {
    fn from(msg: PersistedMessage) -> Self {
        Message {
            role: msg.role.into(),
            content: msg.content.map(MessageContent::Text),
            tool_calls: msg.tool_calls,
            tool_call_id: msg.tool_call_id,
            cache_control: msg.cache_control,
        }
    }
}

impl From<Message> for PersistedMessage {
    fn from(msg: Message) -> Self {
        PersistedMessage {
            role: msg.role.into(),
            content: msg.content.map(|c| c.to_text()),
            tool_calls: msg.tool_calls,
            tool_call_id: msg.tool_call_id,
            cache_control: msg.cache_control,
        }
    }
}

pub struct StatePersistence {
    file_path: std::path::PathBuf,
}

impl StatePersistence {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
        }
    }

    pub fn save(&self, messages: &[Message]) -> Result<(), LlmError> {
        let persisted_messages: Vec<PersistedMessage> =
            messages.iter().cloned().map(|m| m.into()).collect();

        let state = PersistedState {
            version: CURRENT_VERSION,
            messages: persisted_messages,
        };

        let serialized = serde_json::to_string_pretty(&state)
            .map_err(|e| LlmError::PersistenceError(format!("Failed to serialize state: {}", e)))?;

        fs::write(&self.file_path, serialized)
            .map_err(|e| LlmError::PersistenceError(format!("Failed to write file: {}", e)))?;

        Ok(())
    }

    pub fn load(&self) -> Result<Vec<Message>, LlmError> {
        let data = match fs::read_to_string(&self.file_path) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => {
                return Err(LlmError::PersistenceError(format!(
                    "Failed to read file: {}",
                    e
                )))
            }
        };

        let state: PersistedState = serde_json::from_str(&data).map_err(|e| {
            LlmError::PersistenceError(format!("Failed to deserialize state: {}", e))
        })?;

        if state.version > CURRENT_VERSION {
            return Err(LlmError::PersistenceError(format!(
                "Version mismatch: expected {}, found {}",
                CURRENT_VERSION, state.version
            )));
        }

        let messages: Vec<Message> = state.messages.into_iter().map(|m| m.into()).collect();

        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Role;
    use tempfile::tempdir;

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_state.json");

        let persistence = StatePersistence::new(&file_path);

        let messages = vec![
            Message {
                role: Role::User,
                content: Some(crate::MessageContent::text("Hello")),
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            },
            Message {
                role: Role::Assistant,
                content: Some(crate::MessageContent::text("Hi there!")),
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            },
        ];

        persistence.save(&messages).unwrap();

        let loaded = persistence.load().unwrap();

        assert_eq!(loaded.len(), messages.len());
        assert_eq!(loaded[0].content, messages[0].content);
        assert_eq!(loaded[1].content, messages[1].content);
    }

    #[test]
    fn test_save_load_with_tool_result() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_state.json");

        let persistence = StatePersistence::new(&file_path);

        let messages = vec![Message {
            role: Role::Tool,
            content: Some(crate::MessageContent::text("tool output")),
            tool_calls: None,
            tool_call_id: Some("call_123".to_string()),
            cache_control: None,
        }];

        persistence.save(&messages).unwrap();

        let loaded = persistence.load().unwrap();

        assert_eq!(loaded[0].role, Role::Tool);
        assert_eq!(loaded[0].tool_call_id, Some("call_123".to_string()));
    }

    #[test]
    fn test_load_empty_file_returns_empty() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("nonexistent.json");

        let persistence = StatePersistence::new(&file_path);

        let loaded = persistence.load().unwrap();

        assert!(loaded.is_empty());
    }

    #[test]
    fn test_load_corrupted_file_returns_error() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("corrupted.json");

        std::fs::write(&file_path, b"invalid json data").unwrap();

        let persistence = StatePersistence::new(&file_path);

        let result = persistence.load();

        assert!(result.is_err());
    }

    #[test]
    fn test_role_conversion() {
        assert_eq!(PersistedRole::User, Role::User.into());
        assert_eq!(PersistedRole::Assistant, Role::Assistant.into());
        assert_eq!(PersistedRole::System, Role::System.into());
        assert_eq!(PersistedRole::Tool, Role::Tool.into());

        assert_eq!(Role::User, PersistedRole::User.into());
        assert_eq!(Role::Assistant, PersistedRole::Assistant.into());
        assert_eq!(Role::System, PersistedRole::System.into());
        assert_eq!(Role::Tool, PersistedRole::Tool.into());
    }
}
