# limit-cli

AI-powered terminal coding assistant with REPL and TUI.

Part of the [Limit](https://github.com/marioidival/limit) ecosystem.

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
export ANTHROPIC_API_KEY="your-key-here"
```

## License

MIT
