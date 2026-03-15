# limit-llm

[![Crates.io](https://img.shields.io/crates/v/limit-llm.svg)](https://crates.io/crates/limit-llm)
[![Docs.rs](https://docs.rs/limit-llm/badge.svg)](https://docs.rs/limit-llm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Multi-provider LLM client for Rust with streaming support.**

Unified API for Anthropic Claude, OpenAI, z.ai, and local LLMs with built-in token tracking, state persistence, and automatic model handoff.

Part of the [Limit](https://github.com/marioidival/limit) ecosystem.

## Why This Exists

Building AI applications shouldn't require learning different APIs for each LLM provider. `limit-llm` provides a single, consistent interface that works across Anthropic Claude, OpenAI GPT, z.ai GLM, and local models—so you can switch providers without rewriting code.

## Features

- **Multi-provider support**: Anthropic Claude, OpenAI GPT, z.ai GLM, and local LLMs (Ollama, LM Studio, vLLM)
- **Streaming responses**: Async streaming with `futures::Stream` for real-time output
- **Token tracking**: SQLite-based usage tracking with cost estimation
- **State persistence**: Serialize/restore conversation state with bincode
- **Model handoff**: Automatic fallback between providers on failure
- **Tool calling**: Full function/tool support for all compatible providers
- **Thinking mode**: Extended reasoning support (Claude, z.ai)
- **Type-safe**: Full Rust type system with serde integration

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
limit-llm = "0.0.27"
```

**Requirements**: Rust 1.70+, tokio runtime

## Quick Start

### Basic Usage

```rust,no_run
use limit_llm::{AnthropicClient, Message, Role, LlmProvider};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AnthropicClient::new(
        std::env::var("ANTHROPIC_API_KEY")?,
        None,  // default base URL
        60,    // timeout in seconds
        "claude-sonnet-4-6-20260217",
        4096,  // max tokens
    );
    
    let messages = vec![
        Message {
            role: Role::User,
            content: Some("Hello, Claude!".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }
    ];
    
    // Stream the response
    let mut stream = client.send(messages, vec![]).await?;
    
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(limit_llm::ProviderResponseChunk::ContentDelta(text)) => print!("{}", text),
            Ok(limit_llm::ProviderResponseChunk::Done(usage)) => {
                println!("\nTokens: {} in, {} out", usage.input_tokens, usage.output_tokens);
            }
            Err(e) => eprintln!("Error: {}", e),
            _ => {}
        }
    }
    
    Ok(())
}
```

### With Configuration File

```rust,no_run
use limit_llm::{Config, ProviderFactory, LlmProvider};

// Load from ~/.limit/config.toml
let config = Config::load()?;

// Create provider from config
let provider = ProviderFactory::from_config(&config)?;

// Use the provider
let stream = provider.send(vec![], vec![]).await?;
```

## Providers

| Provider | Client | Streaming | Tools | Thinking |
|----------|--------|-----------|-------|----------|
| Anthropic Claude | `AnthropicClient` | ✓ | ✓ | ✓ |
| OpenAI GPT | `OpenAiProvider` | ✓ | ✓ | — |
| z.ai GLM | `ZaiProvider` | ✓ | ✓ | ✓ |
| Local/Ollama | `LocalProvider` | ✓ | — | — |

### Provider Configuration

```toml
# ~/.limit/config.toml
provider = "anthropic"

[providers.anthropic]
model = "claude-sonnet-4-6-20260217"
max_tokens = 4096
timeout = 60
```

### Environment Variables

| Variable | Provider |
|----------|----------|
| `ANTHROPIC_API_KEY` | Anthropic Claude |
| `OPENAI_API_KEY` | OpenAI |
| `ZAI_API_KEY` | z.ai |

## Tool Calling

```rust,no_run
use limit_llm::{Tool, ToolFunction, Message, Role, AnthropicClient, LlmProvider};
use serde_json::json;

let tools = vec![Tool {
    tool_type: "function".to_string(),
    function: ToolFunction {
        name: "get_weather".to_string(),
        description: "Get current weather for a location".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        }),
    },
}];

let messages = vec![Message {
    role: Role::User,
    content: Some("What's the weather in Tokyo?".to_string()),
    tool_calls: None,
    tool_call_id: None,
}];

let client = AnthropicClient::from_env()?;
let stream = client.send(messages, tools).await?;
```

## Token Tracking

```rust,no_run
use limit_llm::TrackingDb;

let tracking = TrackingDb::new("~/.limit/tracking.db")?;

// Record usage (automatically done by clients)
tracking.record_usage("claude-sonnet-4-6-20260217", 100, 50)?;

// Get statistics
let stats = tracking.get_stats()?;
println!("Total cost: ${:.4}", stats.total_cost);
```

## State Persistence

```rust,no_run
use limit_llm::{StatePersistence, Message};

let persistence = StatePersistence::new("~/.limit/state/")?;

// Save conversation
persistence.save("session-123", &messages)?;

// Restore later
let restored = persistence.load::<Vec<Message>>("session-123")?;
```

## Model Handoff

Automatic fallback between providers:

```rust,no_run
use limit_llm::ModelHandoff;

let handoff = ModelHandoff::new()
    .with_primary("claude-sonnet-4-6-20260217")
    .with_fallback("gpt-5.4")
    .with_fallback("glm-5");

// Automatically falls back if primary fails
let response = handoff.complete(messages).await?;
```

## Core Types

| Type | Description |
|------|-------------|
| `Message` | Chat message with role, content, and tool calls |
| `Role` | User, Assistant, System, or Tool |
| `Tool` / `ToolCall` | Function calling definitions |
| `Usage` | Token counting for prompt/completion |
| `Response` | Complete response with content and metadata |

## API Reference

See [docs.rs/limit-llm](https://docs.rs/limit-llm) for full API documentation.

## Examples

```bash
# Run examples
cargo run --example basic
cargo run --example streaming
cargo run --example tool_calling
```

## License

MIT © [Mário Idival](https://github.com/marioidival)
