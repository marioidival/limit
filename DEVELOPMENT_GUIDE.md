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

Limit is a Cargo workspace with 5 crates:

| Crate | Purpose |
|-------|---------|
| `limit-llm` | Multi-provider LLM client (Anthropic, OpenAI, z.ai, local) |
| `limit-agent` | Agent runtime with tool registry and Docker sandbox |
| `limit-tldr` | Code analysis library (AST, call graph, CFG, DFG, PDG) |
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
├── limit-tldr/             # Code analysis library
│   ├── src/
│   │   ├── lib.rs          # TLDR main API
│   │   ├── layers/         # Analysis layers
│   │   │   ├── ast.rs      # Layer 1: Structure
│   │   │   ├── call_graph.rs # Layer 2: Dependencies
│   │   │   ├── cfg.rs      # Layer 3: Control flow
│   │   │   ├── dfg.rs      # Layer 4: Data flow
│   │   │   └── pdg.rs      # Layer 5: Program dependence
│   │   ├── parsers/        # Tree-sitter parsers
│   │   ├── semantic/       # Semantic search (embeddings)
│   │   └── cache/          # Incremental cache
│   └── tests/
│
├── limit-cli/              # CLI application
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── repl.rs         # REPL interface
│   │   ├── tui_bridge.rs   # TUI integration
│   │   ├── session.rs      # Session persistence
│   │   └── tools/          # Tool implementations
│   │       ├── tldr.rs     # tldr_analyze tool
│   │       ├── analysis.rs # grep, ast_grep, lsp
│   │       ├── bash.rs
│   │       ├── git.rs
│   │       ├── web.rs
│   │       └── browser/    # Browser automation
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

    async fn execute(&self, args: serde_json::Value) -> Result<Value, AgentError> {
        let input: String = args["input"].as_str()
            .ok_or_else(|| AgentError::ToolError("Missing input".into()))?
            .to_string();

        // Tool implementation
        Ok(json!({ "result": input }))
    }
}
```

2. **Register tool** in `limit-cli/src/agent_bridge.rs`:

```rust
registry.register(Box::new(MyTool));
```

3. **Add tests** in `limit-cli/src/tools/` or `limit-cli/tests/`

### Example: tldr_analyze Tool

The `tldr_analyze` tool (`limit-cli/src/tools/tldr.rs`) wraps `limit-tldr` library:

```rust
pub struct TldrTool {
    tldr: Arc<RwLock<Option<TLDR>>>,
}

#[async_trait]
impl Tool for TldrTool {
    fn name(&self) -> &str { "tldr_analyze" }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let params: TldrParams = serde_json::from_value(args)?;

        match params.analysis_type {
            AnalysisType::Search => self.search(&params),
            AnalysisType::Context => self.get_context(&params),
            AnalysisType::Source => self.get_source(&params),
            // ...
        }
    }
}
```

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

## limit-tldr: Code Analysis Library

The `limit-tldr` crate provides token-efficient code analysis with 95% savings vs raw code. It's used by the `tldr_analyze` tool.

### Architecture (5 Layers)

| Layer | File | Purpose |
|-------|------|---------|
| **1** | `layers/ast.rs` | Structure — "What functions exist?" |
| **2** | `layers/call_graph.rs` | Dependencies — "Who calls what?" |
| **3** | `layers/cfg.rs` | Control Flow — "How complex is this?" |
| **4** | `layers/dfg.rs` | Data Flow — "Where does this value come from?" |
| **5** | `layers/pdg.rs` | Program Dependence — "What affects this line?" |

### Key Components

- **TLDR** (`lib.rs`) — Main API entry point
- **ParseCoordinator** (`coordinator.rs`) — Discovers and parses files in parallel
- **CacheManager** (`cache/`) — Incremental cache with BLAKE3 hashing
- **SemanticIndex** (`semantic/`) — Optional semantic search with embeddings
- **TreeSitterParser** (`parsers/`) — Multi-language parsing (Rust, Python, JS, TS, Go, Java, C, C++)

### Analysis Types

| Type | Method | Description |
|------|--------|-------------|
| `search` | `search()` | Find functions by name/keyword |
| `context` | `get_context()` | Get function dependencies + callers |
| `source` | `find_function()` | Extract function implementation |
| `impact` | `get_impact()` | Who calls this function? |
| `cfg` | `get_cfg()` | Control flow graph (complexity) |
| `dfg` | `get_dfg()` | Data flow graph (variable origins) |
| `dead_code` | `find_dead_code()` | Find unreachable functions |
| `architecture` | `detect_architecture()` | Detect module layers |

### Adding Language Support

To add a new language to limit-tldr:

1. **Add tree-sitter dependency** in `limit-tldr/Cargo.toml`:
```toml
tree-sitter-my-language = "0.23"
```

2. **Register language** in `limit-tldr/src/parsers/tree_sitter.rs`:
```rust
pub fn get_ts_language(&self, lang: Language) -> Option<TsLanguage> {
    match lang {
        Language::MyLanguage => Some(tree_sitter_my_language::language()),
        // ...
    }
}
```

3. **Add language enum** in `limit-tldr/src/types.rs`:
```rust
pub enum Language {
    MyLanguage,
    // ...
}
```

4. **Add tests** in `limit-tldr/tests/`

---

## TUI Features

### File Autocomplete

The TUI includes a file autocomplete feature triggered by typing `@` in the input field. This is implemented in:

**Backend (`limit-cli/src/file_finder.rs`):**
- `FileFinder` — Scans project directory with `.gitignore` support
- Uses `ignore` crate from ripgrep for smart filtering
- Caches file list for 5 seconds (TTL configurable)
- Fuzzy matching powered by `frizbee` crate

**Frontend (`limit-tui/src/components/file_autocomplete.rs`):**
- `FileAutocompleteWidget` — Renders popup with file suggestions
- `FileMatchData` — Data structure for file matches
- `calculate_popup_area` — Positions popup above input

**Integration (`limit-cli/src/tui_bridge.rs`):**
- `FileAutocompleteState` — Tracks autocomplete state in `TuiApp`
- Handles keyboard navigation (↑/↓/Enter/Tab/Esc)
- Inserts selected path into input field

**Testing:**
- Unit tests: `limit-cli/src/file_finder.rs` (4 tests)
- Unit tests: `limit-tui/src/components/file_autocomplete.rs` (3 tests)
- Integration test: `limit-cli/tests/tui_integration.rs::test_file_autocomplete_integration`

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
| `tldr_analyze` returns empty | Run `tldr.warm()` first to build indexes |
| Semantic search unavailable | Install `fastembed` feature: `cargo build --features semantic` |
| `ast_grep` not found | Install: `brew install ast-grep` or `cargo install ast-grep` |
| LSP operations fail | Install language server (rust-analyzer, typescript-language-server, pylsp) |

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
