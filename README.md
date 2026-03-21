<div align="center">
  <img src="assets/logo.png" alt="Limit Logo" width="200"/>
</div>

# Limit

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/limit-cli.svg)](https://crates.io/crates/limit-cli)

**AI pair programmer that lives in the terminal.**

Edit files, run commands, analyze code, manage git — all through natural language. Supports Claude, OpenAI, z.ai, and local LLMs.

## Install

```bash
# One-line
curl -fsSL https://raw.githubusercontent.com/marioidival/limit/trunk/install.sh | bash

# Or via Cargo
cargo install limit-cli
```

## Requirements

| Platform | Architecture | Binary |
|----------|-------------|--------|
| Linux | x86_64 | `lim-linux-x86_64` |
| Linux | aarch64 (ARM64) | `lim-linux-aarch64` |
| macOS | aarch64 (Apple Silicon) | `lim-macos-aarch64` |

**From source:** Rust 1.70+, git, Unix-like OS (Linux, macOS)

> Note: macOS Intel (x86_64) not supported due to ONNX Runtime limitations.

## Quick Start

```bash
# 1. Configure
echo 'provider = "anthropic"' > ~/.limit/config.toml
export ANTHROPIC_API_KEY="your-key"

# 2. Run
lim
```

## Features

| Feature | Description |
|---------|-------------|
| **Multi-Provider** | Anthropic Claude, OpenAI, z.ai, Ollama, LM Studio, vLLM |
| **18 Tools** | File I/O, Bash, Git, code analysis, web search, browser automation |
| **Code Analysis** | 95% token savings with limit-tldr (AST, call graph, CFG, DFG, PDG) |
| **Session Persistence** | Auto-save/restore conversations |
| **Docker Sandbox** | Optional isolated execution |
| **TUI + REPL** | Beautiful terminal UI or simple text mode |

## Configuration

Create `~/.limit/config.toml`:

```toml
# Anthropic (recommended)
provider = "anthropic"
[providers.anthropic]
model = "claude-sonnet-4-6-20260217"

# OpenAI
# provider = "openai"
# [providers.openai]
# model = "gpt-5.4"

# Local (Ollama, LM Studio, vLLM)
# provider = "local"
# [providers.local]
# model = "llama3.3"
# base_url = "http://localhost:11434/v1/chat/completions"
```

**Environment variables:** `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `ZAI_API_KEY`

## Tools

### Core
| Tool | Description |
|------|-------------|
| `file_read` | Read files (max 50MB) |
| `file_write` | Write files |
| `file_edit` | Diff-based edits |
| `bash` | Execute shell commands |
| `grep` | Regex search |

### Git
| Tool | Description |
|------|-------------|
| `git_status` | Repository status |
| `git_diff` | Show changes |
| `git_log` | Commit history |
| `git_add` | Stage files |
| `git_commit` | Create commit |
| `git_push` / `git_pull` | Sync with remote |
| `git_clone` | Clone repository |

### Code Analysis
| Tool | Description |
|------|-------------|
| `tldr_analyze` | Token-efficient analysis (95% savings) — search, context, source, impact, cfg, dfg, dead_code, architecture. **Requires `/tldr` to enable per project.** See [limit-tldr/README.md](limit-tldr/README.md) |
| `ast_grep` | AST-aware search (Rust, TS, Python) |
| `lsp` | Go-to-definition, find-references |

### Web & Browser
| Tool | Description |
|------|-------------|
| `web_search` | Search the web (Exa AI) |
| `web_fetch` | Fetch pages as markdown |
| `browser` | 46+ automation actions |

## Commands

| Command | Description |
|---------|-------------|
| `/exit` | Save and exit |
| `/clear` | Clear screen |
| `/help` | Show commands |
| `/model` | Show config |
| `/tldr` | Enable code analysis for this project |
| `/warm` | Alias for `/tldr` |
| `/session list` | List sessions |
| `/session new` | New session |
| `/share` | Export session (clipboard/md/json) |

## Optional Dependencies

| Tool | Feature | Install |
|------|---------|---------|
| Docker | Sandbox isolation | [docker.com](https://docs.docker.com/get-docker/) |
| ast-grep | AST search | `brew install ast-grep` |
| rust-analyzer | LSP for Rust | `rustup component add rust-analyzer` |
| typescript-language-server | LSP for TS | `npm i -g typescript-language-server` |
| pylsp | LSP for Python | `pip install python-lsp-server` |

## Crates

| Crate | Description |
|-------|-------------|
| [`limit-llm`](limit-llm) | Multi-provider LLM client |
| [`limit-agent`](limit-agent) | Agent runtime with tool registry |
| [`limit-tldr`](limit-tldr) | Code analysis (95% token savings) |
| [`limit-cli`](limit-cli) | TUI/REPL interface |

## Documentation

- [Configuration Guide](docs/CONFIGURATION.md)
- [Anthropic Setup](docs/CLAUDE_SETUP.md)
- [OpenAI Setup](docs/OPENAI_SETUP.md)
- [z.ai Setup](docs/ZAI_SETUP.md)
- [Local LLMs](docs/LOCAL_PROVIDERS.md)
- [Browser Tool](docs/BROWSER_TOOL.md)
- [Development Guide](DEVELOPMENT_GUIDE.md)

## License

MIT
