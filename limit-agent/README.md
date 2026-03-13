# limit-agent

[![Crates.io](https://img.shields.io/crates/v/limit-agent.svg)](https://crates.io/crates/limit-agent)
[![Docs.rs](https://docs.rs/limit-agent/badge.svg)](https://docs.rs/limit-agent)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Agent runtime for AI applications with tool registry and Docker sandbox.**

Build autonomous AI agents that can execute tools, run code in isolated containers, and maintain state across conversations.

Part of the [Limit](https://github.com/marioidival/limit) ecosystem.

## Features

- **Tool registry**: Define, register, and execute tools dynamically
- **Docker sandbox**: Isolated execution environment for untrusted code
- **Parallel execution**: Run multiple tools concurrently with async/await
- **Event-driven**: Subscribe to agent lifecycle events
- **State management**: Persist and restore agent state
- **LLM integration**: Works seamlessly with `limit-llm`

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
limit-agent = "0.0.25"
```

## Quick Start

### Define a Tool

```rust
use async_trait::async_trait;
use limit_agent::{Tool, ToolRegistry};
use serde_json::Value;

struct WeatherTool;

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }
    
    fn description(&self) -> &str {
        "Get current weather for a location"
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        })
    }
    
    async fn execute(&self, args: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let location = args["location"].as_str().unwrap();
        // Fetch weather data...
        Ok(json!({ "location": location, "temp": 22, "condition": "sunny" }))
    }
}
```

### Register and Execute

```rust
use limit_agent::ToolRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ToolRegistry::new();
    
    registry.register(WeatherTool);
    registry.register(FileReadTool);
    registry.register(ShellTool);
    
    // Execute tool by name
    let result = registry
        .execute("get_weather", json!({ "location": "São Paulo" }))
        .await?;
    
    println!("Weather: {:?}", result);
    
    Ok(())
}
```

### Parallel Execution

```rust
// Execute multiple tools concurrently
let results = registry.execute_all(vec![
    ("get_weather", json!({ "location": "Tokyo" })),
    ("get_weather", json!({ "location": "London" })),
    ("read_file", json!({ "path": "/tmp/data.txt" })),
]).await?;

for result in results {
    println!("{:?}", result);
}
```

## Docker Sandbox

Run untrusted code in isolated Docker containers:

```rust
use limit_agent::sandbox::{DockerSandbox, SandboxConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = DockerSandbox::new(SandboxConfig {
        image: "python:3.11-slim".to_string(),
        memory_mb: 256,
        timeout_secs: 30,
        network_disabled: true,
    })?;
    
    // Execute code in container
    let output = sandbox.run_code(r#"
        print("Hello from sandbox!")
        x = 2 + 2
        print(f"Result: {x}")
    "#).await?;
    
    println!("{}", output.stdout);
    
    Ok(())
}
```

## Event System

Subscribe to agent lifecycle events:

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

```rust
use limit_agent::state::StateManager;

let state = StateManager::new("~/.limit/agent-state/")?;

// Save current state
state.save("session-123", &agent_state)?;

// Restore state
let restored = state.load("session-123")?;
```

## API Reference

### Core Types

| Type | Description |
|------|-------------|
| `Tool` | Trait for defining executable tools |
| `ToolRegistry` | Registry for managing and executing tools |
| `DockerSandbox` | Isolated execution environment |
| `StateManager` | Persist/restore agent state |
| `EventBus` | Event subscription system |

### Built-in Tools

- `EchoTool` — Simple echo for testing

## License

MIT
