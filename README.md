<div align="center">
  <img src="assets/logo.png" alt="Limit Logo" width="200"/>
</div>

# Limit


[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/limit-cli.svg)](https://crates.io/crates/limit-cli)

**Your AI pair programmer that lives in the terminal.**

Edit files, run commands, analyze code, manage git — all through natural language. Limit connects to Anthropic Claude, OpenAI, or z.ai to help you code, with a beautiful TUI or simple REPL.

## Quick Start

### One-line Install

```bash
curl -fsSL https://raw.githubusercontent.com/marioidival/limit/trunk/install.sh | bash
```

### Install via Cargo

```bash
cargo install limit-cli
```

### Manual Install

```bash
# 1. Clone and build
git clone https://github.com/marioidival/limit.git && cd limit
cargo build --release

# 2. Configure your LLM provider
echo 'provider = "anthropic"' > ~/.limit/config.toml
export ANTHROPIC_API_KEY="your-key-here"

# 3. Run
./target/release/lim
```

That's it! Start chatting with your AI coding assistant.

> Use `--no-tui` flag for text-based REPL: `lim --no-tui`

---

## Features

- **Multi-Provider LLM Support** — Anthropic Claude, OpenAI, z.ai, and local LLMs (Ollama, LM Studio, vLLM)
- **17 Built-in Tools** — File I/O, Bash execution, Git operations, code analysis, web search/fetch
- **Session Persistence** — Auto-save/restore conversation history
- **Token Tracking** — SQLite-based usage tracking with cost estimation
- **Docker Sandbox** — Optional containerized tool execution for isolation
- **LSP Integration** — Go-to-definition, find-references
- **AST-Aware Search** — Code search that understands syntax (Rust, TypeScript, Python)
- **Markdown Rendering** — Rich formatting with syntax-highlighted code blocks

---

## Why Limit?

| Feature | Limit | Aider | Cursor | GitHub Copilot |
|---------|:-----:|:-----:|:------:|:--------------:|
| Terminal-native | ✅ | ✅ | ❌ | ❌ |
| Multi-provider LLM | ✅ | ✅ | ❌ | ❌ |
| Local LLM support | ✅ | ✅ | ❌ | ❌ |
| Docker sandbox | ✅ | ❌ | ❌ | ❌ |
| Session persistence | ✅ | ✅ | ✅ | ✅ |
| Token tracking | ✅ | ❌ | ❌ | ✅ |
| Open source | ✅ | ✅ | ❌ | ❌ |
| AST-aware search | ✅ | ❌ | ✅ | ❌ |
| LSP integration | ✅ | ❌ | ✅ | ✅ |

**Perfect for:**
- 🖥️ Developers who live in the terminal
- 🔒 Privacy-conscious teams wanting Docker isolation
- 🏠 Local LLM enthusiasts running models offline
- 📊 Projects requiring audit trails of AI interactions
- 🔄 Multi-model workflows (switch between Claude, GPT-4, z.ai, local)

---

## Installation

### One-line Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/marioidival/limit/trunk/install.sh | bash
```

This will install Limit to `~/.local/bin/lim`.

### From Source

```bash
git clone https://github.com/marioidival/limit.git
cd limit
cargo build --workspace --release
```

### Requirements

**One-line install:**
- curl or wget
- Unix-like OS (Linux, macOS)

**From source:**
- Rust 1.70+
- git
- Unix-like OS (Linux, macOS)

---

## Configuration

📚 **Quick Setup Guides:**
- [Anthropic Claude](docs/CLAUDE_SETUP.md) - Recommended for code analysis
- [OpenAI GPT](docs/OPENAI_SETUP.md) - Fast and reliable
- [z.ai](docs/ZAI_SETUP.md) - Cost-effective alternative
- [Local LLMs](docs/LOCAL_PROVIDERS.md) - Ollama, LM Studio, vLLM
- [Full Configuration Guide](docs/CONFIGURATION.md) - All options in one place

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
timeout = 60
# base_url = "http://localhost:8080/v1/chat/completions"
# Note: Must include full endpoint path (e.g., /v1/chat/completions)
```

### z.ai (ZAI Provider)

```toml
provider = "zai"

[providers.zai]
# api_key is optional - falls back to ZAI_API_KEY env var
api_key = "..."
model = "glm-4.7"
max_tokens = 4096
timeout = 60
# Optional: Enable thinking mode for complex reasoning tasks
# thinking_enabled = false
# clear_thinking = true  # Set to false for Preserved Thinking in multi-turn
```

### Local LLMs (Ollama, LM Studio, vLLM)

```toml
provider = "local"  # or "ollama", "lmstudio", "vllm"

[providers.local]
model = "llama3.2"
base_url = "http://localhost:11434/v1/chat/completions"
# api_key not required for local servers
max_tokens = 4096
timeout = 120  # Longer timeout for local inference
```

See [Local LLM Providers](docs/LOCAL_PROVIDERS.md) for detailed setup guides.

### Environment Variables

Provider API keys can be set via environment variables as fallback:

| Variable | Provider |
|----------|----------|
| `ANTHROPIC_API_KEY` | Anthropic Claude |
| `OPENAI_API_KEY` | OpenAI |
| `ZAI_API_KEY` | z.ai |

---

## Usage

### TUI Mode (Default)

```bash
lim
```

### REPL Mode (Text-only)

```bash
lim --no-tui
```

> If installed from source without the install script: `cargo run --package limit-cli`
### Example Interaction

```
lim> Read the file src/main.rs and explain what it does

[Reading src/main.rs...]

This file contains the main entry point for the CLI application...
```

### Available Commands

| Command | Description |
|---------|-------------|
| `/exit` | Save session and exit |
| `/clear` | Clear the screen |
| `/help` | Show available commands |
| `/model` | Show current model configuration |
| `/session list` | List all saved sessions |
| `/session new` | Create a new session |
| `/session load <id>` | Load a specific session by ID |

---

## Built-in Tools

### File Operations
| Tool | Description |
|------|-------------|
| `file_read` | Read file contents (max 50MB) |
| `file_write` | Write content to file |
| `file_edit` | Edit file using diff-based replacement |

### Shell
| Tool | Description |
|------|-------------|
| `bash` | Execute shell commands with timeout |

### Git
| Tool | Description |
|------|-------------|
| `git_status` | Show repository status |
| `git_diff` | Show changes |
| `git_log` | Show commit history |
| `git_add` | Stage files |
| `git_commit` | Create commit |
| `git_push` | Push to remote |
| `git_pull` | Pull from remote |
| `git_clone` | Clone repository |

### Code Analysis
| Tool | Description |
|------|-------------|
| `grep` | Search files with regex |
| `ast_grep` | AST-aware code search (Rust, TypeScript, Python) |
| `lsp` | LSP operations (go-to-definition, find-references) |

### Web
| Tool | Description |
|------|-------------|
| `web_search` | Search the web using Exa AI for current information |
| `web_fetch` | Fetch and convert web pages to markdown |

---

## Crates

| Crate | Description |
|-------|-------------|
| [`limit-llm`](limit-llm) | Multi-provider LLM client with streaming, SQLite tracking, binary persistence, model handoff |
| [`limit-agent`](limit-agent) | Agent runtime with tool registry, parallel execution, event system, Docker sandbox |
| [`limit-cli`](limit-cli) | REPL interface with 17 tools, markdown rendering, session management |

---

## Development

See [DEVELOPMENT_GUIDE.md](DEVELOPMENT_GUIDE.md) for:
- Building and testing
- Project structure
- Adding new tools/providers
- Code style guidelines
- Debugging tips

---

## Documentation

### Configuration Guides
- [Configuration Overview](docs/CONFIGURATION.md) - All configuration options in one place
- [Anthropic Claude Setup](docs/CLAUDE_SETUP.md) - Detailed guide for Claude setup
- [OpenAI Setup](docs/OPENAI_SETUP.md) - Detailed guide for GPT-4/GPT-3.5 setup
- [z.ai Setup](docs/ZAI_SETUP.md) - Detailed guide for z.ai (GLM-4) setup
- [Local LLM Providers](docs/LOCAL_PROVIDERS.md) - Ollama, LM Studio, vLLM, and custom servers

---

## Session Persistence

Sessions are automatically saved to `~/.limit/sessions/` and include:
- Conversation history (messages)
- Session metadata (SQLite)
- Agent state
- Token usage statistics

On startup, the last session is automatically restored.

---

## Token Tracking

Usage statistics are tracked in `~/.limit/tracking.db`:
- Requests count
- Input/output tokens
- Cost estimation
- Duration metrics

---

## Docker Sandbox (Optional)

Limit can execute tools in isolated Docker containers:

```bash
# Requires Docker to be installed and running
# Automatically detected and used if available
```

Sandbox features:
- 🔒 Isolated execution environment
- 📁 Read-only project mount
- 🚫 No network access
- 💾 512MB memory limit
- ⏱️ 60s timeout

---

## Contributing

We welcome contributions! 

1. Fork and clone the repository
2. Create a feature branch
3. Run tests: `cargo test --workspace`
4. Run lints: `cargo clippy --all-targets && cargo fmt`
5. Submit a PR with conventional commit message


## Known Limitations

| Limitation | Status |
|------------|--------|
| Unix-only TUI | Windows support planned |
| Max 50MB file reads | By design |
| Max 100 iterations per session | Configurable via max_iterations |

---

## License

MIT
