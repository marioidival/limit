# Local LLM Providers

Run Limit with local models using Ollama, LM Studio, vLLM, or any OpenAI-compatible server.

## Quick Start

```toml
# ~/.limit/config.toml
provider = "local"

[providers.local]
model = "llama3.2"  # Your model name
base_url = "http://localhost:11434/v1/chat/completions"
```

That's it! The `local` provider requires no API key and uses sensible defaults for local servers.

## Supported Providers

| Provider | Default Port | Status |
|----------|-------------|--------|
| [Ollama](#ollama) | 11434 | Full support |
| [LM Studio](#lm-studio) | 1234 | Full support |
| [vLLM](#vllm) | 8000 | Full support |
| [Other](#custom-servers) | Varies | OpenAI-compatible |

## Provider Aliases

Limit accepts these provider names (all use the same OpenAI-compatible protocol):

- `local` - Generic local provider (recommended)
- `ollama` - Ollama-specific alias
- `lmstudio` - LM Studio-specific alias
- `vllm` - vLLM-specific alias

```toml
# All equivalent:
provider = "local"
provider = "ollama"
provider = "lmstudio"
provider = "vllm"
```

---

## Ollama

[Ollama](https://ollama.ai/) is the most popular way to run LLMs locally.

### Installation

```bash
# macOS/Linux
curl -fsSL https://ollama.ai/install.sh | sh

# Or via Homebrew
brew install ollama
```

### Start Server

```bash
ollama serve
```

### Pull a Model

```bash
ollama pull llama3.2
ollama pull qwen2.5-coder:7b
ollama pull deepseek-coder:6.7b
```

### Configuration

```toml
provider = "ollama"

[providers.ollama]
model = "llama3.2"
base_url = "http://localhost:11434/v1/chat/completions"
# api_key not required for Ollama
```

### List Available Models

```bash
ollama list
```

### Recommended Models for Coding

| Model | Size | Best For |
|-------|------|----------|
| `qwen2.5-coder:7b` | 7B | General coding, fast |
| `deepseek-coder:6.7b` | 6.7B | Code generation |
| `codellama:7b` | 7B | Code completion |
| `llama3.2:3b` | 3B | Lightweight, fast responses |
| `llama3.1:8b` | 8B | General purpose |

---

## LM Studio

[LM Studio](https://lmstudio.ai/) provides a GUI to run local models.

### Setup

1. Download from [lmstudio.ai](https://lmstudio.ai/)
2. Open LM Studio
3. Go to the "Local Server" tab
4. Start the server (default: `http://localhost:1234`)
5. Load a model

### Configuration

```toml
provider = "lmstudio"

[providers.lmstudio]
model = "local-model"  # Model name shown in LM Studio
base_url = "http://localhost:1234/v1/chat/completions"
```

### Notes

- LM Studio must be running with a model loaded
- The model name in config should match what's shown in LM Studio
- Supports GGUF format models from Hugging Face

---

## vLLM

[vLLM](https://github.com/vllm-project/vllm) is a high-performance inference server.

### Installation

```bash
pip install vllm
```

### Start Server

```bash
vllm serve meta-llama/Llama-3.2-3B-Instruct --port 8000
```

### Configuration

```toml
provider = "vllm"

[providers.vllm]
model = "meta-llama/Llama-3.2-3B-Instruct"
base_url = "http://localhost:8000/v1/chat/completions"
```

### With API Token (Hugging Face)

```toml
[providers.vllm]
model = "meta-llama/Llama-3.2-3B-Instruct"
base_url = "http://localhost:8000/v1/chat/completions"
api_key = "hf_xxx"  # If server requires auth
```

---

## Custom Servers

Any OpenAI-compatible API server works with the `local` provider.

### Configuration Template

```toml
provider = "local"

[providers.local]
model = "your-model-name"
base_url = "http://your-server:port/v1/chat/completions"
api_key = ""  # Optional, if server requires auth
max_tokens = 4096
timeout = 120
```

### Common Endpoints

| Server | Typical Endpoint |
|--------|-----------------|
| Ollama | `/v1/chat/completions` |
| LM Studio | `/v1/chat/completions` |
| vLLM | `/v1/chat/completions` |
| text-generation-webui | `/v1/chat/completions` |
| LocalAI | `/v1/chat/completions` |

> **Important**: Always include the full endpoint path in `base_url`. Limit does not auto-append paths.

---

## Advanced Configuration

### All Options

```toml
[providers.local]
model = "llama3.2"           # Required: model identifier
base_url = "http://..."       # Required: full API endpoint
api_key = ""                  # Optional: auth key if needed
max_tokens = 4096             # Optional: max output tokens (default: 4096)
timeout = 120                 # Optional: request timeout in seconds (default: 60)
max_iterations = 100          # Optional: agent loop limit (default: 100)
```

### Environment Variable

You can also set the base URL via environment variable:

```bash
export LOCAL_API_KEY=""  # Not needed for most local servers
lim
```

---

## Troubleshooting

### "Connection refused"

- Ensure your local server is running
- Check the port matches your server
- Verify `base_url` includes the full path

### "HTTP 404 Not Found"

- Verify `base_url` path is correct
- Check server logs for the correct endpoint
- Some servers use `/v1/api/completions` instead of `/v1/chat/completions`

### Slow Responses

- Try a smaller model (e.g., `llama3.2:3b` instead of `llama3.1:8b`)
- Increase `timeout` if model is slow to generate
- Check GPU/CPU utilization

### Out of Memory

- Use a quantized model (GGUF Q4_K_M or similar)
- Reduce model size (fewer parameters)
- Close other applications

---

## Testing Your Setup

```bash
# Start Limit
lim

# Check current model
lim> /model

# Simple test
lim> hello, can you help me with code?
```

---

## See Also

- [Configuration Guide](CONFIGURATION.md) - Full configuration reference
- [OpenAI Setup](OPENAI_SETUP.md) - Using OpenAI or compatible APIs
- [Development Guide](../DEVELOPMENT_GUIDE.md) - Contributing to Limit
