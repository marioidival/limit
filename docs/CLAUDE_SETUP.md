# Configurando Anthropic Claude com Limit

Este guia mostra como configurar o Limit para usar a API do Anthropic Claude (Claude 3.5 Sonnet, Claude 3 Opus, etc.).

## Pré-requisitos

- [x] Limit instalado (veja [README.md](../README.md#quick-start))
- [x] Uma conta no [Anthropic](https://console.anthropic.com/)
- [x] Uma API Key do Anthropic

## Passo 1: Obter uma API Key

1. Acesse [https://console.anthropic.com/settings/keys](https://console.anthropic.com/settings/keys)
2. Faça login na sua conta Anthropic
3. Clique em **"Create Key"**
4. Dê um nome descritivo (ex: "Limit CLI")
5. Copie a key (começa com `sk-ant-api03-...`)
   - ⚠️ **Importante:** Copie agora, você não poderá ver a key novamente!

## Passo 2: Criar arquivo de configuração

Crie o diretório e arquivo de configuração:

```bash
mkdir -p ~/.limit
nano ~/.limit/config.toml
```

Cole o seguinte conteúdo:

```toml
provider = "anthropic"

[providers.anthropic]
# api_key é opcional - fallback para ANTHROPIC_API_KEY env var
api_key = "sk-ant-api03-..."
model = "claude-3-5-sonnet-20241022"
max_tokens = 4096
timeout = 60
# Optional: Custom API endpoint
# base_url = "https://api.custom-provider.com/v1/messages"
```

### Configuração alternativa (via variável de ambiente)

Se você preferir não expor a API key no arquivo de configuração:

```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
```

Ou para persistir no seu shell (adicione ao `~/.bashrc`, `~/.zshrc`, etc.):

```bash
echo 'export ANTHROPIC_API_KEY="sk-ant-api03-..."' >> ~/.bashrc
source ~/.bashrc
```

## Passo 3: Modelos disponíveis

### Recomendados

| Modelo | Descrição | Use para |
|--------|-----------|----------|
| `claude-3-5-sonnet-20241022` | Mais rápido e econômico, excelente para código | Tarefas diárias, refatoração, debugging |
| `claude-3-5-sonnet-20240620` | Versão anterior do Sonnet | Compatibilidade com workflows existentes |
| `claude-3-opus-20240229` | Mais inteligente, mas mais lento | Tarefas complexas, análise profunda |

### Exemplo de configuração para Claude 3.5 Sonnet (recomendado)

```toml
provider = "anthropic"

[providers.anthropic]
api_key = "sk-ant-api03-..."
model = "claude-3-5-sonnet-20241022"
max_tokens = 4096
timeout = 60
```

### Exemplo de configuração para Claude 3 Opus

```toml
provider = "anthropic"

[providers.anthropic]
api_key = "sk-ant-api03-..."
model = "claude-3-opus-20240229"
max_tokens = 4096
timeout = 120
```

## Passo 4: Testar a configuração

Execute o Limit:

```bash
lim
```

Ou se instalou do source:

```bash
cargo run --package limit-cli
```

Dentro do Limit, verifique a configuração atual:

```
lim> /model
```

Você deve ver algo como:

```
Provider: anthropic
Model: claude-3-5-sonnet-20241022
Max Tokens: 4096
```

## Passo 5: Testar uma conversa

Faça uma pergunta simples para testar:

```
lim> Olá! Você está funcionando corretamente?
```

## Troubleshooting

### Erro: "Invalid API key"

**Causa:** API key incorreta ou não configurada.

**Solução:**
1. Verifique se a key está correta em `~/.limit/config.toml`
2. Ou verifique se `ANTHROPIC_API_KEY` está definida:
   ```bash
   echo $ANTHROPIC_API_KEY
   ```

### Erro: "Rate limit exceeded"

**Causa:** Você excedeu o limite de requisições da sua conta.

**Solução:**
1. Verifique seus limites em [https://console.anthropic.com/settings/usage](https://console.anthropic.com/settings/usage)
2. Aguarde alguns minutos antes de tentar novamente
3. Considere fazer upgrade da sua conta

### Erro: "Timeout"

**Causa:** O modelo demorou muito para responder.

**Solução:** Aumente o timeout:

```toml
[providers.anthropic]
timeout = 300  # 5 minutos em segundos
```

### Erro: "Model not found"

**Causa:** Nome do modelo incorreto ou não disponível na sua conta.

**Solução:**
1. Verifique os modelos disponíveis em [https://docs.anthropic.com/claude/docs/models-overview](https://docs.anthropic.com/claude/docs/models-overview)
2. Alguns modelos podem requerer acesso especial

## Dicas avançadas

### Usando endpoint customizado

Se você usa um provedor compatível com Anthropic:

```toml
[providers.anthropic]
api_key = "sk-ant-api03-..."
model = "claude-3-5-sonnet-20241022"
base_url = "https://api.custom-provider.com/v1/messages"
```

### Ajustando max_tokens

- **Para respostas curtas:** `max_tokens = 512`
- **Para código:** `max_tokens = 2048`
- **Para documentação:** `max_tokens = 4096`
- **Para outputs longos:** `max_tokens = 8192`

### Múltiplos modelos para diferentes usos

Você pode criar arquivos de configuração diferentes:

```bash
# Config para desenvolvimento rápido
~/.limit/config-dev.toml  # com claude-3-5-sonnet

# Config para tarefas complexas
~/.limit/config-prod.toml  # com claude-3-opus
```

E alternar entre eles:

```bash
lim --config ~/.limit/config-dev.toml
```

## Custos estimados

| Modelo | Input (por 1M tokens) | Output (por 1M tokens) |
|--------|----------------------|------------------------|
| Claude 3.5 Sonnet | $3.00 | $15.00 |
| Claude 3 Opus | $15.00 | $75.00 |
| Claude 3 Haiku | $0.25 | $1.25 |

> O Limit rastreia automaticamente o uso em `~/.limit/tracking.db`

## Claude vs Outros Modelos

### Quando usar Claude:

✅ **Claude é melhor para:**
- Análise de código complexo
- Refatoração extensiva
- Debugging detalhado
- Geração de documentação técnica
- Tarefas que requerem compreensão profunda

⚠️ **Considere outros modelos para:**
- Respostas muito rápidas (GPT-3.5 é mais rápido)
- Tarefas simples de codificação
- Quando custo é prioridade absoluta

## Links úteis

- [Documentação Anthropic](https://docs.anthropic.com/)
- [Preços](https://www.anthropic.com/pricing)
- [Modelos disponíveis](https://docs.anthropic.com/claude/docs/models-overview)
- [Limites de uso](https://console.anthropic.com/settings/limits)
- [README do Limit](../README.md)

## Próximos passos

- 📖 Aprenda sobre [OpenAI setup](OPENAI_SETUP.md)
- 📖 Aprenda sobre [z.ai setup](ZAI_SETUP.md)
- 📖 Veja os [comandos disponíveis](../README.md#available-commands)
- 📖 Aprenda sobre [sessões](../README.md#session-persistence)
