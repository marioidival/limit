# Anthropic Claude Setup

Configure Limit to use Anthropic's Claude models.

## Get API Key

1. Visit [https://console.anthropic.com/settings/keys](https://console.anthropic.com/settings/keys)
2. Log in to your Anthropic account
3. Click "Create Key"
4. Copy the key (starts with `sk-ant-api03-...`)

## Configuration

Create `~/.limit/config.toml`:

```toml
provider = "anthropic"

[providers.anthropic]
api_key = "sk-ant-api03-..."
model = "claude-3-5-sonnet-20241022"
max_tokens = 4096
timeout = 60
```

Or use environment variable:

```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
```

## Test

```bash
lim
lim> /model
```

You should see Anthropic provider settings.

## Troubleshooting

**Invalid API key**: Check your key in config or environment variable.

**Rate limit exceeded**: Wait a few minutes or check your usage at [https://console.anthropic.com/settings/usage](https://console.anthropic.com/settings/usage).

**Timeout**: Increase `timeout` value in config.toml.
