# OpenAI Setup

Configure Limit to use OpenAI's GPT models.

## Get API Key

1. Visit [https://platform.openai.com/api-keys](https://platform.openai.com/api-keys)
2. Log in to your OpenAI account
3. Click "Create new secret key"
4. Copy the key (starts with `sk-...`)

## Configuration

Create `~/.limit/config.toml`:

```toml
provider = "openai"

[providers.openai]
api_key = "sk-..."
model = "gpt-4"
max_tokens = 4096
timeout = 300000
```

Or use environment variable:

```bash
export OPENAI_API_KEY="sk-..."
```

## Test

```bash
lim
lim> /model
```

You should see OpenAI provider settings.

## Troubleshooting

**Invalid API key**: Check your key in config or environment variable.

**Rate limit exceeded**: Wait a few minutes or check your usage at [https://platform.openai.com/usage](https://platform.openai.com/usage).

**Timeout**: Increase `timeout` value in config.toml.
