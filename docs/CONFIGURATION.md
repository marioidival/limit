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

## Verify Configuration

```bash
lim
lim> /model
```

## See Also

- [README.md](../README.md) - Main documentation
- [DEVELOPMENT_GUIDE.md](../DEVELOPMENT_GUIDE.md) - Development setup
