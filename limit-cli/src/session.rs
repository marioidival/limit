use crate::error::CliError;
use bincode::{deserialize, serialize};
use chrono::{DateTime, Utc};
use limit_llm::Message;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::instrument;
use uuid::Uuid;

const CURRENT_VERSION: u32 = 2;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub last_accessed: DateTime<Utc>,
    pub message_count: usize,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    version: u32,
    messages: Vec<PersistedMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedMessage {
    role: PersistedRole,
    content: Option<String>,
    tool_calls: Option<Vec<limit_llm::ToolCall>>,
    tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum PersistedRole {
    User,
    Assistant,
    System,
    Tool,
}

impl From<PersistedRole> for limit_llm::Role {
    fn from(role: PersistedRole) -> Self {
        match role {
            PersistedRole::User => limit_llm::Role::User,
            PersistedRole::Assistant => limit_llm::Role::Assistant,
            PersistedRole::System => limit_llm::Role::System,
            PersistedRole::Tool => limit_llm::Role::Tool,
        }
    }
}

impl From<limit_llm::Role> for PersistedRole {
    fn from(role: limit_llm::Role) -> Self {
        match role {
            limit_llm::Role::User => PersistedRole::User,
            limit_llm::Role::Assistant => PersistedRole::Assistant,
            limit_llm::Role::System => PersistedRole::System,
            limit_llm::Role::Tool => PersistedRole::Tool,
        }
    }
}

impl From<PersistedMessage> for Message {
    fn from(msg: PersistedMessage) -> Self {
        Message {
            role: msg.role.into(),
            content: msg.content,
            tool_calls: msg.tool_calls,
            tool_call_id: msg.tool_call_id,
        }
    }
}

impl From<Message> for PersistedMessage {
    fn from(msg: Message) -> Self {
        PersistedMessage {
            role: msg.role.into(),
            content: msg.content,
            tool_calls: msg.tool_calls,
            tool_call_id: msg.tool_call_id,
        }
    }
}

pub struct SessionManager {
    db_path: PathBuf,
    sessions_dir: PathBuf,
}

impl SessionManager {
    pub fn new() -> Result<Self, CliError> {
        // Centralize all session data in ~/.limit/
        let home_dir = dirs::home_dir()
            .ok_or_else(|| CliError::ConfigError("Failed to get home directory".to_string()))?;
        let limit_dir = home_dir.join(".limit");
        fs::create_dir_all(&limit_dir).map_err(|e| {
            CliError::ConfigError(format!("Failed to create .limit directory: {}", e))
        })?;

        let sessions_dir = limit_dir.join("sessions");
        fs::create_dir_all(&sessions_dir).map_err(|e| {
            CliError::ConfigError(format!("Failed to create sessions directory: {}", e))
        })?;

        let db_path = limit_dir.join("session.db");

        Self::with_paths(db_path, sessions_dir)
    }

    /// Create SessionManager with custom paths (for testing)
    pub fn with_paths(db_path: PathBuf, sessions_dir: PathBuf) -> Result<Self, CliError> {
        fs::create_dir_all(&sessions_dir).map_err(|e| {
            CliError::ConfigError(format!("Failed to create sessions directory: {}", e))
        })?;

        let session_manager = Self {
            db_path,
            sessions_dir,
        };

        session_manager.init_db()?;
        Ok(session_manager)
    }

    pub fn init_db(&self) -> Result<(), CliError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| CliError::ConfigError(format!("Failed to open database: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                last_accessed TEXT NOT NULL,
                message_count INTEGER NOT NULL,
                total_input_tokens INTEGER NOT NULL DEFAULT 0,
                total_output_tokens INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(|e| CliError::ConfigError(format!("Failed to create sessions table: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_last_accessed ON sessions(last_accessed DESC)",
            [],
        )
        .map_err(|e| CliError::ConfigError(format!("Failed to create index: {}", e)))?;

        Ok(())
    }

    fn get_connection(&self) -> Result<Connection, CliError> {
        Connection::open(&self.db_path)
            .map_err(|e| CliError::ConfigError(format!("Failed to open database: {}", e)))
    }

    pub fn create_new_session(&self) -> Result<String, CliError> {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO sessions (id, created_at, last_accessed, message_count, total_input_tokens, total_output_tokens) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![&session_id, &now, &now, 0, 0, 0],
        )
        .map_err(|e| CliError::ConfigError(format!("Failed to create session: {}", e)))?;

        Ok(session_id)
    }

    #[instrument(skip(self, messages))]
    pub fn save_session(
        &self,
        session_id: &str,
        messages: &[Message],
        total_input_tokens: u64,
        total_output_tokens: u64,
    ) -> Result<(), CliError> {
        let file_path = self.sessions_dir.join(format!("{}.bin", session_id));

        fs::create_dir_all(&self.sessions_dir).map_err(|e| {
            CliError::ConfigError(format!("Failed to create sessions directory: {}", e))
        })?;

        let persisted_messages: Vec<PersistedMessage> =
            messages.iter().cloned().map(|m| m.into()).collect();

        let state = PersistedState {
            version: CURRENT_VERSION,
            messages: persisted_messages,
        };

        let serialized = serialize(&state)
            .map_err(|e| CliError::ConfigError(format!("Failed to serialize messages: {}", e)))?;

        fs::write(&file_path, serialized)
            .map_err(|e| CliError::ConfigError(format!("Failed to write session file: {}", e)))?;

        let now = Utc::now().to_rfc3339();
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE sessions SET last_accessed = ?1, message_count = ?2, total_input_tokens = ?3, total_output_tokens = ?4 WHERE id = ?5",
            params![&now, messages.len() as i64, total_input_tokens as i64, total_output_tokens as i64, session_id],
        )
        .map_err(|e| CliError::ConfigError(format!("Failed to update session metadata: {}", e)))?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn load_session(&self, session_id: &str) -> Result<Vec<Message>, CliError> {
        let file_path = self.sessions_dir.join(format!("{}.bin", session_id));

        let data = fs::read(&file_path)
            .map_err(|e| CliError::ConfigError(format!("Failed to read session file: {}", e)))?;

        let state: PersistedState = deserialize(&data)
            .map_err(|e| CliError::ConfigError(format!("Failed to deserialize messages: {}", e)))?;

        // Handle version migration if needed
        if state.version > CURRENT_VERSION {
            return Err(CliError::ConfigError(format!(
                "Version mismatch: expected {}, found {}",
                CURRENT_VERSION, state.version
            )));
        }

        let now = Utc::now().to_rfc3339();
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE sessions SET last_accessed = ?1 WHERE id = ?2",
            params![&now, session_id],
        )
        .map_err(|e| CliError::ConfigError(format!("Failed to update last_accessed: {}", e)))?;

        // Filter out system messages - z.ai API doesn't support system role
        let messages: Vec<Message> = state
            .messages
            .into_iter()
            .map(Message::from)
            .filter(|m| m.role != limit_llm::Role::System)
            .collect();

        Ok(messages)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>, CliError> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare("SELECT id, created_at, last_accessed, message_count, total_input_tokens, total_output_tokens FROM sessions ORDER BY last_accessed DESC")
            .map_err(|e| CliError::ConfigError(format!("Failed to prepare query: {}", e)))?;

        let session_iter = stmt
            .query_map([], |row| {
                Ok(SessionInfo {
                    id: row.get(0)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    last_accessed: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    message_count: row.get::<_, i64>(3)? as usize,
                    total_input_tokens: row.get::<_, i64>(4)? as u64,
                    total_output_tokens: row.get::<_, i64>(5)? as u64,
                })
            })
            .map_err(|e| CliError::ConfigError(format!("Failed to query sessions: {}", e)))?;

        let mut sessions = Vec::new();
        for session in session_iter {
            sessions.push(
                session.map_err(|e| {
                    CliError::ConfigError(format!("Failed to parse session: {}", e))
                })?,
            );
        }

        Ok(sessions)
    }

    pub fn get_last_session(&self) -> Result<Option<SessionInfo>, CliError> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare("SELECT id, created_at, last_accessed, message_count, total_input_tokens, total_output_tokens FROM sessions ORDER BY last_accessed DESC LIMIT 1")
            .map_err(|e| CliError::ConfigError(format!("Failed to prepare query: {}", e)))?;

        let mut session_iter = stmt
            .query_map([], |row| {
                Ok(SessionInfo {
                    id: row.get(0)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    last_accessed: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    message_count: row.get::<_, i64>(3)? as usize,
                    total_input_tokens: row.get::<_, i64>(4)? as u64,
                    total_output_tokens: row.get::<_, i64>(5)? as u64,
                })
            })
            .map_err(|e| CliError::ConfigError(format!("Failed to query last session: {}", e)))?;

        match session_iter.next() {
            Some(session) => Ok(Some(session.map_err(|e| {
                CliError::ConfigError(format!("Failed to parse session: {}", e))
            })?)),
            None => Ok(None),
        }
    }

    /// Update token counts for a session
    #[allow(dead_code)]
    pub fn update_session_tokens(
        &self,
        session_id: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> Result<(), CliError> {
        let conn = self.get_connection()?;
        conn.execute(
            "UPDATE sessions SET total_input_tokens = total_input_tokens + ?1, total_output_tokens = total_output_tokens + ?2 WHERE id = ?3",
            params![input_tokens as i64, output_tokens as i64, session_id],
        )
        .map_err(|e| CliError::ConfigError(format!("Failed to update session tokens: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_session_manager_new() {
        let dir = tempdir().unwrap();
        let sessions_dir = dir.path().join("sessions");
        let db_path = dir.path().join("session.db");

        fs::create_dir_all(&sessions_dir).unwrap();

        let manager = SessionManager {
            db_path,
            sessions_dir,
        };

        manager.init_db().unwrap();

        assert!(manager.db_path.exists());
    }

    #[test]
    fn test_create_new_session() {
        let dir = tempdir().unwrap();
        let sessions_dir = dir.path().join("sessions");
        let db_path = dir.path().join("session.db");

        fs::create_dir_all(&sessions_dir).unwrap();

        let manager = SessionManager {
            db_path,
            sessions_dir,
        };

        manager.init_db().unwrap();

        let session_id = manager.create_new_session().unwrap();

        assert!(!session_id.is_empty());

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session_id);
        assert_eq!(sessions[0].message_count, 0);
    }

    #[test]
    fn test_save_and_load_session() {
        let dir = tempdir().unwrap();
        let sessions_dir = dir.path().join("sessions");
        let db_path = dir.path().join("session.db");

        fs::create_dir_all(&sessions_dir).unwrap();

        let manager = SessionManager {
            db_path,
            sessions_dir,
        };

        manager.init_db().unwrap();

        let session_id = manager.create_new_session().unwrap();

        let messages = vec![
            Message {
                role: limit_llm::Role::User,
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: limit_llm::Role::Assistant,
                content: Some("Hi there!".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        manager.save_session(&session_id, &messages, 0, 0).unwrap();

        let loaded = manager.load_session(&session_id).unwrap();

        assert_eq!(loaded.len(), messages.len());
        assert_eq!(loaded[0].content, messages[0].content);
        assert_eq!(loaded[1].content, messages[1].content);

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions[0].message_count, 2);
    }

    #[test]
    fn test_list_sessions() {
        let dir = tempdir().unwrap();
        let sessions_dir = dir.path().join("sessions");
        let db_path = dir.path().join("session.db");

        fs::create_dir_all(&sessions_dir).unwrap();

        let manager = SessionManager {
            db_path,
            sessions_dir,
        };

        manager.init_db().unwrap();

        let session_id1 = manager.create_new_session().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let session_id2 = manager.create_new_session().unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, session_id2);
        assert_eq!(sessions[1].id, session_id1);
    }

    #[test]
    fn test_get_last_session() {
        let dir = tempdir().unwrap();
        let sessions_dir = dir.path().join("sessions");
        let db_path = dir.path().join("session.db");

        fs::create_dir_all(&sessions_dir).unwrap();

        let manager = SessionManager {
            db_path,
            sessions_dir,
        };

        manager.init_db().unwrap();

        let last = manager.get_last_session().unwrap();
        assert!(last.is_none());

        let _session_id1 = manager.create_new_session().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let session_id2 = manager.create_new_session().unwrap();

        let last = manager.get_last_session().unwrap();
        assert!(last.is_some());
        assert_eq!(last.unwrap().id, session_id2);
    }

    #[test]
    fn test_session_persistence_across_restarts() {
        let dir = tempdir().unwrap();
        let sessions_dir = dir.path().join("sessions");
        let db_path = dir.path().join("session.db");

        fs::create_dir_all(&sessions_dir).unwrap();

        let manager1 = SessionManager {
            db_path: db_path.clone(),
            sessions_dir: sessions_dir.clone(),
        };

        manager1.init_db().unwrap();

        let session_id = manager1.create_new_session().unwrap();

        let messages = vec![Message {
            role: limit_llm::Role::User,
            content: Some("Test message".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        manager1.save_session(&session_id, &messages, 0, 0).unwrap();

        drop(manager1);

        let manager2 = SessionManager {
            db_path,
            sessions_dir,
        };

        manager2.init_db().unwrap();

        let loaded = manager2.load_session(&session_id).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, Some("Test message".to_string()));
    }
}
