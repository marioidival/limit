# Configurando OpenAI com Limit

Este guia mostra como configurar o Limit para usar a API do OpenAI (GPT-4, GPT-3.5, etc.).

## Pré-requisitos

- [x] Limit instalado (veja [README.md](../README.md#quick-start))
- [x] Uma conta no [OpenAI](https://platform.openai.com/)
- [x] Uma API Key do OpenAI

## Passo 1: Obter uma API Key

1. Acesse [https://platform.openai.com/api-keys](https://platform.openai.com/api-keys)
2. Faça login na sua conta OpenAI
3. Clique em **"Create new secret key"**
4. Dê um nome descritivo (ex: "Limit CLI")
5. Copie a key (começa com `sk-...`)
   - ⚠️ **Importante:** Copie agora, você não poderá ver a key novamente!

## Passo 2: Criar arquivo de configuração

Crie o diretório e arquivo de configuração:

```bash
mkdir -p ~/.limit
nano ~/.limit/config.toml
```

Cole o seguinte conteúdo:

```toml
provider = "openai"

[providers.openai]
# api_key é opcional - fallback para OPENAI_API_KEY ou ZAI_API_KEY env var
api_key = "sk-..."
model = "gpt-4"
max_tokens = 4096
timeout = 300000
```

### Configuração alternativa (via variável de ambiente)

Se você preferir não expor a API key no arquivo de configuração:

```bash
export OPENAI_API_KEY="sk-..."
```

Ou para persistir no seu shell (adicione ao `~/.bashrc`, `~/.zshrc`, etc.):

```bash
echo 'export OPENAI_API_KEY="sk-..."' >> ~/.bashrc
source ~/.bashrc
```

### Variáveis de ambiente suportadas

O Limit suporta duas variáveis de ambiente para OpenAI (ordem de prioridade):

1. `OPENAI_API_KEY` (mais alta prioridade)
2. `ZAI_API_KEY`

## Passo 3: Modelos disponíveis

### Recomendados

| Modelo | Descrição | Use para |
|--------|-----------|----------|
| `gpt-4` | Modelo mais capaz | Tarefas complexas, análise de código, refatoração |
| `gpt-4-turbo` | Versão mais rápida e barata do GPT-4 | Balance entre performance e custo |
| `gpt-3.5-turbo` | Rápido e econômico | Tarefas simples, debugging rápido |

### Exemplo de configuração para GPT-4 Turbo

```toml
provider = "openai"

[providers.openai]
api_key = "sk-..."
model = "gpt-4-turbo"
max_tokens = 4096
timeout = 300000
```

### Exemplo de configuração para GPT-3.5

```toml
provider = "openai"

[providers.openai]
api_key = "sk-..."
model = "gpt-3.5-turbo"
max_tokens = 4096
timeout = 300000
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
Provider: openai
Model: gpt-4
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
2. Ou verifique se `OPENAI_API_KEY` está definida:
   ```bash
   echo $OPENAI_API_KEY
   ```

### Erro: "Rate limit exceeded"

**Causa:** Você excedeu o limite de requisições da sua conta.

**Solução:**
1. Verifique seus limites em [https://platform.openai.com/usage](https://platform.openai.com/usage)
2. Aguarde alguns minutos antes de tentar novamente
3. Considere fazer upgrade da sua conta

### Erro: "Timeout"

**Causa:** O modelo demorou muito para responder.

**Solução:** Aumente o timeout:

```toml
[providers.openai]
timeout = 600000  # 10 minutos em milissegundos
```

### Erro: "Model not found"

**Causa:** Nome do modelo incorreto ou não disponível na sua conta.

**Solução:**
1. Verifique os modelos disponíveis em [https://platform.openai.com/docs/models](https://platform.openai.com/docs/models)
2. Alguns modelos podem requerer acesso especial (ex: GPT-4)

## Dicas avançadas

### Usando endpoint customizado

Se você usa um provedor compatível com OpenAI:

```toml
[providers.openai]
api_key = "sk-..."
model = "gpt-4"
base_url = "https://api.custom-provider.com/v1/chat/completions"
```

### Ajustando max_tokens

- **Para respostas curtas:** `max_tokens = 512`
- **Para código:** `max_tokens = 2048`
- **Para documentação:** `max_tokens = 4096`

### Múltiplos modelos para diferentes usos

Você pode criar arquivos de configuração diferentes:

```bash
# Config para desenvolvimento rápido
~/.limit/config-dev.toml  # com gpt-3.5-turbo

# Config para tarefas complexas
~/.limit/config-prod.toml  # com gpt-4-turbo
```

E alternar entre eles:

```bash
lim --config ~/.limit/config-dev.toml
```

## Custos estimados

| Modelo | Input (por 1M tokens) | Output (por 1M tokens) |
|--------|----------------------|------------------------|
| GPT-4 | $30.00 | $60.00 |
| GPT-4 Turbo | $10.00 | $30.00 |
| GPT-3.5 Turbo | $0.50 | $1.50 |

> O Limit rastreia automaticamente o uso em `~/.limit/tracking.db`

## Links úteis

- [Documentação OpenAI](https://platform.openai.com/docs)
- [Preços](https://platform.openai.com/pricing)
- [Limites de taxa](https://platform.openai.com/docs/guides/rate-limits)
- [README do Limit](../README.md)

## Próximos passos

- 📖 Aprenda sobre [Claude setup](CLAUDE_SETUP.md)
- 📖 Aprenda sobre [z.ai setup](ZAI_SETUP.md)
- 📖 Veja os [comandos disponíveis](../README.md#available-commands)
