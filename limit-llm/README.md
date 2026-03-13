# limit-llm

[![Crates.io](https://img.shields.io/crates/v/limit-llm.svg)](https://crates.io/crates/limit-llm)
[![Docs.rs](https://docs.rs/limit-llm/badge.svg)](https://docs.rs/limit-llm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Multi-provider LLM client for Rust with streaming support.**

Unified API for Anthropic Claude, OpenAI, and z.ai models with built-in token tracking, state persistence, and automatic model handoff.

Part of the [Limit](https://github.com/marioidival/limit) ecosystem.

## Features

- **Multi-provider support**: Anthropic Claude, OpenAI GPT, z.ai
- **Streaming responses**: Async streaming with `futures::Stream`
- **Token tracking**: SQLite-based usage tracking and cost estimation
- **State persistence**: Serialize/restore conversation state
- **Model handoff**: Automatic fallback between providers
- **Tool calling**: Full function/tool support for supported providers
- **Type-safe**: Full Rust type system with serde integration

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
limit-llm = "0.0.25"
```

## Quick Start

```rust
use limit_llm::{AnthropicClient, Message, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AnthropicClient::from_env()?;
    
    let messages = vec![
        Message {
            role: Role::User,
            content: "Hello, Claude!".to_string(),
            ..Default::default()
        }
    ];
    
    let response = client.complete(messages).await?;
    println!("{}", response.content);
    
    Ok(())
}
```

### Streaming

```rust
use futures::StreamExt;

let mut stream = client.complete_stream(messages).await?;

while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(ProviderResponseChunk::Text(text)) => print!("{}", text),
        Ok(ProviderResponseChunk::Thinking(thought)) => eprintln!("[thinking] {}", thought),
        Err(e) => eprintln!("Error: {}", e),
        _ => {}
    }
}
```

### Tool Calling

```rust
use limit_llm::{Tool, ToolFunction, FunctionCall};

let tools = vec![Tool {
    name: "get_weather".to_string(),
    description: Some("Get current weather".to_string()),
    input_schema: json!({
        "type": "object",
        "properties": {
            "location": {"type": "string"}
        },
        "required": ["location"]
    }),
}];

let response = client.complete_with_tools(messages, tools).await?;
```

## API

### Providers

| Provider | Client | Streaming | Tools | Thinking |
|----------|--------|-----------|-------|----------|
| Anthropic Claude | `AnthropicClient` | ✓ | ✓ | ✓ |
| OpenAI | `OpenAiProvider` | ✓ | ✓ | — |
| z.ai | `ZaiProvider` | ✓ | ✓ | ✓ |
| Local/Ollama | `LocalProvider` | ✓ | — | — |

### Core Types

- `Message` — Chat message with role, content, and tool calls
- `Role` — User, Assistant, or System
- `Tool` / `ToolCall` — Function calling definitions
- `Usage` — Token counting for prompt/completion
- `Response` — Complete response with content and metadata

### Advanced Features

```rust
use limit_llm::{ModelHandoff, TrackingDb, StatePersistence};

// Track usage across sessions
let tracking = TrackingDb::new("~/.limit/tracking.db")?;
tracking.record_usage("claude-3-5-sonnet", &usage)?;

// Persist conversation state
let persistence = StatePersistence::new("~/.limit/state/")?;
persistence.save("session-1", &messages)?;

// Automatic model fallback
let handoff = ModelHandoff::new()
    .with_primary("claude-3-5-sonnet")
    .with_fallback("gpt-4o");
```

## Configuration

Environment variables:

```bash
ANTHROPIC_API_KEY=your-key      # For Claude
OPENAI_API_KEY=your-key          # For GPT
ZAI_API_KEY=your-key             # For z.ai
```

Or use `Config` for programmatic configuration:

```rust
use limit_llm::{Config, ProviderConfig};

let config = Config {
    provider: ProviderConfig::Anthropic {
        api_key: "your-key".to_string(),
        model: "claude-3-5-sonnet".to_string(),
    },
    ..Default::default()
};
```

## License

MIT
