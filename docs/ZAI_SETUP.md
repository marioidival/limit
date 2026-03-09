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

### Using Self-Hosted GLM Models

If you're running a self-hosted GLM-4 compatible server, use the zai provider with a custom `base_url`:

```toml
provider = "zai"

[providers.zai]
api_key = ""  # Leave empty if no auth required
model = "glm-4.7"
base_url = "http://localhost:8082/v1/api/completions"
timeout = 3000000
```

**Important**: The `base_url` should always include the **full API endpoint path**. For example, the official z.ai API uses:
```
https://api.z.ai/api/coding/paas/v4/chat/completions
```

Check your self-hosted server's documentation for its exact endpoint path.

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
