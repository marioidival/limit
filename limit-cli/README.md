# limit-cli

[![Crates.io](https://img.shields.io/crates/v/limit-cli.svg)](https://crates.io/crates/limit-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**AI-powered terminal coding assistant with REPL and TUI.**

An intelligent coding assistant that lives in your terminal. Features multi-provider LLM support, session persistence, and built-in tools for file operations, git, and code analysis.

Part of the [Limit](https://github.com/marioidival/limit) ecosystem.

## Features

- **Multi-provider LLM**: Anthropic Claude, OpenAI GPT, z.ai
- **Two interfaces**: Full TUI (default) or simple REPL mode
- **Built-in tools**: File read/write/edit, bash execution, git operations
- **AST-aware search**: Structural code matching with `ast-grep`
- **LSP integration**: Go-to-definition, find-references
- **Session persistence**: Resume conversations across sessions
- **Web search**: Fetch current information via Exa AI
- **Markdown rendering**: Rich formatting with syntax highlighting

## Installation

### via Cargo

```bash
cargo install limit-cli
```

### via install script

```bash
curl -fsSL https://raw.githubusercontent.com/marioidival/limit/trunk/install.sh | bash
```

## Usage

```bash
# TUI mode (default)
lim

# REPL mode
lim --no-tui
```

## Configuration

Create `~/.limit/config.toml`:

```toml
provider = "anthropic"
```

Set your API key:

```bash
# Anthropic Claude (recommended)
export ANTHROPIC_API_KEY="your-key-here"

# OpenAI
export OPENAI_API_KEY="your-key-here"

# z.ai
export ZAI_API_KEY="your-key-here"
```

## Built-in Tools

| Tool | Description |
|------|-------------|
| `file_read` | Read file contents (max 50MB) |
| `file_write` | Create or overwrite files |
| `file_edit` | Diff-based file editing |
| `bash` | Execute shell commands |
| `git_status` | Show repository status |
| `git_diff` | Show changes |
| `git_log` | Show commit history |
| `git_add` | Stage files |
| `git_commit` | Create commit |
| `git_push` / `git_pull` | Remote operations |
| `grep` | Regex search in files |
| `ast_grep` | AST-aware code search (Rust, TypeScript, Python) |
| `lsp` | Language server operations |
| `web_search` | Search the web via Exa AI |
| `web_fetch` | Fetch URL content |

## Session Management

Sessions are automatically saved to `~/.limit/sessions/`. Conversation history persists across sessions.

Use `/help` in the interface for available commands.

## License

MIT
