# limit-cli

[![Crates.io](https://img.shields.io/crates/v/limit-cli.svg)](https://crates.io/crates/limit-cli)
[![Docs.rs](https://docs.rs/limit-cli/badge.svg)](https://docs.rs/limit-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**AI-powered terminal coding assistant with REPL and TUI.**

An intelligent coding assistant that lives in your terminal. Features multi-provider LLM support, session persistence, and 18 built-in tools for file operations, git, and code analysis.

Part of the [Limit](https://github.com/marioidival/limit) ecosystem.

## Why This Exists

Developers shouldn't have to leave their terminal to get AI assistance. `limit-cli` brings the power of LLMs directly to your command line with a beautiful TUI, full tool integration, and seamless session management.

## Features

- **Multi-provider LLM**: Anthropic Claude, OpenAI GPT, z.ai GLM, and local models
- **Two interfaces**: Full TUI (default) or simple REPL mode with `--no-tui`
- **18 built-in tools**: File I/O, bash execution, git operations, code analysis
- **AST-aware search**: Structural code matching with `ast-grep` (Rust, TypeScript, Python)
- **LSP integration**: Go-to-definition, find-references
- **Session persistence**: Auto-save and restore conversations
- **Token tracking**: SQLite-based usage tracking with cost estimation
- **Web search**: Fetch current information via Exa AI
- **Markdown rendering**: Rich formatting with syntax highlighting
- **File autocomplete**: Type `@` to quickly reference files with fuzzy matching

## Installation

### One-line Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/marioidival/limit/trunk/install.sh | bash
```

### via Cargo

```bash
cargo install limit-cli
```

**Requirements**: Unix-like OS (Linux, macOS), Rust 1.70+

## Quick Start

```bash
# Set your API key
export ANTHROPIC_API_KEY="your-key-here"

# Start the TUI
lim
```

That's it! Start chatting with your AI coding assistant.

### REPL Mode

```bash
lim --no-tui
```

## Configuration

Create `~/.limit/config.toml`:

```toml
provider = "anthropic"

[providers.anthropic]
model = "claude-sonnet-4-6-20260217"
max_tokens = 4096
```

### Environment Variables

| Variable | Provider |
|----------|----------|
| `ANTHROPIC_API_KEY` | Anthropic Claude |
| `OPENAI_API_KEY` | OpenAI |
| `ZAI_API_KEY` | z.ai |

## Built-in Tools

### File Operations
| Tool | Description |
|------|-------------|
| `file_read` | Read file contents (max 50MB) |
| `file_write` | Create or overwrite files |
| `file_edit` | Diff-based file editing |

### Shell & Git
| Tool | Description |
|------|-------------|
| `bash` | Execute shell commands |
| `git_status` | Show repository status |
| `git_diff` | Show changes |
| `git_log` | Show commit history |
| `git_add` | Stage files |
| `git_commit` | Create commit |
| `git_push` / `git_pull` | Remote operations |
| `git_clone` | Clone repository |

### Code Analysis
| Tool | Description |
|------|-------------|
| `grep` | Regex search in files |
| `ast_grep` | AST-aware code search (Rust, TypeScript, Python) |
| `lsp` | Language server operations |

### Web
| Tool | Description |
|------|-------------|
| `web_search` | Search the web via Exa AI |
| `web_fetch` | Fetch URL content as markdown |
| `browser` | Browser automation for testing and scraping |

## Session Management

Sessions are automatically saved to `~/.limit/sessions/`:

```bash
# In the TUI/REPL
/session list      # List all saved sessions
/session new       # Create a new session
/session load <id> # Load a specific session
/share             # Copy session to clipboard
/share md          # Export as markdown file
/share json        # Export as JSON file
```

## File Autocomplete

Type `@` in the TUI to quickly reference files:

```
Read @Cargo.toml and analyze the dependencies
```

Features:
- Fuzzy matching powered by Frizbee
- Respects `.gitignore` and `.ignore`
- Keyboard navigation (↑/↓/Enter/Tab/Esc)

## Commands

| Command | Description |
|---------|-------------|
| `/exit` | Save session and exit |
| `/clear` | Clear the screen |
| `/help` | Show available commands |
| `/model` | Show current model configuration |
| `/session list` | List all saved sessions |
| `/session new` | Create a new session |
| `/session load <id>` | Load a specific session |
| `/share` | Copy session to clipboard |
| `/share md` | Export session as markdown |
| `/share json` | Export session as JSON |

## Token Tracking

Usage statistics are tracked in `~/.limit/tracking.db`:

- Request count
- Input/output tokens
- Cost estimation
- Duration metrics

## License

MIT © [Mário Idival](https://github.com/marioidival)
