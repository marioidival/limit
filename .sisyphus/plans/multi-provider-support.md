# Multi-Provider LLM Support (Revised)

## TL;DR

> **Summary**: Complete multi-provider support (Claude, OpenAI, z.ai) by finishing partial impl in providers.rs, adding OpenAI provider, replacing config, and wiring factory.
>
> **Deliverables**:
> - Exported `providers` module with trait
> - `impl LlmProvider for AnthropicClient`
> - `OpenAiProvider` (OpenAI + z.ai via base_url)
> - New config schema with `provider` field
> - `ProviderFactory` → `Box<dyn LlmProvider>`
> - Updated `AgentBridge`
>
> **Effort**: Medium (1 day)
> **Parallel**: YES - 4 waves, 8 tasks
> **Critical Path**: Config → AnthropicImpl → OpenAI → Factory → Bridge

---

## Context

### Current State
- **providers.rs**: Trait + enums + configs exist (not exported)
- **client.rs**: AnthropicClient ready (missing trait methods)
- **config.rs**: Single-provider (needs replacement)
- **agent_bridge.rs**: Uses AnthropicClient directly

### Key Decisions
| Decision | Choice |
|----------|--------|
| ResponseChunk | Consolidate → ProviderResponseChunk |
| Anthropic impl | Direct on AnthropicClient |
| Config | Breaking change, `provider` field |
| Env priority | Config wins (env is fallback) |
| z.ai URL | Full URL required |
| Tests | After implementation |

### Metis Findings
- OpenAI SSE format differs (separate parser)
- Two config systems (Config vs ProviderConfig)
- Tool call formats differ between providers

---

## Work Objectives

### Must Have
- `provider = "anthropic"|"openai"` in config
- Env fallback: ANTHROPIC_API_KEY, OPENAI_API_KEY, ZAI_API_KEY
- Factory returns `Box<dyn LlmProvider>`
- z.ai via `base_url` (full URL)

### Must NOT
- CLI --provider flag
- Config backward compat
- Common SSE abstraction
- BaseProvider trait
- SDKs

---

## Execution Strategy

```
Wave 1 (3 tasks, PARALLEL):
├── T1: Export providers + consolidate types [quick]
├── T2: Replace config schema [quick]
└── T3: Impl LlmProvider for AnthropicClient [quick]

Wave 2 (1 task):
└── T4: Create OpenAiProvider [deep]

Wave 3 (2 tasks, SEQUENTIAL):
├── T5: Create ProviderFactory [quick]
└── T6: Update AgentBridge [unspecified-high]

Wave 4 (2 tasks, PARALLEL):
├── T7: Config validation + errors [quick]
└── T8: Integration tests [unspecified-high]
```

---

- [x] 1. **Export providers module + consolidate types**

  **What**: 
  - Add `pub mod providers;` to lib.rs
  - Remove `ResponseChunk` enum from client.rs
  - Replace ResponseChunk → ProviderResponseChunk
  - Add `pub use providers::ProviderResponseChunk;` to lib.rs
  - Update imports in agent_bridge.rs

  **Must NOT**: Rename ProviderResponseChunk, create aliases

  **References**:
  - `limit-llm/src/lib.rs` - Add module export
  - `limit-llm/src/client.rs:19-27` - ResponseChunk to remove
  - `limit-llm/src/providers.rs:12-20` - Target type
  - `limit-cli/src/agent_bridge.rs:10` - Import to update

  **QA**: `cargo check --workspace && cargo test --workspace` → PASS
- [x] 2. **Replace config.rs schema**

  **What**:
  - New schema:
    ```toml
    provider = "anthropic"
    [providers.anthropic]
    api_key = "..."  # optional, falls back to ANTHROPIC_API_KEY
    model = "claude-3-5-sonnet-20241022"
    [providers.openai]
    api_key = "..."  # optional, falls back to OPENAI_API_KEY
    model = "gpt-4"
    base_url = "https://api.z.ai/api/paas/v4/chat/completions"
    ```
  - Add `api_key_or_env()` (config wins, env fallback)
  - Error on old format

  **Must NOT**: Keep old Config, dual format support

  **References**:
  - `limit-llm/src/config.rs:6-61` - Replace
  - `limit-llm/src/providers.rs:60-85` - Config patterns

  **QA**: New config parses, env fallback works, old format errors
- [x] 3. **Impl LlmProvider for AnthropicClient**

  **What**:
  - Add trait methods:
    - `provider_name() -> &str` → "anthropic"
    - `model_name() -> &str` → `&self.model`
    - `clone_box() -> Box<dyn LlmProvider>`
  - send() already matches trait

  **Must NOT**: Modify send() logic

  **References**:
  - `limit-llm/src/providers.rs:24-40` - Trait
  - `limit-llm/src/client.rs:72-76` - send()

  **QA**: `cargo test -p limit-llm` → PASS
- [x] 4. **Create OpenAiProvider**

  **What**:
  - Create `limit-llm/src/openai_provider.rs`
  - Impl LlmProvider trait
  - OpenAI SSE format: `data: {"choices":[{"delta":{"content":"..."}}]}`
  - Tool calls: `choices[0].delta.tool_calls`
  - Terminator: `data: [DONE]`
  - z.ai via base_url

  **Must NOT**: Abstract SSE, reuse Anthropic parser, add retry

  **References**:
  - `limit-llm/src/client.rs:100-200` - HTTP pattern
  - `limit-llm/src/providers.rs:24-40` - Trait
  - OpenAI API docs

  **QA**: OpenAI streaming, z.ai, tool calls (mock tests)
- [x] 5. **Create ProviderFactory**

  **What**:
  - Create `limit-llm/src/provider_factory.rs`
  - `create_provider(config) -> Result<Box<dyn LlmProvider>, LlmError>`
  - Match on `config.provider`:
    - "anthropic" → AnthropicClient
    - "openai" → OpenAiProvider
  - Error on unknown

  **Must NOT**: Auto-detect, lazy loading

  **References**:
  - `limit-llm/src/config.rs` - Config
  - `limit-llm/src/client.rs` - AnthropicClient
  - `limit-llm/src/openai_provider.rs` - OpenAiProvider

  **QA**: Correct types returned, unknown errors
- [x] 6. **Update AgentBridge**

  **What**:
  - Replace `llm_client: AnthropicClient` → `Box<dyn LlmProvider>`
  - Use `ProviderFactory::create_provider(&config)`
  - Update ResponseChunk → ProviderResponseChunk

  **Must NOT**: Change behavior

  **References**:
  - `limit-cli/src/agent_bridge.rs:37` - Field type
  - `limit-cli/src/agent_bridge.rs:56-68` - Instantiation

  **QA**: `cargo test --workspace` → PASS
- [x] 7. **Config validation + errors**

  **What**:
  - `Config::validate() -> Result<(), ConfigError>`
  - Errors: missing provider, unknown provider, missing api_key, old format
  - Call in `Config::load()`

  **Must NOT**: Auto-fix, silent fallback

  **References**:
  - `limit-llm/src/config.rs`

  **QA**: All error cases
- [x] 8. **Integration tests**

  **What**:
  - Tests in `limit-llm/tests/`:
    - provider_switching.rs
    - env_var_fallback.rs
    - config_validation.rs
  - Update README

  **Must NOT**: E2E tests

  **QA**: `cargo test --workspace` → all pass

  **Commit**: YES
  - Message: `feat(llm): add multi-provider support`
  - Pre-commit: `cargo test --workspace && cargo clippy --workspace`
---

## Final Verification

- [ ] F1. Plan Compliance — oracle
- [x] F2. Code Quality — unspecified-high
- [x] F3. Manual QA — unspecified-high
- [ ] F4. Scope Fidelity — deep

---

## Success Criteria

```bash
cargo run -- chat "hello"  # uses provider from config
cargo test --workspace && echo "PASS"
```

## Unresolved Questions

None.
