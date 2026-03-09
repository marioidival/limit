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

### Using Custom OpenAI-Compatible Servers

Limit supports custom OpenAI-compatible API servers (local LLMs, proxy services, etc.). Add the `base_url` parameter:

```toml
[providers.openai]
api_key = ""  # Leave empty if server doesn't require auth
model = "glm-4.7"
base_url = "http://localhost:8080/v1/chat/completions"
timeout = 3000000
```

**Important**: Some OpenAI-compatible servers require the **full API path** in `base_url`. Check your server's documentation for the correct endpoint:

- Standard OpenAI: `https://api.openai.com/v1` (Limit appends `/chat/completions`)
- Ollama: `http://localhost:11434/v1`
- LM Studio: `http://localhost:1234/v1`
- Custom servers (e.g., vLLM): May require full path like `http://localhost:8082/v1/api/completions`

If you get HTTP 404 errors, try adding the full endpoint path to `base_url`.

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
