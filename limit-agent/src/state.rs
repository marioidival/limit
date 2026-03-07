use crate::error::AgentError;
use bincode::{deserialize, serialize};
use limit_llm::types::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::instrument;

const STATE_DIR: &str = ".limit";
const STATE_FILE: &str = "agent-state.bin";
const MAX_ITERATIONS: u32 = 50;
const MAX_LOOP_COUNT: u32 = 3;

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
    tool_call_history: Vec<String>,
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
            tool_call_history: Vec::new(),
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

        let encoded = serialize(self)
            .map_err(|e| AgentError::BincodeError(format!("Serialization failed: {:?}", e)))?;
        fs::write(&path, encoded)?;

        Ok(())
    }

    #[instrument]
    pub fn load_state() -> Result<Self, AgentError> {
        let path = Self::state_path();

        if !path.exists() {
            return Ok(Self::new());
        }

        let encoded = fs::read(&path)?;
        let _state: AgentState = deserialize(&encoded)
            .map_err(|e| AgentError::BincodeError(format!("Deserialization failed: {:?}", e)))?;

        Ok(_state)
    }

    #[instrument(skip(self))]
    pub fn increment_iteration(&mut self) -> Result<(), AgentError> {
        self.iteration += 1;

        if self.iteration > MAX_ITERATIONS {
            return Err(AgentError::MaxIterationsReached(MAX_ITERATIONS));
        }

        Ok(())
    }

    #[instrument(skip(self, args))]
    pub fn check_loop_detection(
        &mut self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<(), AgentError> {
        let signature = format!("{}:{}", tool_name, args);

        let count = self
            .tool_call_history
            .iter()
            .filter(|x| **x == signature)
            .count();

        if count >= MAX_LOOP_COUNT as usize {
            return Err(AgentError::LoopDetected {
                tool_name: tool_name.to_string(),
                args: args.to_string(),
                count,
            });
        }

        self.tool_call_history.push(signature);

        Ok(())
    }

    pub fn iteration(&self) -> u32 {
        self.iteration
    }

    pub fn is_max_iterations(&self) -> bool {
        self.iteration >= MAX_ITERATIONS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn create_test_message() -> Message {
        Message {
            role: limit_llm::types::Role::User,
            content: "test message".to_string(),
            tool_calls: None,
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
    fn test_increment_iteration() {
        let mut state = AgentState::new();

        for i in 1..=10 {
            state.increment_iteration().unwrap();
            assert_eq!(state.iteration, i);
        }
    }

    #[test]
    fn test_max_iterations() {
        let mut state = AgentState::new();

        state.iteration = MAX_ITERATIONS - 1;

        state.increment_iteration().unwrap();
        assert_eq!(state.iteration, MAX_ITERATIONS);

        let result = state.increment_iteration();
        assert!(result.is_err());
        assert!(matches!(result, Err(AgentError::MaxIterationsReached(50))));
    }

    #[test]
    fn test_is_max_iterations() {
        let mut state = AgentState::new();
        assert!(!state.is_max_iterations());

        state.iteration = MAX_ITERATIONS;
        assert!(state.is_max_iterations());
    }

    #[test]
    fn test_loop_detection() {
        let mut state = AgentState::new();
        let args = create_test_tool_args();

        for _ in 0..MAX_LOOP_COUNT {
            state.check_loop_detection("test_tool", &args).unwrap();
        }

        let result = state.check_loop_detection("test_tool", &args);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(AgentError::LoopDetected {
                tool_name,
                ..
            }) if tool_name == "test_tool"
        ));
    }

    #[test]
    fn test_loop_detection_different_args() {
        let mut state = AgentState::new();
        let args1 = serde_json::json!({"arg": "value1"});
        let args2 = serde_json::json!({"arg": "value2"});

        // Same tool, different args should not trigger loop detection
        for _ in 0..MAX_LOOP_COUNT {
            state.check_loop_detection("test_tool", &args1).unwrap();
        }

        // 4th call with same args should fail
        assert!(state.check_loop_detection("test_tool", &args1).is_err());

        // Different args, should work
        state.check_loop_detection("test_tool", &args2).unwrap();

        state.check_loop_detection("test_tool", &args2).unwrap();
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
