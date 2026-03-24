# Limit Configuration

Configure Limit with your preferred LLM provider.

## Supported Providers

- [Anthropic Claude](CLAUDE_SETUP.md)
- [OpenAI](OPENAI_SETUP.md)
- [z.ai](ZAI_SETUP.md)
- [Local LLMs](LOCAL_PROVIDERS.md) - Ollama, LM Studio, vLLM

## Quick Setup

### 1. Create config file

```bash
mkdir -p ~/.limit
nano ~/.limit/config.toml
```

### 2. Add provider configuration

```toml
provider = "<anthropic|openai|zai>"

[providers.anthropic]
api_key = "sk-ant-api03-..."

[providers.openai]
api_key = "sk-..."
# Optional: Custom API endpoint (for OpenAI-compatible servers)
# base_url = "http://localhost:8080/v1/chat/completions"
# Note: Some servers require the full path (e.g., /v1/api/completions)

[providers.zai]
api_key = "..."
```

### 3. Or use environment variables

| Provider | Environment Variable |
|----------|---------------------|
| Anthropic | `ANTHROPIC_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| z.ai | `ZAI_API_KEY` |

```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
lim
```

## Configuration Options

Each provider supports the following options:

| `base_url` | string | - | Custom API endpoint (optional) |", `thinking_enabled` | boolean | Enable thinking/reasoning mode (Z.AI only, default: false) | `clear_thinking` | boolean | Preserve thinking between turns (Z.AI only) | Default: true |"]
### max_iterations

Controls how many tool call iterations the agent can perform per session:

```toml
[providers.anthropic]
model = "claude-3-5-sonnet-20241022"
max_iterations = 100  # Default
# max_iterations = 0  # Unlimited (not recommended)
# max_iterations = 50 # More conservative
```

**Recommendation**: Keep the default (100) for most use cases. Lower values for cost-sensitive scenarios.

---

## Context Compaction

Limit automatically manages context window usage by compacting messages when approaching token limits.

### How It Works

When the conversation context exceeds the threshold (context window - reserve tokens), Limit:
1. Keeps the system message
2. Keeps recent messages up to the token budget
3. Discards older messages to fit within the limit

This prevents context overflow errors and reduces token costs.

### Configuration

```toml
[compaction]
enabled = true           # Enable/disable compaction (default: true)
reserve_tokens = 16384   # Tokens reserved for LLM response (default: 16384)
keep_recent_tokens = 20000  # Not used in current implementation (future: summary target)
```

### Defaults

| Setting | Default | Description |
|---------|---------|-------------|
| `enabled` | `true` | Enable token-aware compaction |
| `reserve_tokens` | `16384` | Tokens reserved for response |
| `keep_recent_tokens` | `20000` | Target for future summarization |

### Example

```toml
# ~/.limit/config.toml

provider = "anthropic"

[providers.anthropic]
model = "claude-3-5-sonnet-20241022"

[compaction]
enabled = true
reserve_tokens = 8192   # More aggressive compaction
```

### Token Estimation

Limit uses `tiktoken` with the `cl100k_base` tokenizer (same as GPT-4/Claude) for accurate token counting.

---

## Verify Configuration

```bash
lim
lim> /model
```

## See Also

- [README.md](../README.md) - Main documentation
- [DEVELOPMENT_GUIDE.md](../DEVELOPMENT_GUIDE.md) - Development setup
