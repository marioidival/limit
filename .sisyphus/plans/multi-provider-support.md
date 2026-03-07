# Multi-Provider Support (v4)

## TL;DR
Add OpenAI + z.ai providers. Trait-based abstraction. Provider sections in config. First-run setup wizard. Normalized tool calling (definitions + results).

**Deliverables**: LlmProvider trait | AnthropicProvider | OpenAiProvider | ZaiProvider (exp) | MultiProviderConfig | First-run setup | Tool norm (def+results)
**Effort**: Large (5-7h) | **Parallel**: YES (4 waves) | **Critical Path**: W1 → W2 → W3 → W4

---

## Context

### What Changed Since v1
- ✅ `base_url` added (commit 1c57125) - reuse for ProviderConfig.base_url
- ✅ z.ai SSE parsing fixed (commit 822c078) - works via base_url
- ✅ Tracing/logging added (commit a08b47d)
- ⚠️ client.rs significantly changed - update refs

### Provider Research Summary (2026-03-07)

| Provider | Type | Auth | Endpoint | Default Model | Pricing (1M tokens) |
|----------|------|------|----------|---------------|---------------------|
| **Anthropic** | Native | `x-api-key` + `anthropic-version` | `/v1/messages` | `claude-sonnet-4-6` | $3 / $15 |
| **OpenAI** | Native | `Authorization: Bearer` | `/v1/chat/completions` | `gpt-5-mini` | $0.25 / $2.00 |
| **z.ai** | OpenAI-compatible | `Authorization: Bearer` | `https://api.z.ai/api/paas/v4/` | `glm-5` | $1 / $3.2 |

### API Format Differences

| Aspect | Anthropic | OpenAI | z.ai |
|--------|-----------|--------|------|
| **Messages** | `{role, content}` | `messages[]` array | Same as OpenAI |
| **Streaming** | SSE events (message_start, content_block_delta) | SSE chunks with `data:` | Same as OpenAI |
| **Tool Calling** | `tool_use` / `tool_result` | `tool_calls[]` array | Same as OpenAI |
| **Tool Result** | `{tool_use_id, output}` | `{tool_call_id, content}` | Same as OpenAI |
| **Function Args** | Accumulated JSON | Complete JSON string | Batched (30s timeout risk) |

### Decisions (confirmed + Metis)
- Config: Provider sections `[providers.X]`
- Session: One provider per session
- Default: First configured provider (alphabetical if no `default` key)
- Tests: TDD
- Arch: Trait-based abstraction
- First-run: Interactive provider setup wizard
- Config: Replace flat Config with MultiProviderConfig
- **NEW**: Tool RESULT normalization (not just definitions)
- **NEW**: z.ai experimental (30s timeout), auto fallback to non-streaming
- **NEW**: Non-interactive mode: Error with instructions
- **NEW**: Session-provider mismatch: Prompt user to choose new provider
- **NEW**: Migration failure modes: Handle missing api_key, corrupted config, permission errors

### Default Models per Provider
```rust
match provider_type {
    ProviderType::Anthropic => "claude-sonnet-4-6",
    ProviderType::OpenAI => "gpt-5-mini",
    ProviderType::Zai => "glm-5",
}
```

### Current State
```
limit-llm/src/
├── client.rs      # AnthropicClient (works for z.ai via base_url)
├── config.rs      # Flat config (api_key, model, max_tokens, base_url)
├── types.rs       # Message, Tool, ToolCall, etc.
├── error.rs       # LlmError (generic)
└── lib.rs         # Exports
```

---

## Objectives

**Core**: Multi-provider via unified trait. First-run wizard. Silent migration.

**CRITICAL ADDITIONS** (from Metis review):
- Tool RESULT normalization (not just definitions)
- z.ai experimental mode with auto fallback
- Non-interactive mode detection
- Session-provider mismatch handling

**Deliverables**:
- `limit-llm/src/provider.rs`: LlmProvider trait
- `limit-llm/src/anthropic.rs`: AnthropicProvider (refactored)
- `limit-llm/src/openai.rs`: OpenAiProvider (native format)
- `limit-llm/src/zai.rs`: ZaiProvider (OpenAI-compatible, **experimental**)
- `limit-llm/src/config.rs`: MultiProviderConfig (replaces Config)
- `limit-cli/src/setup.rs`: First-run provider setup wizard
- Tool normalization: Definitions + **RESULTS**
- Tests: Unit + integration (mockito)

**Done When**:
- [ ] cargo test --workspace passes
- [ ] First run without config triggers setup wizard
- [ ] Existing flat configs migrate silently
- [ ] Can use Anthropic with `[providers.anthropic]`
- [ ] Can use OpenAI with `[providers.openai]`
- [ ] Can use z.ai with `[providers.zai]` (experimental warning shown)
- [ ] Tool calling (def + result) identical across providers
- [ ] z.ai timeout triggers auto fallback
- [ ] Non-interactive mode shows error with instructions
- [ ] Session-provider mismatch prompts user
- [ ] Clear errors for invalid configs

**Must Have**: Trait | AnthropicProvider | OpenAiProvider | ZaiProvider (exp) | MultiProviderConfig | First-run setup | Tool norm (def+results) | Migration | Tests

**Must NOT**: Break existing users (migration required) | Multi-provider sessions | Dynamic switching | CLI flags | Expose provider-specific features

---

## Waves

```
W1 (Foundation):
├── T1: LlmProvider trait + ProviderError + ProviderType [no deps]
├── T2: MultiProviderConfig types [no deps]
├── T3: Tool definition normalization [no deps]
├── T3.5: Tool RESULT normalization [depends: T3]
└── T4: ProviderConfig tests [depends: T2]

W2 (Providers - after W1 complete):
├── T5: Research Anthropic API + implement AnthropicProvider [depends: T1, T3, T3.5]
├── T6: Research OpenAI API + implement OpenAiProvider [depends: T1, T3, T3.5]
└── T7: Research z.ai API + implement ZaiProvider (exp) [depends: T1, T3, T3.5]

W3 (Config + CLI - after W2 complete):
├── T8: Config loader [depends: T2]
├── T9: First-run setup wizard (non-interactive detect) [depends: T2]
├── T10: Config migration (failure modes) [depends: T2]
└── T11: Update limit-cli (session-provider mismatch) [depends: T5, T6, T7, T8]

W4 (Verification - after W3 complete):
├── T12: Integration tests (tool results) [depends: T5, T6, T7]
├── T13: Manual QA [depends: T11]
├── T14: Backward compat [depends: T10, T11]
└── T15: Docs [depends: T11]

Total: 15 tasks + 4 final verification
Critical Path: T1/T2/T3 → T3.5/T4 → T5/T6/T7 → T8/T9/T10 → T11 → T12/T13/T14/T15
```

---

## Tasks

### Wave 1

**DEPENDENCIES**: T1-T3 parallel (no deps) → T3.5 (needs T3) → T4 (needs T2)

- [ ] **T1. LlmProvider trait + ProviderError + ProviderType**
  - Create `provider.rs` with trait:
    ```rust
    pub trait LlmProvider: Send + Sync {
      fn name(&self) -> &str;
      fn default_model(&self) -> &'static str;
      fn is_experimental(&self) -> bool { false }
      async fn send(&self, messages: Vec<Message>, tools: Vec<Tool>) 
        -> Pin<Box<dyn Stream<Item = Result<ResponseChunk, ProviderError>> + Send + '_>>;
    }
    ```
  - Extend `error.rs` with ProviderError:
    ```rust
    pub enum ProviderError {
      AuthFailed(String),
      RateLimited(String),
      InvalidResponse(String),
      NetworkError(String),
      ConfigError(String),
      NoProviderConfigured,
      ProviderSpecific { provider: String, code: String, message: String },
      MigrationError(String),
      SessionError(String),
    }
    ```
  - Add to `types.rs`:
    ```rust
    #[derive(Debug, Deserialize, Clone, PartialEq)]
    #[serde(rename_all = "lowercase")]
    pub enum ProviderType { Anthropic, OpenAI, Zai }
    
    impl ProviderType {
      pub fn default_model(&self) -> &'static str {
        match self {
          Self::Anthropic => "claude-sonnet-4-6",
          Self::OpenAI => "gpt-5-mini",
          Self::Zai => "glm-5",
        }
      }
    }
    ```
  - Test: trait object-safe, error conversions, default models
  - Refs: client.rs:45-118 (current send impl)
  - Commit: NO

- [ ] **T2. MultiProviderConfig types**
  - Replace `Config` in `config.rs`:
    ```rust
    #[derive(Debug, Deserialize, Clone)]
    pub struct ProviderConfig {
      pub api_key: String,  // REQUIRED (not Option)
      #[serde(default)]
      pub model: Option<String>,
      #[serde(default = "default_max_tokens")]
      pub max_tokens: u32,
      #[serde(default = "default_timeout")]
      pub timeout: u64,
      pub base_url: Option<String>,
      #[serde(default)]
      pub provider_type: ProviderType,
    }
    
    #[derive(Debug, Deserialize, Clone)]
    pub struct MultiProviderConfig {
      pub providers: std::collections::HashMap<String, ProviderConfig>,
      #[serde(default)]
      pub default: Option<String>,
    }
    
    impl MultiProviderConfig {
      pub fn has_providers(&self) -> bool { !self.providers.is_empty() }
      pub fn validate(&self) -> Result<(), ProviderError> { ... }
    }
    ```
  - Test: serialize/deserialize TOML, validation
  - Refs: config.rs:6-17 (current Config)
  - Commit: NO

- [ ] **T3. Tool definition normalization**
  - Add to `types.rs`:
    ```rust
    impl Tool {
      /// Anthropic format: {name, description, input_schema}
      pub fn to_anthropic_format(&self) -> Value { ... }
      
      /// OpenAI format: {type: "function", function: {name, description, parameters}}
      pub fn to_openai_format(&self) -> Value { ... }
    }
    ```
  - Test: conversion, edge cases
  - Refs: types.rs:19-46
  - Commit: NO

- [ ] **T3.5. Tool RESULT normalization** (CRITICAL - from Metis)
  - Add to `types.rs`:
    ```rust
    /// Normalized tool result (provider-agnostic)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ToolResult {
      pub tool_use_id: String,
      pub content: String,
      pub is_error: bool,
    }
    
    impl ToolResult {
      /// Parse from Anthropic format: {\"tool_use_id\": \"...\", \"output\": ...}
      pub fn from_anthropic(block: &Value) -> Result<Self, ProviderError> { ... }
      
      /// Parse from OpenAI format: {\"tool_call_id\": \"...\", \"content\": ...}
      pub fn from_openai(block: &Value) -> Result<Self, ProviderError> { ... }
      
      /// Convert to Anthropic message for sending back
      pub fn to_anthropic_message(&self) -> Value {
        json!({
          "type": "tool_result",
          "tool_use_id": self.tool_use_id,
          "content": self.content,
        })
      }
      
      /// Convert to OpenAI message for sending back
      pub fn to_openai_message(&self) -> Value {
        json!({
          "role": "tool",
          "tool_call_id": self.tool_use_id,
          "content": self.content,
        })
      }
    }
    ```
  - Update `Message` type to use `ToolResult`
  - Update `agent_bridge.rs:329-333` to use normalized format
  - Test: bidirectional conversion, error cases
  - Refs: agent_bridge.rs:329-333 (current tool result handling)
  - Commit: NO

- [ ] **T4. ProviderConfig tests**
  - Test TOML parsing:
    ```toml
    [providers.anthropic]
    api_key = "sk-ant-..."
    model = "claude-opus-4-6"
    
    [providers.openai]
    api_key = "sk-..."
    provider_type = "openai"
    
    [providers.zai]
    api_key = "zai-..."
    provider_type = "zai"
    base_url = "https://api.z.ai/api/paas/v4/"
    ```
  - Test: empty providers HashMap, missing api_key, unknown provider_type
  - Commit: NO

### Wave 2

**DEPENDENCIES**: All depend on W1 complete (T1, T3, T3.5)

- [ ] **T5. Research Anthropic API + AnthropicProvider**
  
  **Research Phase** (READ-ONLY):
  - Read official docs: https://docs.anthropic.com/en/api/messages
  - Understand: endpoint format, auth headers, request/response structure
  - Understand: SSE streaming events (message_start, content_block_delta, etc.)
  - Understand: tool_use / tool_result format
  - Understand: error codes (400, 401, 429, 500, 529)
  - Document findings in code comments
  
  **Implementation Phase**:
  - Create `anthropic.rs`
  - Extract logic from `client.rs:AnthropicClient`
  - Implement `LlmProvider` trait
  - Default model: `claude-sonnet-4-6`
  - Headers:
    ```
    x-api-key: {api_key}
    anthropic-version: 2023-06-01
    content-type: application/json
    ```
  - Endpoint: `https://api.anthropic.com/v1/messages` (or base_url)
  - Use `to_anthropic_format()` for tools
  - Use `ToolResult::from_anthropic()` for results
  - Reuse SSE parsing from client.rs:202-320
  - Test: all existing client tests pass via provider
  - Refs: client.rs (full file)
  - Commit: YES (`feat(llm): implement AnthropicProvider`)

- [ ] **T6. Research OpenAI API + OpenAiProvider**
  
  **Research Phase** (READ-ONLY):
  - Read official docs: https://platform.openai.com/docs/api-reference/chat
  - Understand: endpoint format, auth header
  - Understand: request/response structure (messages array, choices)
  - Understand: SSE streaming format (data: chunks, [DONE])
  - Understand: function calling (tools, tool_calls)
  - Understand: error codes (400, 401, 403, 429, 500, 503)
  - Document findings in code comments
  
  **Implementation Phase**:
  - Create `openai.rs`
  - Implement `LlmProvider` trait
  - Default model: `gpt-5-mini`
  - Headers:
    ```
    Authorization: Bearer {api_key}
    Content-Type: application/json
    ```
  - Endpoint: `https://api.openai.com/v1/chat/completions` (or base_url)
  - Parse OpenAI streaming:
    ```json
    {"choices":[{"delta":{"content":"Hello"}}]}
    {"choices":[{"delta":{"tool_calls":[{"id":"x","function":{"name":"y","arguments":"{}"}}]}}]}
    data: [DONE]
    ```
  - Use `to_openai_format()` for tools
  - Use `ToolResult::from_openai()` for results
  - Test: mock server, verify request/response format
  - Refs: client.rs (pattern)
  - Commit: YES (`feat(llm): implement OpenAiProvider`)

- [ ] **T7. Research z.ai API + ZaiProvider** (EXPERIMENTAL)
  
  **Research Phase** (READ-ONLY):
  - Read docs: https://open.bigmodel.cn/dev/api
  - **CRITICAL**: Test actual API calls (CONFIRM compatibility, not assume)
  - Document: 30s timeout, `reasoning_content` field
  - Document: Any incompatibilities found
  - Document: Tool result format differences
  
  **Implementation Phase**:
  - Create `zai.rs`
  - Implement `LlmProvider` trait (extends OpenAiProvider pattern)
  - Default model: `glm-5`
  - Default base_url: `https://api.z.ai/api/paas/v4/`
  - Same auth as OpenAI
  - **EXPERIMENTAL MODE**:
    ```rust
    impl LlmProvider for ZaiProvider {
      fn name(&self) -> &str { "z.ai (experimental)" }
      fn is_experimental(&self) -> bool { true }
    }
    ```
  - **AUTO FALLBACK** (on 30s timeout):
    ```rust
    // Detect timeout and fallback to non-streaming
    async fn send(&self, messages: Vec<Message>, tools: Vec<Tool>) -> ... {
      let start = Instant::now();
      match self.try_streaming(messages.clone(), tools.clone()).await {
        Ok(stream) => Ok(stream),
        Err(ProviderError::NetworkError(_)) if start.elapsed() > Duration::from_secs(25) => {
          warn!("z.ai streaming timeout, falling back to non-streaming mode");
          self.send_non_streaming(messages, tools).await
        }
        Err(e) => Err(e),
      }
    }
    ```
  - Test: mock server, timeout simulation
  - Refs: openai.rs (reuse logic)
  - Commit: YES (`feat(llm): implement ZaiProvider (experimental)`)

### Wave 3

**DEPENDENCIES**: T8-T10 depend on T2, T11 depends on T5-T7 + T8

- [ ] **T8. Config loader**
  - Replace `Config` with `MultiProviderConfig` in lib.rs exports
  - Add to `config.rs`:
    ```rust
    impl MultiProviderConfig {
      pub fn load() -> Result<Self, io::Error> { ... }
      pub fn get_default_provider(&self) -> Result<Box<dyn LlmProvider>, ProviderError> {
        let name = self.default.as_ref()
          .or_else(|| self.providers.keys().next())  // first alphabetically
          .ok_or(ProviderError::NoProviderConfigured)?;
        
        let config = self.providers.get(name)
          .ok_or(ProviderError::ConfigError(format!("Provider '{}' not found", name)))?;
        
        match config.provider_type {
          ProviderType::Anthropic => Ok(Box::new(AnthropicProvider::new(config)?)),
          ProviderType::OpenAI => Ok(Box::new(OpenAiProvider::new(config)?)),
          ProviderType::Zai => Ok(Box::new(ZaiProvider::new(config)?)),
        }
      }
    }
    ```
  - Test: multi-provider, single provider, no providers, missing default
  - Refs: config.rs:36-48 (current load)
  - Commit: NO

- [ ] **T9. First-run setup wizard** (non-interactive detection)
  - Create `limit-cli/src/setup.rs`
  - **NON-INTERACTIVE DETECTION**:
    ```rust
    pub fn should_run_wizard() -> Result<bool, ProviderError> {
      // Skip if --help or --version
      if std::env::args().any(|a| a == "--help" || a == "--version") {
        return Ok(false);
      }
      
      // Error if non-interactive
      if !atty::is(atty::Stream::Stdin) {
        eprintln!("Error: Interactive setup required.");
        eprintln!("Run 'limit' with a terminal to configure your first provider.");
        eprintln!("Or manually create ~/.limit/config.toml");
        std::process::exit(1);
      }
      
      // Check if config has providers
      let config = MultiProviderConfig::load()?;
      Ok(!config.has_providers())
    }
    ```
  - Prompt: "No provider configured. Choose one:"
    ```
    1. Anthropic (Claude) - Recommended
       Model: claude-sonnet-4-6 | Pricing: $3/1M input, $15/1M output
    
    2. OpenAI (GPT) - Cost-efficient for coding
       Model: gpt-5-mini | Pricing: $0.25/1M input, $2/1M output
    
    3. z.ai (GLM) - ⚠️ EXPERIMENTAL
       Model: glm-5 | Pricing: $1/1M input, $3.2/1M output
       Limitation: 30s streaming timeout (auto fallback enabled)
    ```
  - Ask for API key (masked input)
  - Ask for model (show default)
  - Write config to `~/.limit/config.toml`
  - Test: interactive flow, non-interactive error, config written
  - Refs: atty crate, dialoguer crate
  - Commit: YES (`feat(cli): add first-run provider setup wizard`)

- [ ] **T10. Config migration** (failure modes)
  - Add to `config.rs`:
    ```rust
    pub fn migrate_or_load() -> Result<MultiProviderConfig, ProviderError> {
      let path = config_path();
      
      // No config exists -> trigger wizard
      if !path.exists() {
        return Err(ProviderError::NoProviderConfigured);
      }
      
      let content = fs::read_to_string(&path)
        .map_err(|e| ProviderError::MigrationError(format!("Failed to read config: {}", e)))?;
      
      // Try new format first
      if let Ok(config) = toml::from_str::<MultiProviderConfig>(&content) {
        return Ok(config);
      }
      
      // Try old format (flat)
      if let Ok(old) = toml::from_str::<Config>(&content) {
        return migrate_flat_config(&path, old);
      }
      
      Err(ProviderError::MigrationError("Config file is corrupted".to_string()))
    }
    
    fn migrate_flat_config(path: &Path, old: Config) -> Result<MultiProviderConfig, ProviderError> {
      // Validate
      let api_key = old.api_key
        .ok_or(ProviderError::MigrationError("Missing api_key in config".to_string()))?;
      
      let migrated = MultiProviderConfig {
        providers: [("anthropic".to_string(), ProviderConfig {
          api_key,
          model: Some(old.model),
          provider_type: ProviderType::Anthropic,
          base_url: old.base_url,
          ..Default::default()
        })].into_iter().collect(),
        default: Some("anthropic".to_string()),
      };
      
      // Validate migrated config
      migrated.validate()?;
      
      // Write to temp file
      let temp_path = path.with_extension("toml.tmp");
      let content = toml::to_string(&migrated)
        .map_err(|e| ProviderError::MigrationError(format!("Failed to serialize: {}", e)))?;
      fs::write(&temp_path, &content)
        .map_err(|e| ProviderError::MigrationError(format!("Failed to write temp: {}", e)))?;
      
      // Backup original
      fs::copy(path, path.with_extension("toml.bak"))
        .map_err(|e| ProviderError::MigrationError(format!("Failed to backup: {}", e)))?;
      
      // Atomic rename
      fs::rename(&temp_path, path)
        .map_err(|e| ProviderError::MigrationError(format!("Failed to rename: {}", e)))?;
      
      info!("Migrated config from flat format to multi-provider format");
      Ok(migrated)
    }
    ```
  - **Error Cases**:
    - `MissingApiKey`: Trigger wizard
    - `CorruptedConfig`: Trigger wizard with warning
    - `PermissionDenied`: Exit with clear message
    - `DiskFull`: Exit with clear message
  - Test: migration preserves all values, error cases
  - Refs: config.rs
  - Commit: NO

- [ ] **T11. Update limit-cli** (session-provider mismatch)
  - Replace `Config` with `MultiProviderConfig` everywhere
  - **SESSION-PROVIDER MISMATCH**:
    ```rust
    // In session loading:
    pub fn load_session(path: &Path, config: &MultiProviderConfig) -> Result<Session, SessionError> {
      let session: Session = bincode::deserialize(&fs::read(path)?)?;
      
      // Check if provider still exists
      if !config.providers.contains_key(&session.provider_name) {
        return Err(SessionError::ProviderNotFound {
          missing: session.provider_name,
          available: config.providers.keys().cloned().collect(),
        });
      }
      
      Ok(session)
    }
    
    // In main.rs:
    match session::load_session(&session_path, &config) {
      Err(SessionError::ProviderNotFound { missing, available }) => {
        println!("⚠️  Provider '{}' was removed from config.", missing);
        println!("Available providers: {}", available.join(", "));
        println!("\nChoose a new provider:");
        let new_provider = prompt_provider_selection(&available)?;
        session.update_provider(new_provider);
      }
      other => other,
    }
    ```
  - Update `agent_bridge.rs` to use `Box<dyn LlmProvider>`
  - **Migration Checklist**:
    - [ ] Replace all `AnthropicClient` with `Box<dyn LlmProvider>`
    - [ ] Update all `.send()` calls to use trait method
    - [ ] Update all `config.api_key` references to `MultiProviderConfig`
    - [ ] Update tests to mock `LlmProvider` trait
  - Test: CLI works, mismatch triggers prompt
  - Refs: agent_bridge.rs, tui_bridge.rs, main.rs
  - Commit: NO

### Wave 4

**DEPENDENCIES**: All depend on W3 complete (T11 working)

- [ ] **T12. Integration tests** (tool results)
  - **Test Migration Plan**:
    - [ ] Move Anthropic-specific tests from client.rs to AnthropicProvider
    - [ ] Create provider-agnostic tests using `Box<dyn LlmProvider>`
    - [ ] Update mocks to be provider-agnostic
  - Mock Anthropic API:
    ```rust
    mock.mock("POST", "/v1/messages")
      .with_chunked_body(|w| {
        w.write_all(b"data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello\"}}\n\n")?;
        w.write_all(b"data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"tool_use\",\"id\":\"t1\",\"name\":\"test\"}}\n\n")?;
        w.write_all(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{...}}\n\n")?;
        Ok(())
      })
    ```
  - Mock OpenAI API:
    ```rust
    mock.mock("POST", "/v1/chat/completions")
      .with_chunked_body(|w| {
        w.write_all(b"data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n")?;
        w.write_all(b"data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"id\":\"t1\",\"function\":{\"name\":\"test\",\"arguments\":\"{}\"}}]}}]}\n\n")?;
        w.write_all(b"data: [DONE]\n\n")?;
        Ok(())
      })
    ```
  - Mock z.ai API (same as OpenAI)
  - **Test tool results** across all providers:
    ```rust
    #[test]
    fn test_tool_result_normalization_anthropic() {
      let raw = json!({"tool_use_id": "abc", "output": "result"});
      let result = ToolResult::from_anthropic(&raw).unwrap();
      assert_eq!(result.tool_use_id, "abc");
      assert_eq!(result.content, "result");
    }
    
    #[test]
    fn test_tool_result_normalization_openai() {
      let raw = json!({"tool_call_id": "xyz", "content": "result"});
      let result = ToolResult::from_openai(&raw).unwrap();
      assert_eq!(result.tool_use_id, "xyz");
      assert_eq!(result.content, "result");
    }
    ```
  - Test streaming, error handling (401, 429, 500)
  - Test first-run setup flow
  - Test z.ai timeout fallback
  - Refs: client.rs tests
  - Commit: YES (`test: add provider integration tests`)

- [ ] **T13. Manual QA**
  - **Executable Scenarios**:
    ```bash
    # 1. Fresh install (no config)
    rm ~/.limit/config.toml
    cargo run --package limit-cli
    # Assert: Setup wizard appears
    
    # 2. Anthropic with real key
    echo '{"role":"user","content":"Say hello"}' | cargo run --package limit-cli
    # Assert: Response contains non-empty text
    
    # 3. OpenAI with real key
    # Update config to use openai provider
    echo '{"role":"user","content":"Say hello"}' | cargo run --package limit-cli
    # Assert: Response contains non-empty text
    
    # 4. z.ai timeout simulation
    # Send request that takes >30s (large tool call)
    # Assert: Warning logged, fallback to non-streaming
    
    # 5. Session-provider mismatch
    # Create session with anthropic, remove from config, restart
    # Assert: Prompt to choose new provider
    ```
  - Evidence: `.sisyphus/evidence/final-qa/`
  - Commit: NO

- [ ] **T14. Backward compat**
  - **Executable Scenario**:
    ```bash
    # 1. Setup old config
    cat > ~/.limit/config.toml <<EOF
    api_key = "sk-ant-test"
    model = "claude-3-5-sonnet"
    max_tokens = 4096
    timeout = 60
    EOF
    
    # 2. Run CLI
    cargo run --package limit-cli <<< "Hello"
    # Assert: Works without error
    
    # 3. Verify migration
    grep -q "\\[providers.anthropic\\]" ~/.limit/config.toml || exit 1
    grep -q "provider_type = \"anthropic\"" ~/.limit/config.toml || exit 1
    
    # 4. Verify backup
    test -f ~/.limit/config.toml.bak || exit 1
    ```
  - Commit: NO

- [ ] **T15. Docs**
  - Update README with multi-provider setup
  - Document first-run wizard
  - Document provider-specific limitations:
    ```markdown
    ## Provider Limitations
    
    ### z.ai (Experimental)
    - ⚠️ 30-second streaming timeout on large tool calls
    - Auto-fallback to non-streaming mode on timeout
    - Not recommended for tool-heavy workflows
    - `reasoning_content` field not exposed (provider-specific)
    ```
  - Example configs:
    ```toml
    # Single provider (Anthropic)
    [providers.claude]
    api_key = "sk-ant-..."
    model = "claude-opus-4-6"  # optional override
    
    # Multiple providers
    [providers.anthropic]
    api_key = "sk-ant-..."
    provider_type = "anthropic"
    
    [providers.gpt]
    api_key = "sk-..."
    provider_type = "openai"
    
    default = "anthropic"  # optional
    
    # z.ai (experimental)
    [providers.zai]
    api_key = "..."
    provider_type = "zai"
    base_url = "https://api.z.ai/api/paas/v4/"  # optional override
    ```
  - Commit: YES (`docs: update README with multi-provider setup`)

---

## Final Verification

- [ ] **F1. Plan compliance** (oracle) - All Must Have present, no Must NOT Have
- [ ] **F2. Code quality** - cargo test + clippy + fmt pass
- [ ] **F3. Manual QA** - All providers work with real keys
- [ ] **F4. Scope fidelity** - No scope creep, backward compat maintained

---

## Success

```bash
cargo test --workspace     # All pass
cargo clippy -- -D warnings # No warnings
cargo fmt --check           # Formatted
```

- [ ] All Must Have present
- [ ] No Must NOT Have
- [ ] Tests pass
- [ ] First-run wizard works
- [ ] Migration works
- [ ] Anthropic works (claude-sonnet-4-6)
- [ ] OpenAI works (gpt-5-mini)
- [ ] z.ai works (glm-5, experimental)
- [ ] Tool calling (def + result) works across all
- [ ] Streaming works
- [ ] z.ai timeout fallback works
- [ ] Non-interactive mode errors gracefully
- [ ] Session-provider mismatch prompts user
- [ ] Clear errors

---

## Provider Research Notes (2026-03-07)

### Anthropic API Reference
- **Docs**: https://docs.anthropic.com/en/api/messages
- **Endpoint**: `POST /v1/messages`
- **Auth**: `x-api-key` header + `anthropic-version: 2023-06-01`
- **Streaming**: SSE events (message_start, content_block_start, content_block_delta, content_block_stop, message_delta, message_stop)
- **Tool Calling**: `tool_use` content blocks, `tool_result` in messages
- **Tool Result Format**: `{"tool_use_id": "...", "output": ...}`
- **Errors**: 400 (invalid_request), 401 (auth), 403 (permission), 429 (rate_limit), 500 (api_error), 529 (overloaded)

### OpenAI API Reference
- **Docs**: https://platform.openai.com/docs/api-reference/chat
- **Endpoint**: `POST /v1/chat/completions`
- **Auth**: `Authorization: Bearer {key}`
- **Streaming**: SSE with `data:` prefix, `data: [DONE]` terminator
- **Tool Calling**: `tools` array in request, `tool_calls` array in response
- **Tool Result Format**: `{"tool_call_id": "...", "content": ...}`
- **Errors**: 400 (bad_request), 401 (auth), 403 (permission), 429 (rate_limit), 500 (internal), 503 (unavailable)

### z.ai API Reference
- **Compatibility**: OpenAI-compatible (CONFIRM via actual testing in T7)
- **Endpoint**: `https://api.z.ai/api/paas/v4/`
- **Auth**: `Authorization: Bearer {key}`
- **Models**: glm-5 (flagship), glm-4.7-flash (FREE), glm-4.7-flashx (cheap)
- **Gotchas**:
  - 30-second idle timeout on streaming (GitHub issue #12949)
  - `reasoning_content` field for thinking mode (provider-specific, not exposed)
  - Alternative endpoint: `open.bigmodel.cn` if timeout issues
  - Tool arguments batched (not streamed incrementally)
  - Tool result format: Same as OpenAI (`tool_call_id`, `content`)

---

## Decisions Applied (v4)

1. ✅ ProviderType: Anthropic, OpenAI, Zai
2. ✅ Default models: claude-sonnet-4-6, gpt-5-mini, glm-5
3. ✅ Research tasks before each provider implementation
4. ✅ z.ai is OpenAI-compatible (reuse OpenAI logic)
5. ✅ Document provider-specific gotchas in code comments
6. ✅ **Tool RESULT normalization** (T3.5) - CRITICAL from Metis
7. ✅ **z.ai experimental** with auto fallback on 30s timeout
8. ✅ **Non-interactive mode**: Error with instructions
9. ✅ **Session-provider mismatch**: Prompt user to choose new provider
10. ✅ **Migration failure modes**: Handle missing api_key, corrupted config, permission errors
11. ✅ **Dependencies explicit**: W1 → W2 → W3 → W4

---

## Metis Review Applied (v4)

**CRITICAL FIXES**:
1. Added T3.5 for tool RESULT normalization (not just definitions)
2. z.ai marked experimental with auto fallback on 30s timeout
3. Non-interactive mode detection in T9 (error with instructions)
4. Session-provider mismatch handling in T11 (prompt user)
5. Migration failure modes in T10 (atomic write, backup, validation)

**DEPENDENCIES CLARIFIED**:
- W1 (no deps) → W2 (needs T1, T3, T3.5) → W3 (needs T2 + W2) → W4 (needs W3)
- Critical path: T1/T2/T3 → T3.5/T4 → T5/T6/T7 → T8/T9/T10 → T11 → T12/T13/T14/T15

**RISKS MITIGATED**:
1. Tool result format mismatch → T3.5 added
2. z.ai compatibility assumptions → T7 requires actual API testing, not just doc reading
3. Migration data loss → Atomic write + backup + validation in T10
4. Breaking existing tests → T12 includes explicit test migration plan
5. Non-interactive wizard hang → T9 detects tty and exits with instructions
6. Session-provider coupling → T11 handles mismatch with user prompt

**GUARDRAILS ADDED**:
1. "One provider per session" enforced at AgentBridge::new()
2. Migration checklist in T11 for replacing AnthropicClient
3. Provider selection: `default` key > first alphabetical > error
4. Wizard triggers: no config OR empty providers AND interactive mode AND not --help/--version

**SCOPE CREEP PREVENTED**:
- Must NOT: Multi-model per provider config
- Must NOT: Expose provider-specific features (reasoning_content)
- Must NOT: Dynamic provider switching
- Must NOT: Config hot reload
- Must NOT: Non-streaming mode in v1 (except z.ai fallback)

**EDGE CASES HANDLED**:
1. Empty response from provider → Check in agent_bridge.rs loop
2. Tool call with empty arguments → Valid, executor handles
3. Provider error mid-stream → Error yielded, partial content discarded
4. Concurrent config edits → Config loaded once at startup
5. Duplicate tool names → Validate at registry startup
6. Very long tool arguments → No explicit limit (accept provider limit)
7. z.ai timeout during tool calling → Auto fallback to non-streaming

**ACCEPTANCE CRITERIA IMPROVED**:
- All "user confirms" replaced with shell commands + assertions
- T13 (Manual QA) has executable bash scripts
- T14 (Backward compat) has executable bash scripts
- Error cases added to all test coverage
