# Limit Configuration

Configure Limit with your preferred LLM provider.

## Supported Providers

- [Anthropic Claude](CLAUDE_SETUP.md)
- [OpenAI](OPENAI_SETUP.md)
- [z.ai](ZAI_SETUP.md)

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

## Multiple Config Files

Create different configs for different use cases:

```bash
~/.limit/config-dev.toml
~/.limit/config-prod.toml
```

Use with:

```bash
lim --config ~/.limit/config-dev.toml
```

## Verify Configuration

```bash
lim
lim> /model
```

## See Also

- [README.md](../README.md) - Main documentation
- [DEVELOPMENT_GUIDE.md](../DEVELOPMENT_GUIDE.md) - Development setup
