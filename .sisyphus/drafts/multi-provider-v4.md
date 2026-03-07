# Draft: Multi-Provider Support v4

## Requirements (confirmed)
- Multi-provider support: Anthropic, OpenAI, z.ai (experimental)
- First-run setup wizard when no provider configured
- Provider sections in config.toml: `[providers.X]`
- Replace flat Config with MultiProviderConfig
- One provider per session (no multi-provider sessions)
- Silent migration from flat config (with failure mode handling)
- Tool calling (definitions + results) normalized across providers
- z.ai experimental mode with auto fallback on 30s timeout
- Non-interactive mode: Error with instructions
- Session-provider mismatch: Prompt user to choose new provider

## Technical Decisions (v4)

### ProviderType Enum
```rust
pub enum ProviderType { 
  Anthropic,  // Native Anthropic API
  OpenAI,     // Native OpenAI API
  Zai         // OpenAI-compatible (experimental)
}
```

### Default Models per Provider
| Provider | Default Model | Reasoning |
|----------|--------------|-----------|
| Anthropic | `claude-sonnet-4-6` | Best balance speed/intelligence, $3/$15 per 1M |
| OpenAI | `gpt-5-mini` | Cost-efficient coding, $0.25/$2 per 1M |
| z.ai | `glm-5` | Flagship model, $1/$3.2 per 1M (experimental) |

### API Compatibility Matrix
| Provider | Auth Header | Endpoint | Streaming Format | Tool Format | Tool Result Format |
|----------|-------------|----------|------------------|-------------|-------------------|
| Anthropic | `x-api-key` + `anthropic-version` | `/v1/messages` | SSE events | tool_use/tool_result | `{tool_use_id, output}` |
| OpenAI | `Authorization: Bearer` | `/v1/chat/completions` | SSE data: chunks | tools/tool_calls | `{tool_call_id, content}` |
| z.ai | `Authorization: Bearer` | `api.z.ai/api/paas/v4/` | Same as OpenAI | Same as OpenAI | Same as OpenAI |

### Architecture
- Trait: `LlmProvider` (name, default_model, is_experimental, send)
- Error: `ProviderError` (AuthFailed, RateLimited, InvalidResponse, NetworkError, ConfigError, NoProviderConfigured, ProviderSpecific, MigrationError, SessionError)
- Config: `MultiProviderConfig { providers: HashMap<String, ProviderConfig>, default: Option<String> }`
- Setup: `setup.rs` with dialoguer for interactive prompts + atty for tty detection
- Tool Result: `ToolResult { tool_use_id, content, is_error }` - provider-agnostic

## Metis Review Decisions

1. **Tool RESULT normalization**: Added T3.5 - parse tool results from Anthropic (`tool_use_id`, `output`) and OpenAI (`tool_call_id`, `content`) formats
2. **z.ai experimental**: Marked as experimental, auto fallback to non-streaming on 30s timeout
3. **Non-interactive mode**: Detect no tty, exit with error + instructions
4. **Session-provider mismatch**: Prompt user to choose new provider when session's provider removed
5. **Migration failure modes**: Handle missing api_key, corrupted config, permission errors with atomic write + backup
6. **Dependencies explicit**: W1 → W2 → W3 → W4, all dependencies marked in tasks

## Research Findings

### Anthropic (from docs.anthropic.com)
- Streaming: SSE with events (message_start, content_block_delta, message_stop)
- Tool calling: tool_use blocks with accumulated JSON (input_json_delta)
- Tool result format: `{"tool_use_id": "...", "output": ...}`
- Error codes: 400, 401, 403, 429, 500, 529
- Default model reasoning: Sonnet 4.6 best balance, Opus 4.6 for complex tasks, Haiku 4.5 for speed

### OpenAI (from platform.openai.com)
- Streaming: SSE with `data:` prefix, `[DONE]` terminator
- Tool calling: tools array, tool_calls in response, arguments as complete JSON string
- Tool result format: `{"tool_call_id": "...", "content": ...}`
- Error codes: 400, 401, 403, 429, 500, 503
- Default model reasoning: gpt-5-mini for cost-efficient coding, gpt-5.2 for complex tasks

### z.ai (from research)
- **CRITICAL**: OpenAI-compatible (reuse OpenAI logic!)
- Endpoint: `https://api.z.ai/api/paas/v4/`
- Models: glm-5 (flagship), glm-4.7-flash (FREE), glm-4.7-flashx (cheap)
- **GOTCHA**: 30-second idle timeout on streaming connections
- **GOTCHA**: `reasoning_content` field for thinking mode (not exposed - provider-specific)
- **GOTCHA**: Tool arguments batched (not streamed incrementally)
- **EXPERIMENTAL**: Auto fallback to non-streaming on timeout
- Workaround: Use `open.bigmodel.cn` if timeout issues

## Task Structure (v4)

### Wave 1: Foundation (dependencies: none)
- T1: LlmProvider trait + ProviderError + ProviderType
- T2: MultiProviderConfig types
- T3: Tool definition normalization
- **T3.5: Tool RESULT normalization** (NEW - from Metis)
- T4: ProviderConfig tests [depends: T2]

### Wave 2: Providers (depends: W1 complete)
- T5: Research Anthropic docs → implement AnthropicProvider [depends: T1, T3, T3.5]
- T6: Research OpenAI docs → implement OpenAiProvider [depends: T1, T3, T3.5]
- T7: Research z.ai docs → implement ZaiProvider (experimental) [depends: T1, T3, T3.5]

### Wave 3: Config + CLI (depends: W2 complete)
- T8: Config loader [depends: T2]
- **T9: First-run setup wizard (non-interactive detect)** [depends: T2]
- **T10: Config migration (failure modes)** [depends: T2]
- **T11: Update limit-cli (session-provider mismatch)** [depends: T5, T6, T7, T8]

### Wave 4: Verification (depends: W3 complete)
- **T12: Integration tests (tool results + test migration plan)** [depends: T5, T6, T7]
- T13: Manual QA (executable scenarios) [depends: T11]
- T14: Backward compat (executable scenarios) [depends: T10, T11]
- T15: Docs [depends: T11]

**Total**: 15 tasks + 4 final verification

## Critical Path
```
T1/T2/T3 (parallel) 
  → T3.5/T4 (parallel, needs T3/T2)
  → T5/T6/T7 (parallel, needs T1+T3+T3.5)
  → T8/T9/T10 (parallel, needs T2)
  → T11 (sequential, needs T5-7 + T8)
  → T12/T13/T14/T15 (parallel, needs T11)
```

## Open Questions (resolved)
1. ✅ ProviderType enum values: Anthropic, OpenAI, Zai
2. ✅ Default models: claude-sonnet-4-6, gpt-5-mini, glm-5
3. ✅ z.ai compatibility: OpenAI-compatible (reuse logic)
4. ✅ Research before implementation: YES, documented in tasks
5. ✅ z.ai status: Experimental with auto fallback
6. ✅ Tool timeout behavior: Auto fallback to non-streaming
7. ✅ Non-interactive mode: Error with instructions
8. ✅ Session-provider mismatch: Prompt user

## Scope Boundaries
- INCLUDE: Trait abstraction, 3 providers (z.ai experimental), setup wizard, migration, tool normalization (def+results)
- EXCLUDE: Multi-provider sessions, dynamic switching, CLI flags, provider-specific features (reasoning_content)
- MUST NOT: Multi-model per provider, expose provider-specific features, config hot reload, non-streaming mode (except z.ai fallback)

---

## Plan Version History

**v1**: Initial plan
**v2**: Added first-run wizard, replaced Config
**v3**: Added ProviderType.Zai, research tasks, default models
**v4**: **Metis review applied**
  - T3.5 (tool results normalization)
  - z.ai experimental with auto fallback
  - Non-interactive detection
  - Session-provider mismatch handling
  - Migration failure modes
  - Explicit dependencies
  - Test migration plan
  - Executable acceptance criteria
