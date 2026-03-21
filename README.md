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
- **18 Built-in Tools** — File I/O, Bash execution, Git operations, code analysis (including tldr_analyze), web search/fetch, browser automation
- **Session Persistence** — Auto-save/restore conversation history
- **Token Tracking** — SQLite-based usage tracking with cost estimation
- **Docker Sandbox** — Optional containerized tool execution for isolation
- **LSP Integration** — Go-to-definition, find-references
- **AST-Aware Search** — Code search that understands syntax (Rust, TypeScript, Python)
- **Markdown Rendering** — Rich formatting with syntax-highlighted code blocks
- **File Autocomplete** — Type `@` in TUI to quickly reference files with fuzzy matching

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
| File autocomplete | ✅ | ❌ | ✅ | ✅ |

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

### Supported Platforms

Prebuilt binaries are available for:

| Platform | Architecture | Binary |
|----------|-------------|--------|
| Linux | x86_64 | `lim-linux-x86_64` |
| Linux | aarch64 (ARM64) | `lim-linux-aarch64` |
| macOS | aarch64 (Apple Silicon) | `lim-macos-aarch64` |

> Note: macOS Intel (x86_64) is not supported due to ONNX Runtime not providing prebuilt binaries for that target. Apple Silicon Macs can still run the x86_64 binary via Rosetta 2.

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
model = "claude-sonnet-4-6-20260217"  # or "claude-opus-4-6-20260205" for most capable
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
model = "gpt-5.4"  # or "gpt-4" for legacy
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
model = "glm-5"  # 744B parameters, 200K context, best open-weights model
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
model = "llama3.3"  # or "qwen2.5", "deepseek-r1", "mistral"
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
| `/share` | Copy session to clipboard (markdown) |
| `/share md` | Export session as markdown file |
| `/share json` | Export session as JSON file |

---

## File Autocomplete (TUI)

When using the TUI mode, you can quickly reference files in your project by typing `@` followed by the filename:

```
Read @Cargo.toml and analyze the dependencies
```

**Features:**
- **Fuzzy Matching** — Powered by Frizbee for fast, typo-tolerant search
- **Smart Filtering** — Automatically respects `.gitignore` and `.ignore` files
- **Keyboard Navigation** — Use ↑/↓ to navigate, Enter/Tab to select, Esc to cancel
- **Visual Highlighting** — Matching characters are highlighted in yellow
- **Cached Results** — File list is cached for 5 seconds for better performance

**Example:**
```
Input: Analyze @src/main

[Popup appears showing:]
┌─────────────────────────────┐
│ ► src/main.rs               │  ← selected
│   src/main_test.rs          │
│   src/bin/main.rs           │
└─────────────────────────────┘

[After Enter:]
Input: Analyze @src/main.rs and explain
```

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
| `tldr_analyze` | Token-efficient code analysis - 95% token savings vs raw code. See [limit-tldr](#limit-tldr) for details |

### Web
| Tool | Description |
|------|-------------|
| `web_search` | Search the web using Exa AI for current information |
| `web_fetch` | Fetch and convert web pages to markdown |

### Browser Automation
| Tool | Description |
|------|-------------|
| `browser` | Full browser automation with 46+ actions: open, click, fill, screenshot, wait, tabs, cookies, and more. See [Browser Tool](docs/BROWSER_TOOL.md) for details. |

---

## limit-tldr

**Code analysis that actually fits in context — 95% token savings vs raw code.**

The `limit-tldr` crate extracts structure, traces dependencies, and delivers exactly what an LLM needs without reading entire files.

### Architecture (5 Layers)

| Layer | Name | Purpose |
|-------|------|---------|
| **1** | AST | Structure — "What functions exist?" |
| **2** | Call Graph | Dependencies — "Who calls what?" |
| **3** | CFG | Control Flow — "How complex is this?" |
| **4** | DFG | Data Flow — "Where does this value come from?" |
| **5** | PDG | Program Dependence — "What affects this line?" |

### Analysis Types

| Type | Description |
|------|-------------|
| `search` | Find functions by name/keyword |
| `context` | Get function dependencies + callers |
| `source` | Extract function implementation code |
| `impact` | Who calls this function? |
| `cfg` | Control flow graph (complexity) |
| `dfg` | Data flow graph (variable origins) |
| `dead_code` | Find unreachable functions |
| `architecture` | Detect module layers |

### Features

- **Semantic Search** — Code search using embeddings (optional)
- **Multi-language** — Rust, Python, JavaScript, TypeScript, Go, Java, C, C++
- **Incremental Cache** — BLAKE3 hashing for fast re-indexing
- **Dead Code Detection** — Find functions never called
- **Program Slicing** — Extract relevant lines for a specific point

### Example

```rust
use limit_tldr::{TLDR, Config};

let mut tldr = TLDR::new("./my-project", Config::default()).await?;
tldr.warm().await?;

// Get context for LLM
let context = tldr.get_context("process_data", 2).await?;

// Impact analysis
let callers = tldr.get_impact("hash_password")?;

// Find dead code
let dead = tldr.find_dead_code(&["main"])?;
```

---

## Crates

| Crate | Description |
|-------|-------------|
| [`limit-llm`](limit-llm) | Multi-provider LLM client with streaming, SQLite tracking, binary persistence, model handoff |
| [`limit-agent`](limit-agent) | Agent runtime with tool registry, parallel execution, event system, Docker sandbox |
| [`limit-tldr`](limit-tldr) | Code analysis library with 95% token savings — AST, call graph, CFG, DFG, PDG, semantic search |
| [`limit-cli`](limit-cli) | REPL interface with 18 tools, markdown rendering, session management |

---

## Optional Dependencies

Limit works out-of-the-box, but some features require external tools:

### Required (tool fails if missing)

| Tool | Feature | Install |
|------|---------|---------|
| **git** | Git operations | `brew install git` |
| **grep** | Regex search in files | Usually pre-installed |
| **sh** | Shell command execution | Usually pre-installed |

### Optional (feature disabled if missing)

| Tool | Feature | Install |
|------|---------|---------|
| **Docker** | Sandbox isolation | [Install Docker](https://docs.docker.com/get-docker/) |
| **ast-grep** | AST-aware code search | `brew install ast-grep` or `cargo install ast-grep` |
| **rust-analyzer** | LSP for Rust | `rustup component add rust-analyzer` |
| **typescript-language-server** | LSP for TypeScript/JavaScript | `npm i -g typescript-language-server` |
| **pylsp** | LSP for Python | `pip install python-lsp-server` |

> **Note:** LSP integration is currently a placeholder. Full implementation requires an LSP client library.

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
- [OpenAI Setup](docs/OPENAI_SETUP.md) - Detailed guide for GPT-5.4 setup
- [z.ai Setup](docs/ZAI_SETUP.md) - Detailed guide for z.ai (GLM-5) setup
- [Local LLM Providers](docs/LOCAL_PROVIDERS.md) - Ollama, LM Studio, vLLM, and custom servers
- [Browser Tool](docs/BROWSER_TOOL.md) - Browser automation for testing and scraping

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
