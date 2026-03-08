// Integration test for TUI conversation flow
//
// This test verifies the complete interaction between limit-cli, limit-tui, and the agent.

use limit_cli::{AgentBridge, TuiBridge, TuiState};
use limit_llm::{Config as LlmConfig, ProviderConfig};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

#[test]
fn test_tui_integration_full_conversation() {
    // Create a config with an API key (even if invalid, we just test the flow)
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("test-integration-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    // Create agent bridge
    let agent_bridge = AgentBridge::new(config).unwrap();
    assert!(agent_bridge.is_ready());

    // Verify tool definitions are available
    let tools = agent_bridge.get_tool_definitions();
    assert!(!tools.is_empty());
    assert!(tools.iter().any(|t| t.function.name == "file_read"));

    println!("Agent bridge created with {} tools", tools.len());
}

#[test]
fn test_tui_bridge_event_ordering() {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    let agent_bridge = AgentBridge::new(config).unwrap();
    let (tx, rx) = mpsc::unbounded_channel();

    let mut tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

    // Simulate event sequence: Thinking -> ToolStart -> ToolComplete -> Done
    let events = vec![
        limit_cli::AgentEvent::Thinking,
        limit_cli::AgentEvent::ToolStart {
            name: "file_read".to_string(),
            args: serde_json::json!({"path": "/tmp/test.txt"}),
        },
        limit_cli::AgentEvent::ToolComplete {
            name: "file_read".to_string(),
            result: "Hello, World!".to_string(),
        },
        limit_cli::AgentEvent::Done,
    ];

    for event in events {
        tx.send(event).unwrap();
        tui_bridge.process_events().unwrap();
        thread::sleep(Duration::from_millis(10));
    }

    // Verify final state is Idle
    assert_eq!(tui_bridge.state(), TuiState::Idle);

    // Verify chat has system messages
    assert!(tui_bridge.chat_view().lock().unwrap().message_count() > 0);
}

#[test]
fn test_tui_bridge_tool_execution_display() {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    let agent_bridge = AgentBridge::new(config).unwrap();
    let (tx, rx) = mpsc::unbounded_channel();

    let mut tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

    // Add user message
    tui_bridge.add_user_message("Read the file /tmp/test.txt".to_string());
    assert_eq!(tui_bridge.chat_view().lock().unwrap().message_count(), 1);

    // Send tool events
    tx.send(limit_cli::AgentEvent::ToolStart {
        name: "file_read".to_string(),
        args: serde_json::json!({"path": "/tmp/test.txt"}),
    })
    .unwrap();
    tui_bridge.process_events().unwrap();

    // Should be in ToolExecuting state
    assert!(matches!(tui_bridge.state(), TuiState::ToolExecuting { .. }));

    // Progress bar should be at 0.0
    assert_eq!(tui_bridge.progress_bar().lock().unwrap().value(), 0.0);

    // Send tool complete event
    tx.send(limit_cli::AgentEvent::ToolComplete {
        name: "file_read".to_string(),
        result: "File content here".to_string(),
    })
    .unwrap();
    tui_bridge.process_events().unwrap();

    // Progress bar should be at 1.0
    assert_eq!(tui_bridge.progress_bar().lock().unwrap().value(), 1.0);

    // State should be Idle
    assert_eq!(tui_bridge.state(), TuiState::Idle);

    // Chat should have more messages
    assert!(tui_bridge.chat_view().lock().unwrap().message_count() > 1);
}

#[test]
fn test_tui_bridge_error_handling() {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    let agent_bridge = AgentBridge::new(config).unwrap();
    let (tx, rx) = mpsc::unbounded_channel();

    let mut tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

    // Send error event
    tx.send(limit_cli::AgentEvent::Error(
        "Tool execution failed".to_string(),
    ))
    .unwrap();
    tui_bridge.process_events().unwrap();

    // State should be Error
    assert!(matches!(tui_bridge.state(), TuiState::Error(_)));

    // Chat should have error message
    assert!(tui_bridge.chat_view().lock().unwrap().message_count() > 0);
}

#[test]
fn test_tui_bridge_spinner_animation() {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    let agent_bridge = AgentBridge::new(config).unwrap();
    let (tx, rx) = mpsc::unbounded_channel();

    let tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

    // Send thinking event
    tx.send(limit_cli::AgentEvent::Thinking).unwrap();

    let mut tui_bridge_mut = tui_bridge;
    tui_bridge_mut.process_events().unwrap();

    // State should be Thinking
    assert_eq!(tui_bridge_mut.state(), TuiState::Thinking);

    // Tick spinner multiple times and verify it changes
    let frames: Vec<String> = (0..5)
        .map(|_| {
            let frame = tui_bridge_mut
                .spinner()
                .lock()
                .unwrap()
                .current_frame()
                .to_string();
            tui_bridge_mut.tick_spinner();
            frame
        })
        .collect();

    // All frames should be different (at least some)
    let unique_frames: std::collections::HashSet<&String> = frames.iter().collect();
    assert!(unique_frames.len() > 1);
}

#[test]
fn test_tui_bridge_content_streaming() {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    let agent_bridge = AgentBridge::new(config).unwrap();
    let (tx, rx) = mpsc::unbounded_channel();

    let mut tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

    // Send multiple content chunks
    let chunks = ["Hello", " ", "World", "!"];
    for chunk in chunks.iter() {
        tx.send(limit_cli::AgentEvent::ContentChunk(chunk.to_string()))
            .unwrap();
        tui_bridge.process_events().unwrap();
    }

    // Chat should have messages for each chunk
    let chat = tui_bridge.chat_view().lock().unwrap();
    assert!(chat.message_count() >= chunks.len());
}

#[test]
fn test_tui_bridge_is_ready() {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    let agent_bridge = AgentBridge::new(config).unwrap();
    let (_tx, rx) = mpsc::unbounded_channel();

    let tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

    // The underlying agent bridge should be ready
    assert!(tui_bridge.agent_bridge().is_ready());
}

#[test]
fn test_tui_bridge_get_tool_definitions() {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    let agent_bridge = AgentBridge::new(config).unwrap();
    let (_tx, rx) = mpsc::unbounded_channel();

    let tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

    // Get tool definitions through the agent bridge
    let tools = tui_bridge.agent_bridge().get_tool_definitions();
    assert!(!tools.is_empty());
}

#[test]
fn test_tui_bridge_tool_schema() {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    let agent_bridge = AgentBridge::new(config).unwrap();
    let (_tx, rx) = mpsc::unbounded_channel();

    let tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

    // Verify file_read tool schema
    let tools = tui_bridge.agent_bridge().get_tool_definitions();
    let file_read = tools
        .iter()
        .find(|t| t.function.name == "file_read")
        .unwrap();

    assert_eq!(file_read.function.name, "file_read");
    assert!(file_read.function.description.contains("Read"));
    assert!(file_read.function.parameters["properties"]["path"]["type"] == "string");
}

#[test]
fn test_tui_bridge_with_good_config() {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("good-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 8192,
            timeout: 120,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    let agent_bridge = AgentBridge::new(config).unwrap();
    let (_tx, rx) = mpsc::unbounded_channel();

    let tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

    // Verify the bridge was created successfully
    assert_eq!(tui_bridge.state(), TuiState::Idle);
    assert!(tui_bridge.agent_bridge().is_ready());
    assert_eq!(
        tui_bridge.agent_bridge().model(),
        "claude-3-5-sonnet-20241022"
    );
    assert_eq!(tui_bridge.agent_bridge().max_tokens(), 8192);
    assert_eq!(tui_bridge.agent_bridge().timeout(), 120);
}

#[test]
fn test_tui_bridge_from_string_config() {
    // This test verifies that the agent bridge can be created from a config
    // The actual config loading happens in the REPL, so we just verify the bridge works
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: Some("string-config-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            max_tokens: 4096,
            timeout: 60,
        },
    );
    let config = LlmConfig {
        provider: "anthropic".to_string(),
        providers,
    };

    let agent_bridge = AgentBridge::new(config).unwrap();
    let (_tx, rx) = mpsc::unbounded_channel();

    let tui_bridge = TuiBridge::new(agent_bridge, rx).unwrap();

    // Verify the bridge was created and is ready
    assert!(tui_bridge.agent_bridge().is_ready());
}
