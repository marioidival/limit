# Configurando z.ai com Limit

Este guia mostra como configurar o Limit para usar a API do z.ai (ZAI Provider).

## Pré-requisitos

- [x] Limit instalado (veja [README.md](../README.md#quick-start))
- [x] Uma conta no [z.ai](https://z.ai/)
- [x] Uma API Key do z.ai

## Passo 1: Obter uma API Key

1. Acesse [https://z.ai/](https://z.ai/)
2. Faça login na sua conta z.ai
3. Vá para **Configurações** → **API Keys**
4. Clique em **"Generate New Key"**
5. Dê um nome descritivo (ex: "Limit CLI")
6. Copie a key gerada
   - ⚠️ **Importante:** Copie agora, você não poderá ver a key novamente!

## Passo 2: Criar arquivo de configuração

Crie o diretório e arquivo de configuração:

```bash
mkdir -p ~/.limit
nano ~/.limit/config.toml
```

Cole o seguinte conteúdo:

```toml
provider = "zai"

[providers.zai]
# api_key é opcional - fallback para ZAI_API_KEY env var
api_key = "..."
model = "glm-4.7"
max_tokens = 4096
timeout = 300000
# Optional: Enable thinking mode
# thinking_enabled = true
```

### Configuração alternativa (via variável de ambiente)

Se você preferir não expor a API key no arquivo de configuração:

```bash
export ZAI_API_KEY="..."
```

Ou para persistir no seu shell (adicione ao `~/.bashrc`, `~/.zshrc`, etc.):

```bash
echo 'export ZAI_API_KEY="..."' >> ~/.bashrc
source ~/.bashrc
```

### Variáveis de ambiente suportadas

O Limit suporta a variável `ZAI_API_KEY` para o provedor z.ai.

## Passo 3: Modelos disponíveis

### Recomendados

| Modelo | Descrição | Use para |
|--------|-----------|----------|
| `glm-4.7` | Modelo mais recente e capaz | Tarefas complexas, código, análise |
| `glm-4` | Versão estável do GLM-4 | Desenvolvimento geral |
| `glm-3-turbo` | Mais rápido e econômico | Tarefas simples, debugging rápido |

### Exemplo de configuração para GLM-4.7 (recomendado)

```toml
provider = "zai"

[providers.zai]
api_key = "..."
model = "glm-4.7"
max_tokens = 4096
timeout = 300000
```

### Exemplo de configuração para GLM-3 Turbo

```toml
provider = "zai"

[providers.zai]
api_key = "..."
model = "glm-3-turbo"
max_tokens = 4096
timeout = 300000
```

## Passo 4: Modo Thinking (Avançado)

O z.ai oferece um modo de "thinking" que pode melhorar a qualidade das respostas:

```toml
[providers.zai]
api_key = "..."
model = "glm-4.7"
thinking_enabled = true
max_tokens = 4096
timeout = 300000
```

**Quando usar thinking_enabled:**
- ✅ Tarefas complexas de programação
- ✅ Análise de arquitetura
- ✅ Resolução de bugs difíceis
- ⚠️ Aumenta o tempo de resposta
- ⚠️ Pode aumentar o custo

## Passo 5: Testar a configuração

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
Provider: zai
Model: glm-4.7
Max Tokens: 4096
```

## Passo 6: Testar uma conversa

Faça uma pergunta simples para testar:

```
lim> Olá! Você está funcionando corretamente?
```

## Troubleshooting

### Erro: "Invalid API key"

**Causa:** API key incorreta ou não configurada.

**Solução:**
1. Verifique se a key está correta em `~/.limit/config.toml`
2. Ou verifique se `ZAI_API_KEY` está definida:
   ```bash
   echo $ZAI_API_KEY
   ```

### Erro: "Rate limit exceeded"

**Causa:** Você excedeu o limite de requisições da sua conta.

**Solução:**
1. Verifique seus limites no painel do z.ai
2. Aguarde alguns minutos antes de tentar novamente
3. Considere fazer upgrade da sua conta

### Erro: "Timeout"

**Causa:** O modelo demorou muito para responder.

**Solução:** Aumente o timeout:

```toml
[providers.zai]
timeout = 600000  # 10 minutos em milissegundos
```

### Erro: "Model not found"

**Causa:** Nome do modelo incorreto ou não disponível na sua conta.

**Solução:**
1. Verifique os modelos disponíveis na documentação do z.ai
2. Verifique se o modelo está habilitado na sua conta

## Dicas avançadas

### Ajustando max_tokens

- **Para respostas curtas:** `max_tokens = 512`
- **Para código:** `max_tokens = 2048`
- **Para documentação:** `max_tokens = 4096`
- **Para outputs longos:** `max_tokens = 8192`

### Múltiplos modelos para diferentes usos

Você pode criar arquivos de configuração diferentes:

```bash
# Config para desenvolvimento rápido
~/.limit/config-dev.toml  # com glm-3-turbo

# Config para tarefas complexas
~/.limit/config-prod.toml  # com glm-4.7 + thinking_enabled
```

E alternar entre eles:

```bash
lim --config ~/.limit/config-dev.toml
```

### Configuração com modo thinking ativado

```toml
[providers.zai]
api_key = "..."
model = "glm-4.7"
thinking_enabled = true
max_tokens = 4096
timeout = 600000  # Timeout maior para modo thinking
```

## Comparação: z.ai vs Outros Modelos

### Quando usar z.ai:

✅ **z.ai é bom para:**
- Custo-benefício competitivo
- Suporte a português nativo
- Modelo GLM-4.7 é muito capaz
- Mode thinking para tarefas complexas

⚠️ **Considere outros modelos para:**
- Integração com ferramentas OpenAI (use OpenAI)
- Necessidade de Claude específico (use Anthropic)
- Requisitos de compliance específicos

## Links úteis

- [Documentação z.ai](https://z.ai/docs)
- [Painel z.ai](https://z.ai/dashboard)
- [GLM-4 Model](https://z.ai/docs/models/glm-4)
- [README do Limit](../README.md)

## Próximos passos

- 📖 Aprenda sobre [OpenAI setup](OPENAI_SETUP.md)
- 📖 Aprenda sobre [Claude setup](CLAUDE_SETUP.md)
- 📖 Veja os [comandos disponíveis](../README.md#available-commands)
- 📖 Aprenda sobre [sessões](../README.md#session-persistence)
