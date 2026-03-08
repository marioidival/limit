# Guia de Configuração do Limit

Este guia mostra como configurar o Limit com diferentes provedores de LLM (OpenAI, Anthropic Claude, z.ai).

## Visão geral

O Limit su múltiplos provedores de LLM. Você pode configurar:

| Provedor | Documentação | Modelos |
|----------|--------------|---------|
| **Anthropic Claude** | [Ver guia](CLAUDE_SETUP.md) | Claude 3.5 Sonnet, Claude 3 Opus, Haiku |
| **OpenAI** | [Ver guia](OPENAI_SETUP.md) | GPT-4, GPT-4 Turbo, GPT-3.5 Turbo |
| **z.ai** | [Ver guia](ZAI_SETUP.md) | GLM-4.7, GLM-4, GLM-3 Turbo |

## Configuração básica

### 1. Criar arquivo de configuração

Crie o diretório e arquivo de configuração:

```bash
mkdir -p ~/.limit
nano ~/.limit/config.toml
```

### 2. Estrutura básica do config.toml

```toml
provider = "<anthropic|openai|zai>"

[providers.anthropic]
api_key = "sk-ant-api03-..."
model = "claude-3-5-sonnet-20241022"
max_tokens = 4096
timeout = 60

[providers.openai]
api_key = "sk-..."
model = "gpt-4"
max_tokens = 4096
timeout = 300000

[providers.zai]
api_key = "..."
model = "glm-4.7"
max_tokens = 4096
timeout = 300000
thinking_enabled = true  # opcional
```

## Variáveis de ambiente

Você pode usar variáveis de ambiente como fallback para API keys:

| Variável | Provedor |
|----------|----------|
| `ANTHROPIC_API_KEY` | Anthropic Claude |
| `OPENAI_API_KEY` | OpenAI |
| `ZAI_API_KEY` | z.ai |

Exemplo:

```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
lim
```

Ou adicione ao seu `~/.bashrc` ou `~/.zshrc`:

```bash
echo 'export ANTHROPIC_API_KEY="sk-ant-api03-..."' >> ~/.bashrc
source ~/.bashrc
```

## Múltiplos arquivos de configuração

Você pode ter múltiplos arquivos de configuração para diferentes cenários:

```bash
# Config para desenvolvimento rápido
~/.limit/config-dev.toml

# Config para produção
~/.limit/config-prod.toml

# Config para testes
~/.limit/config-test.toml
```

E alternar entre eles:

```bash
lim --config ~/.limit/config-dev.toml
```

## Escolhendo o provedor certo

### Use Anthropic Claude se:
- ✅ Você precisa de excelente compreensão de código
- ✅ Trabalha com tarefas complexas de refatoração
- ✅ Precisa de respostas mais detalhadas
- ✅ Valoriza qualidade sobre velocidade

### Use OpenAI se:
- ✅ Você precisa de respostas muito rápidas
- ✅ Integra com ferramentas existentes OpenAI
- ✅ Precisa de GPT-4 para tarefas específicas
- ✅ Já tem experiência com GPT models

### Use z.ai se:
- ✅ Quer custo-benefício competitivo
- ✅ Precisa de suporte nativo em português
- ✅ Quer experimentar o modo "thinking"
- ✅ Procura uma alternativa econômica

## Rastreamento de uso

O Limit rastreia automaticamente seu uso de LLM em:

```bash
~/.limit/tracking.db
```

Você pode ver estatísticas de:
- Número de requisições
- Tokens de input/output
- Custo estimado
- Duração das requisições

## Sessões

As sessões são salvas automaticamente em:

```bash
~/.limit/sessions/
```

Gerencie sessões com os comandos do Limit:

```
lim> /session list      # Listar todas as sessões
lim> /session new       # Criar nova sessão
lim> /session load <id> # Carregar sessão específica
```

## Comandos úteis

### Ver configuração atual

```
lim> /model
```

### Listar comandos disponíveis

```
lim> /help
```

### Sair salvando

```
lim> /exit
```

## Troubleshooting comum

### Erro: "Invalid API key"
- Verifique se a API key está correta
- Verifique se a variável de ambiente está definida

### Erro: "Rate limit exceeded"
- Aguarde alguns minutos
- Verifique seus limites no painel do provedor
- Considere fazer upgrade da sua conta

### Erro: "Timeout"
- Aumente o timeout no config.toml
- Use um modelo mais rápido (ex: GPT-3.5 ao invés de GPT-4)

### Erro: "Model not found"
- Verifique o nome do modelo
- Alguns modelos podem requerer acesso especial

## Suporte

Se você tiver problemas:

1. 📖 Consulte os guias específicos do provedor acima
2. 📖 Veja o [README principal](../README.md)
3. 📖 Consulte o [Guia de Desenvolvimento](../DEVELOPMENT_GUIDE.md)
4. 🐛 Abra uma issue no [GitHub](https://github.com/marioidival/limit/issues)

## Links úteis

- [README Principal](../README.md)
- [Guia de Desenvolvimento](../DEVELOPMENT_GUIDE.md)
- [Setup Anthropic Claude](CLAUDE_SETUP.md)
- [Setup OpenAI](OPENAI_SETUP.md)
- [Setup z.ai](ZAI_SETUP.md)
