use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Thinking {
        version: u32,
    },
    ToolCall {
        version: u32,
        name: String,
        args: HashMap<String, serde_json::Value>,
    },
    ToolResult {
        version: u32,
        output: String,
    },
    FileChange {
        version: u32,
        path: String,
        diff: String,
    },
    Error {
        version: u32,
        message: String,
    },
    Done {
        version: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = Event::Thinking { version: 1 };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Event::Thinking { version: 1 }));
    }
}
