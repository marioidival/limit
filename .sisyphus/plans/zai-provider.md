# Z.AI Provider Implementation Plan

## TL;DR

> **Quick Summary**: Add dedicated Z.AI provider to limit-llm crate. Wraps OpenAI provider with Z.AI-specific defaults and features (thinking mode, reasoning_content).
>
> **Deliverables**:
> - `src/zai_provider.rs` - ZaiProvider implementation
> - Updated config validation and factory
> - Support for reasoning_content streaming
> - Tests and documentation
>
> **Estimated Effort**: Medium (2-3 hours after decisions)
> **Parallel Execution**: NO - Sequential waves due to dependencies
> **Critical Path**: Decisions → Enum Update → Provider Implementation → Tests

---

## Context

### Original Request
Create a Z.AI client as a dedicated provider in limit-llm, instead of using it through OpenAI provider with custom base_url.

### Interview Summary
**Key Discussions**:
- Z.AI is OpenAI-compatible (already works via OpenAI provider)
- Goal: Cleaner config (`provider = "zai"` vs `provider = "openai"` + custom base_url)
- Z.AI has specific features: thinking mode, reasoning_content field, different error format

**Research Findings**:
- OpenAI provider uses standard SSE streaming (compatible with Z.AI)
- Z.AI adds `reasoning_content` field not in current `ProviderResponseChunk` enum
- Z.AI error format differs from OpenAI (numeric codes vs types)
- Default endpoint: `https://api.z.ai/api/coding/paas/v4/chat/completions`

### Metis Review
**Identified Gaps** (5 blocking questions):
1. Should we add `ReasoningDelta` chunk type?
2. Thinking mode default behavior?
3. Preserved thinking support level?
4. Which base URL to use as default?
5. Error handling strategy?

---

## Work Objectives

### Core Objective
Implement dedicated ZAI provider that extends OpenAI provider with Z.AI-specific features while maintaining code reuse and consistency.

### Concrete Deliverables
- `src/zai_provider.rs` - ZaiProvider struct wrapping OpenAiProvider
- Modified `src/config.rs` - ZAI validation and env var support
- Modified `src/provider_factory.rs` - ZAI factory case
- Modified `src/providers.rs` - ReasoningDelta chunk type (if Q1=yes)
- Modified `src/lib.rs` - Export zai_provider module
- `tests/zai_provider_test.rs` - Unit tests
- Updated `README.md` - ZAI configuration example

### Definition of Done
- [ ] `cargo test --package limit-llm` passes all tests
- [ ] ZAI provider creates successfully with default config
- [ ] Streaming includes reasoning_content (if Q1=yes)
- [ ] ZAI errors parse correctly
- [ ] Config validates ZAI provider
- [ ] README includes ZAI example

### Must Have
- Dedicated "zai" provider type
- ZAI_API_KEY env var support
- Default base_url to Z.AI endpoint
- Thinking mode configuration
- Streaming support (reasoning + content)

### Must NOT Have (Guardrails)
- NO vision/multimodal support (defer)
- NO web search integration (defer)
- NO per-request thinking mode override (config-only for v1)
- NO auto-tracking reasoning_content in history (manual for v1)
- NO modification of LlmProvider trait interface

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (mockito + cargo test)
- **Automated tests**: YES (TDD)
- **Framework**: Rust built-in test framework + mockito
- **TDD**: Each task follows RED (failing test) → GREEN (impl) → REFACTOR

### QA Policy
Every task includes unit tests with mocked HTTP responses.
Evidence: Test outputs in cargo test output.

### Test Categories
- Unit: Config parsing, provider creation, streaming mocks
- Integration: Optional (behind feature flag, requires ZAI_API_KEY)

---

## Execution Strategy

### Wave 0: User Decisions (BLOCKING)
Get answers to 5 questions before implementation.

### Wave 1: Core Types & Config (Sequential)
Foundation changes affecting multiple files.

### Wave 2: Provider Implementation (Sequential)
Main implementation work.

### Wave 3: Testing & Documentation (Sequential)
Verification and docs.

---

## TODOs

### User Decisions ✅ CONFIRMED: A A C A A

All decisions confirmed by user:
- ✅ Q1: Add `ReasoningDelta(String)` to `ProviderResponseChunk` enum
- ✅ Q2: Thinking mode disabled by default
- ✅ Q3: Config flag only for preserved thinking (manual tracking)
- ✅ Q4: Use coding path endpoint (`https://api.z.ai/api/coding/paas/v4/chat/completions`)
- ✅ Q5: Parse Z.AI errors and convert to LlmError::ApiError

---

### Wave 1: Core Types & Config

- [x] 1. **Add ReasoningDelta to ProviderResponseChunk enum**
  
  **What to do**:
  - Add `ReasoningDelta(String)` variant to `ProviderResponseChunk` in `src/providers.rs:12-21`
  - Update any exhaustive match statements
  
  **References**:
  - `src/providers.rs:12-21` - Enum definition
  - `src/openai_provider.rs:203-262` - Chunk yielding logic
  
  **Acceptance Criteria**:
  - [ ] Variant added to enum
  - [ ] `cargo check --package limit-llm` passes
  
  **Test**:
  ```rust
  let chunk = ProviderResponseChunk::ReasoningDelta("test".to_string());
  assert!(matches!(chunk, ProviderResponseChunk::ReasoningDelta(_)));
  ```
  
  **Commit**: YES
  - Message: `feat(types): add ReasoningDelta to ProviderResponseChunk`
  - Files: `src/providers.rs`

- [x] 2. **Update config validation for ZAI**
  
  **What to do**:
  - Add "zai" to validation whitelist in `src/config.rs:55`
  - Add ZAI_API_KEY env var support in `api_key_or_env()` at `src/config.rs:42-50`
  
  **References**:
  - `src/config.rs:55` - Validation
  - `src/config.rs:42-50` - Env var handling
  
  **Acceptance Criteria**:
  - [ ] Config validates with `provider = "zai"`
  - [ ] ZAI_API_KEY env var works
  
  **Test**:
  ```rust
  let config = toml::from_str::<Config>(r#"provider="zai"
[providers.zai]
model="glm-4.7""#).unwrap();
  config.validate().unwrap();
  ```
  
  **Commit**: NO (groups with task 3)

- [x] 3. **Add ZAI to provider factory**
  
  **What to do**:
  - Add "zai" match arm in `src/provider_factory.rs:23-42`
  - Set default base_url: `https://api.z.ai/api/coding/paas/v4/chat/completions`
  
  **References**:
  - `src/provider_factory.rs:23-42` - Factory logic
  
  **Acceptance Criteria**:
  - [ ] Factory creates ZAI provider
  - [ ] Default base_url is coding path
  
  **Commit**: YES
  - Message: `feat(config): add ZAI provider support`
  - Files: `src/config.rs`, `src/provider_factory.rs`

---

### Wave 2: Provider Implementation

- [x] 4. **Create ZaiProvider struct**
  
  **What to do**:
  - Create `src/zai_provider.rs`
  - Define `ZaiProvider` wrapping `OpenAiProvider`
  - Add thinking config: `thinking_enabled: bool` (default: false), `clear_thinking: bool` (default: true)
  
  **References**:
  - `src/openai_provider.rs:14-40` - Pattern to follow
  
  **Acceptance Criteria**:
  - [ ] Struct compiles
  - [ ] Default base_url set correctly
  
  **Test**:
  ```rust
  let provider = ZaiProvider::new("key".to_string(), None, "glm-4.7", 4096, 60, ThinkingConfig::default());
  assert_eq!(provider.openai.base_url, "https://api.z.ai/api/coding/paas/v4/chat/completions");
  ```
  
  **Commit**: NO (groups with task 5)

- [x] 5. **Implement LlmProvider trait**
  
  **What to do**:
  - Implement `send()` with thinking mode parameter (if enabled)
  - Parse `reasoning_content` → yield `ReasoningDelta`
  - Yield `ContentDelta`, handle tool calls
  - Return `provider_name() = "zai"`
  
  **References**:
  - `src/openai_provider.rs:44-96` - Trait impl pattern
  
  **Acceptance Criteria**:
  - [ ] Implements LlmProvider
  - [ ] Yields ReasoningDelta chunks
  - [ ] Thinking mode in request body
  
  **Test**:
  ```rust
  // Mock SSE with reasoning_content
  let stream = provider.send(vec![], vec![]).await.unwrap();
  let chunks: Vec<_> = stream.collect().await;
  assert!(chunks.iter().any(|c| matches!(c, Ok(ProviderResponseChunk::ReasoningDelta(_)))));
  ```
  
  **Commit**: YES
  - Message: `feat(provider): implement ZaiProvider with reasoning support`
  - Files: `src/zai_provider.rs`

- [x] 6. **Add ZAI error parsing**
  
  **What to do**:
  - Parse Z.AI format: `{"error":{"code":"1214","message":"..."}}`
  - Convert to `LlmError::ApiError(format!("Code {}: {}", code, message))`
  
  **References**:
  - `src/openai_provider.rs:99-138` - Error handling
  
  **Acceptance Criteria**:
  - [ ] Z.AI errors parse correctly
  
  **Test**:
  ```rust
  // Mock 400 with Z.AI error
  let result = provider.send(vec![], vec![]).await;
  assert!(matches!(result, Err(LlmError::ApiError(msg)) if msg.contains("1214")));
  ```
  
  **Commit**: NO (groups with task 5)

- [x] 7. **Export zai_provider module**
  
  **What to do**:
  - Add `pub mod zai_provider;` to `src/lib.rs`
  - Add `pub use zai_provider::ZaiProvider;` to exports
  
  **References**:
  - `src/lib.rs:1-21` - Module structure
  
  **Acceptance Criteria**:
  - [ ] Module exported
  - [ ] `cargo build --package limit-llm` passes
  
  **Commit**: YES
  - Message: `feat(lib): export ZaiProvider`
  - Files: `src/lib.rs`

---

### Wave 3: Testing & Documentation

- [x] 8. **Create comprehensive unit tests**
  
  **What to do**:
  - Create `tests/zai_provider_test.rs`
  - Test cases: config, factory, streaming w/ reasoning, errors, env var, default URL
  
  **References**:
  - `src/openai_provider.rs:342-493` - Test patterns
  - `tests/provider_switching.rs` - Config tests
  
  **Acceptance Criteria**:
  - [ ] All tests pass
  - [ ] `cargo test --package limit-llm` succeeds
  
  **Commit**: YES
  - Message: `test(zai): add comprehensive unit tests`
  - Files: `tests/zai_provider_test.rs`

- [x] 9. **Update README with ZAI configuration**
  
  **What to do**:
  - Add ZAI config example
  - Document thinking mode and reasoning_content
  
  **Example**:
  ```toml
  provider = "zai"
  [providers.zai]
  model = "glm-4.7"
  # thinking_enabled = true  # optional
  # clear_thinking = false   # preserve reasoning across turns
  ```
  
  **Acceptance Criteria**:
  - [ ] ZAI example in README
  
  **Commit**: YES
  - Message: `docs(readme): add ZAI provider configuration`
  - Files: `README.md`

---

## Final Verification Wave

- [x] F1. Plan Compliance Audit (oracle) - Verify all deliverables present
- [x] F2. Code Quality Review (unspecified-high) - cargo clippy + fmt + test
- [x] F3. Manual QA (unspecified-high) - Config example works
- [x] F4. Scope Fidelity Check (deep) - No scope creep

---

## Commit Strategy

- Commit 1: Add ReasoningDelta to ProviderResponseChunk (if Q1=yes)
- Commit 2: Add ZAI provider implementation
- Commit 3: Add ZAI tests and docs

---

## Success Criteria

### Verification Commands
```bash
cargo test --package limit-llm                    # All tests pass
cargo clippy --package limit-llm -- -D warnings   # No warnings
cargo fmt --package limit-llm -- --check          # Formatted
```

### Final Checklist
- [ ] All "Must Have" features present
- [ ] All "Must NOT Have" features absent
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code formatted
- [ ] README updated
