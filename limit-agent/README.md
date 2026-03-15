# limit-agent

[![Crates.io](https://img.shields.io/crates/v/limit-agent.svg)](https://crates.io/crates/limit-agent)
[![Docs.rs](https://docs.rs/limit-agent/badge.svg)](https://docs.rs/limit-agent)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Agent runtime for AI applications with tool registry and Docker sandbox.**

Build autonomous AI agents that can execute tools, run code in isolated containers, and maintain state across conversations.

Part of the [Limit](https://github.com/marioidival/limit) ecosystem.

## Why This Exists

Building AI agents requires a flexible tool execution system that's both powerful and safe. `limit-agent` provides a production-ready runtime with Docker sandboxing, parallel execution, and persistent state—so you can focus on your agent's logic, not infrastructure.

## Features

- **Tool Registry**: Define, register, and execute tools dynamically with async/await
- **Docker Sandbox**: Isolated execution environment for untrusted code
- **Parallel Execution**: Run multiple tools concurrently for better performance
- **Event-driven**: Subscribe to agent lifecycle events for logging and monitoring
- **State Management**: Persist and restore agent state across sessions
- **LLM Integration**: Works seamlessly with `limit-llm`

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
limit-agent = "0.0.27"
```

**Requirements**: Rust 1.70+, tokio runtime, Docker (optional, for sandbox)

## Quick Start

### Define a Custom Tool

```rust
use async_trait::async_trait;
use limit_agent::{Tool, AgentError};
use serde_json::{json, Value};

struct WeatherTool;

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }
    
    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let location = args["location"].as_str().unwrap_or("Unknown");
        // Fetch weather data from API...
        Ok(json!({
            "location": location,
            "temp": 22,
            "condition": "sunny"
        }))
    }
}
```

### Register and Execute

```rust
use limit_agent::ToolRegistry;
# use async_trait::async_trait;
# use limit_agent::{Tool, AgentError};
# use serde_json::{json, Value};
# struct WeatherTool;
# #[async_trait]
# impl Tool for WeatherTool {
#     fn name(&self) -> &str { "get_weather" }
#     async fn execute(&self, args: Value) -> Result<Value, AgentError> {
#         Ok(json!({"temp": 22}))
#     }
# }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ToolRegistry::new();
    
    // Register tools
    registry.register(WeatherTool);
    
    // Execute tool by name
    let result = registry
        .execute("get_weather", json!({ "location": "São Paulo" }))
        .await?;
    
    println!("Weather: {:?}", result);
    
    Ok(())
}
```

## Parallel Execution

Execute multiple tools concurrently for better performance:

```rust
# use limit_agent::ToolRegistry;
# use serde_json::json;
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let registry = ToolRegistry::new();
// ... register tools ...

// Execute multiple tools in parallel
let results = registry.execute_all(vec![
    ("get_weather", json!({ "location": "Tokyo" })),
    ("get_weather", json!({ "location": "London" })),
    ("read_file", json!({ "path": "/tmp/data.txt" })),
]).await?;

for result in results {
    println!("{:?}", result);
}
# Ok(())
# }
```

## Docker Sandbox

Run untrusted code in isolated Docker containers:

```rust,no_run
use limit_agent::sandbox::{DockerSandbox, SandboxConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = DockerSandbox::new(SandboxConfig {
        image: "python:3.11-slim".to_string(),
        memory_mb: 256,
        timeout_secs: 30,
        network_disabled: true,
    })?;
    
    // Execute code safely in container
    let output = sandbox.run_code(r#"
        print("Hello from sandbox!")
        x = 2 + 2
        print(f"Result: {x}")
    "#).await?;
    
    println!("{}", output.stdout);
    
    Ok(())
}
```

### Sandbox Security Features

| Feature | Description |
|---------|-------------|
| Network isolation | Container has no network access |
| Memory limit | Configurable memory ceiling (default: 256MB) |
| Timeout | Execution time limit (default: 30s) |
| Read-only mount | Project directory mounted read-only |
| Non-root user | Container runs as unprivileged user |

## Event System

Subscribe to agent lifecycle events for logging, monitoring, or debugging:

```rust
use limit_agent::events::{EventBus, Event};

let events = EventBus::new();

events.subscribe(|event| {
    match event {
        Event::ToolStarted { name, args } => {
            println!("Tool {} started with {:?}", name, args);
        }
        Event::ToolCompleted { name, result, duration } => {
            println!("Tool {} completed in {:?}", name, duration);
        }
        Event::Error { tool, message } => {
            eprintln!("Error in {}: {}", tool, message);
        }
        _ => {}
    }
});
```

## State Management

Persist and restore agent state across sessions:

```rust,no_run
use limit_agent::state::StateManager;

let state = StateManager::new("~/.limit/agent-state/")?;

// Save current state
state.save("session-123", &agent_state)?;

// Restore state later
let restored = state.load("session-123")?;
```

## Core Types

| Type | Description |
|------|-------------|
| `Tool` | Trait for defining executable tools |
| `ToolRegistry` | Registry for managing and executing tools |
| `DockerSandbox` | Isolated execution environment |
| `StateManager` | Persist/restore agent state |
| `EventBus` | Event subscription system |

### Built-in Tools

- `EchoTool` — Simple echo for testing the tool pipeline

## API Reference

See [docs.rs/limit-agent](https://docs.rs/limit-agent) for full API documentation.

## Examples

```bash
# Run examples
cargo run --example basic
cargo run --example parallel
cargo run --example sandbox
```

## License

MIT © [Mário Idival](https://github.com/marioidival)
