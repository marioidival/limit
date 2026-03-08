# limit

A Rust-based code agent with multi-provider LLM support (Anthropic Claude, OpenAI, z.ai). Features a REPL interface with file operations, bash execution, git operations, code analysis tools, and a terminal UI.

## Features

- **Multi-Provider LLM Support** - Anthropic Claude, OpenAI, and z.ai with streaming API
- **15 Built-in Tools** - File I/O, Bash, Git, and code analysis
- **Session Persistence** - Auto-save/restore conversation history
- **Token Tracking** - SQLite-based usage tracking
- **Terminal UI** - Ratatui-based TUI with virtual DOM rendering
- **Docker Sandbox** - Optional containerized tool execution

## Crates

| Crate | Description |
|-------|-------------|
| `limit-llm` | Multi-provider LLM client (Anthropic, OpenAI) with streaming, SQLite tracking, binary persistence |
| `limit-agent` | Agent runtime with tool registry, conditional execution, Docker sandbox |
| `limit-cli` | REPL interface with 15 tools, markdown rendering, session management |
| `limit-tui` | Terminal UI with Virtual DOM, flexbox layout, chat/diff views |

## Installation

```bash
# Clone the repository
git clone https://github.com/marioidival/limit.git
cd limit

# Build all crates
cargo build --workspace --release
```

## Configuration

Create a configuration file at `~/.limit/config.toml`:

### Anthropic Claude (Default)

```toml
provider = "anthropic"

[providers.anthropic]
# api_key is optional - falls back to ANTHROPIC_API_KEY env var
api_key = "sk-ant-api03-..."
model = "claude-3-5-sonnet-20241022"
max_tokens = 4096
timeout = 60
# Optional: Custom API endpoint
# base_url = "https://api.custom-provider.com/v1/messages"
```

### OpenAI

```toml
provider = "openai"

[providers.openai]
# api_key is optional - falls back to OPENAI_API_KEY or ZAI_API_KEY env var
api_key = "sk-..."
model = "gpt-4"
max_tokens = 4096
timeout = 300000
```

### z.ai (ZAI Provider)

```toml
provider = "zai"

[providers.zai]
# api_key is optional - falls back to ZAI_API_KEY env var
api_key = "..."
model = "glm-4.7"
# Optional: Custom endpoint (defaults to ZAI coding path)
# base_url = "https://api.z.ai/api/coding/paas/v4/chat/completions"
max_tokens = 4096
timeout = 300000
# Optional: Enable thinking mode (default: false)
# thinking_enabled = true
# Optional: Preserve thinking across turns (default: true)
# clear_thinking = false
```
### Environment Variables

Provider API keys can be set via environment variables as fallback:

- `ANTHROPIC_API_KEY` - For Anthropic Claude
- `OPENAI_API_KEY` - For OpenAI
- `ZAI_API_KEY` - For z.ai provider

## Usage

### REPL Interface

```bash
cargo run --package limit-cli
```

```
limit> Read the file src/main.rs
limit> What does this code do?
limit> /help
limit> /exit
```

### Available Commands

| Command | Description |
|---------|-------------|
| `/exit` | Save session and exit |
| `/clear` | Clear conversation history |
| `/help` | Show available commands |

## Built-in Tools

### File Operations
- **file_read** - Read file contents (max 50MB)
- **file_write** - Write content to file
- **file_edit** - Edit file using diff-based replacement

### Shell
- **bash** - Execute shell commands with timeout

### Git
- **git_status** - Show repository status
- **git_diff** - Show changes
- **git_log** - Show commit history
- **git_add** - Stage files
- **git_commit** - Create commit
- **git_push** - Push to remote
- **git_pull** - Pull from remote
- **git_clone** - Clone repository

### Code Analysis
- **grep** - Search files with regex
- **ast_grep** - AST-aware code search (Rust, TypeScript, Python)
- **lsp** - LSP operations (go-to-definition, find-references)

## Architecture

```
limit/
├── limit-llm/           # LLM API layer
│   ├── client.rs        # Anthropic streaming client
│   ├── openai_provider.rs  # OpenAI/z.ai streaming client
│   ├── provider_factory.rs # Provider factory
│   ├── providers.rs     # LlmProvider trait
│   ├── config.rs        # Configuration loading
│   ├── tracking.rs      # Token usage tracking
│   ├── persistence.rs   # Binary state persistence
│   └── handoff.rs       # Model context handoff
│
├── limit-agent/         # Agent runtime
│   ├── tool.rs          # Tool trait definition
│   ├── registry.rs      # Tool registry
│   ├── executor.rs      # Parallel tool execution
│   ├── sandbox.rs       # Docker sandbox
│   └── state.rs         # Agent state management
│
├── limit-cli/           # CLI application
│   ├── repl.rs          # REPL interface
│   ├── agent_bridge.rs  # LLM-Agent integration
│   ├── tui_bridge.rs    # TUI integration
│   ├── session.rs       # Session persistence
│   └── tools/           # Tool implementations
│
└── limit-tui/           # Terminal UI
    ├── vdom.rs          # Virtual DOM
    ├── backend.rs       # Ratatui backend
    ├── layout.rs        # Flexbox layout
    └── components/      # UI components
        ├── chat.rs      # Chat view
        ├── diff.rs      # Diff view
        ├── progress.rs  # Progress indicators
        └── prompt.rs    # Interactive prompts
```

## Development

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace -- --test-threads=1

# Run specific package tests
cargo test --package limit-cli

# Run E2E integration tests
cargo test --package limit-cli --test e2e_test

# Run with release optimizations
cargo build --workspace --release
```

### Running Examples

```bash
# Chat view demo
cargo run --package limit-tui --example chat_demo

# Diff view demo
cargo run --package limit-tui --example diff_demo

# Progress indicators demo
cargo run --package limit-tui --example progress_demo

# Interactive prompts demo
cargo run --package limit-tui --example prompt_demo
```

## Session Persistence

Sessions are automatically saved to `~/.limit/sessions/` and include:
- Conversation history (messages)
- Session metadata (SQLite)
- Agent state

On startup, the last session is automatically restored.

## Token Tracking

Usage statistics are tracked in `~/.limit/tracking.db`:
- Requests count
- Input/output tokens
- Cost estimation
- Duration metrics

## Docker Sandbox (Optional)

Limit can execute tools in isolated Docker containers:

```bash
# Requires Docker to be installed and running
# Automatically detected and used if available
```

Sandbox features:
- Isolated execution environment
- Read-only project mount
- No network access
- 512MB memory limit
- 60s timeout

## Constraints

The MVP has these intentional limitations:
- Unix-only TUI (no Windows support)
- No syntax highlighting
- No mouse support
- No split views/tabs
- Max 50MB file reads
- Max 50 tool calls per session

## License

MIT
