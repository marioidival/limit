use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Events emitted during agent execution.
///
/// These events can be used for logging, monitoring, and debugging
/// agent behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// Agent is thinking/reasoning.
    Thinking { version: u32 },
    /// A tool is being called.
    ToolCall {
        version: u32,
        name: String,
        args: HashMap<String, serde_json::Value>,
    },
    /// A tool has completed with output.
    ToolResult { version: u32, output: String },
    /// A file has been modified.
    FileChange {
        version: u32,
        path: String,
        diff: String,
    },
    /// An error occurred.
    Error { version: u32, message: String },
    /// Agent execution completed.
    Done { version: u32 },
}

/// Event bus for subscribing to agent lifecycle events.
///
/// Note: This is a placeholder for future event system implementation.
pub struct EventBus {
    _private: (),
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Subscribe to events.
    ///
    /// Note: This is a placeholder. Full implementation coming soon.
    pub fn subscribe<F>(&self, _callback: F)
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        // Placeholder - full implementation will store callbacks
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
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

    #[test]
    fn test_event_bus_new() {
        let bus = EventBus::new();
        assert!(matches!(bus, EventBus { .. }));
    }

    #[test]
    fn test_event_bus_default() {
        let bus = EventBus::default();
        assert!(matches!(bus, EventBus { .. }));
    }

    #[test]
    fn test_event_bus_subscribe() {
        let bus = EventBus::new();
        bus.subscribe(|_event| {
            // Callback would handle events here
        });
    }
}
