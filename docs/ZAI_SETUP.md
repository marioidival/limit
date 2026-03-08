# z.ai Setup

Configure Limit to use z.ai's GLM models.

## Get API Key

1. Visit [https://z.ai/](https://z.ai/)
2. Log in to your z.ai account
3. Go to Settings → API Keys
4. Click "Generate New Key"
5. Copy the key

## Configuration

Create `~/.limit/config.toml`:

```toml
provider = "zai"

[providers.zai]
api_key = "..."
model = "glm-4.7"
max_tokens = 4096
timeout = 300000
```

Optional: Enable thinking mode

```toml
[providers.zai]
api_key = "..."
model = "glm-4.7"
thinking_enabled = true
```

Or use environment variable:

```bash
export ZAI_API_KEY="..."
```

## Test

```bash
lim
lim> /model
```

You should see z.ai provider settings.

## Troubleshooting

**Invalid API key**: Check your key in config or environment variable.

**Rate limit exceeded**: Wait a few minutes or check your z.ai dashboard.

**Timeout**: Increase `timeout` value in config.toml.
