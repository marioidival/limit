# Development Guide

## Getting Started

### Prerequisites

- **Rust 1.70+** — Install via [rustup](https://rustup.rs/)
- **Unix-like OS** — Linux or macOS (Windows support planned)
- **Docker** (optional) — For sandbox execution

### Clone and Build

```bash
git clone https://github.com/marioidival/limit.git
cd limit
cargo build --workspace
```

---

## Project Structure

Limit is a Cargo workspace with 4 crates:

| Crate | Purpose |
|-------|---------|
| `limit-llm` | Multi-provider LLM client (Anthropic, OpenAI, z.ai) |
| `limit-agent` | Agent runtime with tool registry and Docker sandbox |
| `limit-cli` | CLI application with REPL and TUI modes |
| `limit-tui` | Terminal UI components with Virtual DOM |

### Directory Layout

```
limit/
├── limit-llm/              # LLM API layer
│   ├── src/
│   │   ├── client.rs       # Anthropic streaming client
│   │   ├── openai_provider.rs
│   │   ├── zai_provider.rs
│   │   ├── providers.rs    # LlmProvider trait
│   │   ├── config.rs       # Configuration loading
│   │   ├── tracking.rs     # Token usage tracking
│   │   └── persistence.rs  # Binary state persistence
│   └── tests/
│
├── limit-agent/            # Agent runtime
│   ├── src/
│   │   ├── tool.rs         # Tool trait definition
│   │   ├── registry.rs     # Tool registry
│   │   ├── executor.rs     # Parallel tool execution
│   │   ├── sandbox.rs      # Docker sandbox
│   │   └── state.rs        # Agent state management
│   └── tests/
│
├── limit-cli/              # CLI application
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── repl.rs         # REPL interface
│   │   ├── tui_bridge.rs   # TUI integration
│   │   ├── session.rs      # Session persistence
│   │   └── tools/          # Tool implementations
│   └── tests/
│
└── limit-tui/              # Terminal UI
    ├── src/
    │   ├── vdom.rs         # Virtual DOM
    │   ├── backend.rs      # Ratatui backend
    │   ├── layout.rs       # Flexbox layout
    │   └── components/     # UI components
    └── tests/
```

---

## Development Commands

### Building

```bash
# Build all crates (debug)
cargo build --workspace

# Build with release optimizations
cargo build --workspace --release

# Build specific crate
cargo build --package limit-cli
```

### Testing

```bash
# Run all tests
cargo test --workspace -- --test-threads=1

# Run tests for specific crate
cargo test --package limit-cli
cargo test --package limit-llm
cargo test --package limit-agent
cargo test --package limit-tui

# Run E2E integration tests
cargo test --package limit-cli --test e2e_test

# Run specific test
cargo test --package limit-llm --test providers
```

### Linting & Formatting

```bash
# Run clippy lints
cargo clippy --all-targets

# Format code
cargo fmt

# Check formatting without applying
cargo fmt -- --check
```

### Running

```bash
# Run CLI (TUI mode)
cargo run --package limit-cli

# Run CLI (REPL mode)
cargo run --package limit-cli -- --no-tui
```

---

## Code Style

### Conventional Commits

Use conventional commit messages:

```
feat: add new tool for database operations
fix: resolve memory leak in session persistence
refactor: improve error handling in LLM client
docs: update configuration examples
test: add integration tests for git tools
chore: update dependencies
```

### Pre-commit Checklist

1. `cargo fmt` — Format code
2. `cargo clippy --all-targets` — Fix all warnings
3. `cargo test --workspace` — All tests pass
4. Commit only files you worked on

---

## Architecture Overview

### Data Flow

```
User Input (REPL/TUI)
        │
        ▼
   AgentBridge
        │
        ├──▶ LlmProvider (Anthropic/OpenAI/zai)
        │         │
        │         ▼
        │    Streaming Response
        │         │
        │         ▼
        │    Tool Calls (if needed)
        │         │
        ▼         │
   ToolRegistry ◀┘
        │
        ├──▶ FileTools (read, write, edit)
        ├──▶ BashTool (shell execution)
        ├──▶ GitTools (status, diff, commit, etc.)
        └──▶ AnalysisTools (grep, ast_grep, lsp)
        │
        ▼
   Tool Results → LlmProvider → Response to User
```

### Key Traits

#### `LlmProvider` (limit-llm)

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, messages: &[Message], tools: &[Tool]) 
        -> Result<Response>;
    async fn complete_stream(&self, messages: &[Message], tools: &[Tool]) 
        -> impl Stream<Item = Result<ProviderResponseChunk>>;
}
```

#### `Tool` (limit-agent)

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<String>;
}
```

---

## Adding a New Tool

1. **Create tool struct** in `limit-cli/src/tools/`:

```rust
pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    
    fn description(&self) -> &str {
        "Description of what the tool does"
    }
    
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            },
            "required": ["input"]
        })
    }
    
    async fn execute(&self, args: serde_json::Value) -> Result<String> {
        let input: String = args["input"].as_str()
            .ok_or_else(|| anyhow!("Missing input"))?
            .to_string();
        
        // Tool implementation
        Ok(format!("Result: {}", input))
    }
}
```

2. **Register tool** in `limit-cli/src/agent_bridge.rs`:

```rust
registry.register(Box::new(MyTool));
```

3. **Add tests** in `limit-cli/tests/`

---

## Adding a New LLM Provider

1. **Create provider** in `limit-llm/src/`:

```rust
pub struct MyProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

#[async_trait]
impl LlmProvider for MyProvider {
    // Implement trait methods
}
```

2. **Add to factory** in `limit-llm/src/provider_factory.rs`:

```rust
match provider_name {
    "myprovider" => Box::new(MyProvider::new(config)?),
    // ...
}
```

3. **Add config support** in `limit-llm/src/config.rs`

4. **Add tests** in `limit-llm/tests/`

---

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run --package limit-cli
```

### Log Levels

- `TRACE` — Very verbose, all operations
- `DEBUG` — Detailed debugging info
- `INFO` — General information
- `WARN` — Warnings only
- `ERROR` — Errors only

### Common Issues

| Issue | Solution |
|-------|----------|
| API key not found | Check `~/.limit/config.toml` or env vars |
| Session not loading | Check `~/.limit/sessions/` permissions |
| Docker sandbox fails | Ensure Docker daemon is running |
| Build fails | Run `cargo clean` then rebuild |

---

## Release Process

1. Update version in all `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create git tag: `git tag v0.x.x`
5. Push tag: `git push origin v0.x.x`

---

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make changes and add tests
4. Run: `cargo fmt && cargo clippy --all-targets && cargo test --workspace`
5. Commit with conventional commit message
6. Push and create Pull Request

See [AGENTS.md](AGENTS.md) for project-specific conventions.
