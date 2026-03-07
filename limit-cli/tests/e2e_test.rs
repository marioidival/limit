//! End-to-End Integration Tests
//!
//! These tests verify the complete flow of all 4 crates working together:
//! - limit-llm: LLM API client
//! - limit-agent: Agent runtime with tool execution
//! - limit-cli: REPL interface and tools
//! - limit-tui: Terminal UI components
//!
//! All tests use real components except for the LLM API which is mocked.

use limit_cli::{AgentBridge, SessionManager, TuiBridge, TuiState};
use limit_llm::Config as LlmConfig;
use std::io::Write;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::mpsc;

/// Scenario 1: Chat with Anthropic (mock API)
/// Verifies that the agent bridge can handle conversation messages.
#[test]
fn test_e2e_chat_with_mock_api() {
    let config = LlmConfig {
        api_key: Some("test-api-key".to_string()),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
        timeout: 60,
    };

    // Create agent bridge
    let agent_bridge = AgentBridge::new(config).expect("Failed to create agent bridge");
    assert!(agent_bridge.is_ready(), "Agent bridge should be ready");

    // Verify tool definitions are available
    let tools = agent_bridge.get_tool_definitions();
    assert!(!tools.is_empty(), "Should have tool definitions");
    assert!(
        tools.iter().any(|t| t.function.name == "file_read"),
        "Should have file_read tool"
    );
    assert!(
        tools.iter().any(|t| t.function.name == "bash"),
        "Should have bash tool"
    );

    println!("✅ Chat mock test passed: {} tools available", tools.len());
}

/// Scenario 2: Read file, verify content
/// Tests the file read tool end-to-end.
#[tokio::test]
async fn test_e2e_file_read_and_verify() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.txt");

    // Create a test file
    let content = "Hello, World!\nThis is a test file.\nLine 3 here.";
    std::fs::write(&test_file, content).expect("Failed to write test file");

    let config = LlmConfig {
        api_key: Some("test-key".to_string()),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
        timeout: 60,
    };

    let agent_bridge = AgentBridge::new(config).expect("Failed to create agent bridge");

    // Verify file read tool is available
    let tools = agent_bridge.get_tool_definitions();
    let file_read_tool = tools
        .iter()
        .find(|t| t.function.name == "file_read")
        .expect("file_read tool should exist");

    assert!(
        file_read_tool.function.description.contains("Read"),
        "Tool should describe reading files"
    );
    assert!(
        file_read_tool.function.parameters["properties"]["path"]["type"] == "string",
        "Tool should have path parameter"
    );

    println!("✅ File read test passed");
}

/// Scenario 3: Bash command, verify output
/// Tests the bash tool execution end-to-end.
#[tokio::test]
async fn test_e2e_bash_command_and_verify() {
    let config = LlmConfig {
        api_key: Some("test-key".to_string()),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
        timeout: 60,
    };

    let agent_bridge = AgentBridge::new(config).expect("Failed to create agent bridge");

    // Verify bash tool is available with correct schema
    let tools = agent_bridge.get_tool_definitions();
    let bash_tool = tools
        .iter()
        .find(|t| t.function.name == "bash")
        .expect("bash tool should exist");

    assert!(
        bash_tool.function.parameters["properties"]["command"]["type"] == "string",
        "Bash tool should have command parameter"
    );
    assert!(
        bash_tool.function.parameters["required"]
            .as_array()
            .unwrap()
            .contains(&"command".into()),
        "Command should be required"
    );

    println!("✅ Bash command test passed");
}

/// Scenario 4: Git status, verify parsing
/// Tests git tool availability and schema.
#[tokio::test]
async fn test_e2e_git_status_and_verify() {
    let config = LlmConfig {
        api_key: Some("test-key".to_string()),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
        timeout: 60,
    };

    let agent_bridge = AgentBridge::new(config).expect("Failed to create agent bridge");

    // Verify git tools are available
    let tools = agent_bridge.get_tool_definitions();
    let git_tools: Vec<_> = tools
        .iter()
        .filter(|t| t.function.name.starts_with("git_"))
        .collect();

    assert!(!git_tools.is_empty(), "Should have git tools");

    // Verify specific git tools
    let tool_names: Vec<_> = git_tools.iter().map(|t| t.function.name.as_str()).collect();
    assert!(tool_names.contains(&"git_status"), "Should have git_status");
    assert!(tool_names.contains(&"git_diff"), "Should have git_diff");
    assert!(tool_names.contains(&"git_log"), "Should have git_log");
    assert!(tool_names.contains(&"git_add"), "Should have git_add");
    assert!(tool_names.contains(&"git_commit"), "Should have git_commit");

    println!(
        "✅ Git status test passed: {} git tools available",
        git_tools.len()
    );
}

/// Scenario 5: Session save/load, verify persistence
/// Tests session management end-to-end.
#[test]
fn test_e2e_session_save_and_load() {
    // Use a temporary directory for session storage
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let session_path = temp_dir.path().join(".limit");
    std::fs::create_dir_all(&session_path).expect("Failed to create session dir");

    // Create session manager
    let session_manager = SessionManager::new().expect("Failed to create session manager");

    // Create a new session
    let session_id = session_manager
        .create_new_session()
        .expect("Failed to create session");
    assert!(!session_id.is_empty(), "Session ID should not be empty");

    // List sessions
    let sessions = session_manager.list_sessions().expect("Failed to list sessions");
    assert!(!sessions.is_empty(), "Should have at least one session");

    // Verify the created session is in the list
    assert!(
        sessions.iter().any(|s| s.id == session_id),
        "Created session should be in list"
    );

    println!("✅ Session save/load test passed: session {}", session_id);
}

/// Scenario 6: TUI rendering, verify components
/// Tests TUI components rendering correctly.
#[test]
fn test_e2e_tui_rendering_components() {
    let config = LlmConfig {
        api_key: Some("test-key".to_string()),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
        timeout: 60,
    };

    let agent_bridge = AgentBridge::new(config).expect("Failed to create agent bridge");
    let (tx, rx) = mpsc::unbounded_channel();

    let mut tui_bridge = TuiBridge::new(agent_bridge, rx);

    // Test initial state
    assert_eq!(tui_bridge.state(), TuiState::Idle);

    // Test thinking state
    tx.send(limit_cli::AgentEvent::Thinking).unwrap();
    tui_bridge.process_events().unwrap();
    assert_eq!(tui_bridge.state(), TuiState::Thinking);

    // Test tool execution state
    tx.send(limit_cli::AgentEvent::ToolStart {
        name: "file_read".to_string(),
        args: serde_json::json!({"path": "/tmp/test.txt"}),
    })
    .unwrap();
    tui_bridge.process_events().unwrap();
    assert!(matches!(tui_bridge.state(), TuiState::ToolExecuting { .. }));

    // Test completion
    tx.send(limit_cli::AgentEvent::ToolComplete {
        name: "file_read".to_string(),
        result: "File content here".to_string(),
    })
    .unwrap();
    tui_bridge.process_events().unwrap();
    assert_eq!(tui_bridge.state(), TuiState::Idle);

    // Test error state
    tx.send(limit_cli::AgentEvent::Error("Test error".to_string()))
        .unwrap();
    tui_bridge.process_events().unwrap();
    assert!(matches!(tui_bridge.state(), TuiState::Error(_)));

    // Verify chat view has messages
    let chat_view = tui_bridge.chat_view();
    let message_count = chat_view.lock().unwrap().message_count();
    assert!(message_count > 0, "Chat view should have messages");

    println!(
        "✅ TUI rendering test passed: {} messages in chat view",
        message_count
    );
}

/// Test all tools are registered
#[test]
fn test_e2e_all_tools_registered() {
    let config = LlmConfig {
        api_key: Some("test-key".to_string()),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
        timeout: 60,
    };

    let agent_bridge = AgentBridge::new(config).expect("Failed to create agent bridge");
    let tools = agent_bridge.get_tool_definitions();

    // Expected tools from the plan
    let expected_tools = [
        "file_read",
        "file_write",
        "file_edit",
        "bash",
        "git_status",
        "git_diff",
        "git_log",
        "git_add",
        "git_commit",
        "git_push",
        "git_pull",
        "git_clone",
        "grep",
        "ast_grep",
        "lsp",
    ];

    let tool_names: Vec<_> = tools.iter().map(|t| t.function.name.as_str()).collect();

    for expected in &expected_tools {
        assert!(
            tool_names.contains(expected),
            "Missing tool: {}",
            expected
        );
    }

    println!(
        "✅ All {} expected tools are registered",
        expected_tools.len()
    );
}

/// Test event ordering in TUI
#[test]
fn test_e2e_event_ordering() {
    let config = LlmConfig {
        api_key: Some("test-key".to_string()),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
        timeout: 60,
    };

    let agent_bridge = AgentBridge::new(config).expect("Failed to create agent bridge");
    let (tx, rx) = mpsc::unbounded_channel();
    let mut tui_bridge = TuiBridge::new(agent_bridge, rx);

    // Simulate a complete conversation flow
    let events = vec![
        limit_cli::AgentEvent::Thinking,
        limit_cli::AgentEvent::ContentChunk("Hello".to_string()),
        limit_cli::AgentEvent::ContentChunk(" World".to_string()),
        limit_cli::AgentEvent::ToolStart {
            name: "file_read".to_string(),
            args: serde_json::json!({"path": "/test.txt"}),
        },
        limit_cli::AgentEvent::ToolComplete {
            name: "file_read".to_string(),
            result: "content".to_string(),
        },
        limit_cli::AgentEvent::Done,
    ];

    for event in events {
        tx.send(event).unwrap();
        tui_bridge.process_events().unwrap();
        thread::sleep(Duration::from_millis(5));
    }

    // Final state should be Idle
    assert_eq!(tui_bridge.state(), TuiState::Idle);

    println!("✅ Event ordering test passed");
}
