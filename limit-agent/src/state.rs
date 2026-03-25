use crate::error::AgentError;
use limit_llm::types::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::instrument;

const STATE_DIR: &str = ".limit";
const STATE_FILE: &str = "agent-state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub timestamp: u64,
    pub action: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub messages: Vec<Message>,
    pub tool_results: HashMap<String, serde_json::Value>,
    pub decisions: Vec<Decision>,
    pub todos: Vec<Todo>,
    pub iteration: u32,
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            tool_results: HashMap::new(),
            decisions: Vec::new(),
            todos: Vec::new(),
            iteration: 0,
        }
    }

    fn state_path() -> PathBuf {
        PathBuf::from(STATE_DIR).join(STATE_FILE)
    }

    #[instrument(skip(self))]
    pub fn save_state(&self) -> Result<(), AgentError> {
        let path = Self::state_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let encoded = serde_json::to_string_pretty(self).map_err(|e| {
            AgentError::SerializationError(format!("Serialization failed: {:?}", e))
        })?;
        fs::write(&path, encoded)?;

        Ok(())
    }

    #[instrument]
    pub fn load_state() -> Result<Self, AgentError> {
        let path = Self::state_path();

        if !path.exists() {
            return Ok(Self::new());
        }

        let encoded = fs::read_to_string(&path)?;
        let _state: AgentState = serde_json::from_str(&encoded).map_err(|e| {
            AgentError::SerializationError(format!("Deserialization failed: {:?}", e))
        })?;

        Ok(_state)
    }

    #[instrument(skip(self))]
    pub fn check_loop_detection(
        &mut self,
        tool_name: &str,
        _args: &serde_json::Value,
    ) -> Result<(), AgentError> {
        Ok(())
    }

    pub fn iteration(&self) -> u32 {
        self.iteration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn create_test_message() -> Message {
        Message {
            role: limit_llm::types::Role::User,
            content: Some("test message".to_string()),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }
    }

    fn create_test_tool_args() -> serde_json::Value {
        serde_json::json!({"arg": "value"})
    }

    #[test]
    fn test_agent_state_default() {
        let state = AgentState::default();
        assert_eq!(state.iteration, 0);
        assert!(state.messages.is_empty());
        assert!(state.decisions.is_empty());
        assert!(state.todos.is_empty());
        assert!(state.tool_results.is_empty());
    }

    #[test]
    fn test_agent_state_new() {
        let state = AgentState::new();
        assert_eq!(state.iteration, 0);
        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_loop_detection() {
        let mut state = AgentState::new();
        let args = create_test_tool_args();

        // Loop detection now always returns Ok
        for _ in 0..10 {
            state.check_loop_detection("test_tool", &args).unwrap();
        }
    }

    #[test]
    fn test_save_and_load_state() -> Result<(), AgentError> {
        let path = AgentState::state_path();
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(".limit");

        let mut state = AgentState::new();

        state.decisions.push(Decision {
            timestamp: 1234567890,
            action: "test_action".to_string(),
            reason: "test reason".to_string(),
        });
        state.todos.push(Todo {
            id: "todo_1".to_string(),
            content: "test todo".to_string(),
            status: TodoStatus::Pending,
        });

        state.save_state()?;

        let loaded_state = AgentState::load_state()?;

        assert_eq!(loaded_state.decisions.len(), 1);
        assert_eq!(loaded_state.todos.len(), 1);

        Ok(())
    }
}
