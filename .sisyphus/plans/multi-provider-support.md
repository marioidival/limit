# Multi-Provider Support (base_url)

## TL;DR

> **Quick Summary**: Add `base_url` config option to allow custom Anthropic-compatible API endpoints
> 
> **Deliverables**:
> - Config field `base_url: Option<String>`
> - Client uses config values instead of hardcoded
> - Tests for custom URL behavior
> 
> **Estimated Effort**: Quick
> **Parallel Execution**: YES - 2 waves
> **Critical Path**: Task 1 → Task 2/3 → Task 4 → Task 5

---

## Context

### Original Request
Allow limit to accept custom base URLs for Anthropic-like API providers.

### Interview Summary
- **Config format**: Single provider with `base_url` field
- **Providers**: User specifies full URL (e.g., `https://api.z.ai/api/anthropic`)
- **Headers**: Standard Anthropic headers (no customization)
- **Tests**: After implementation

### Metis Review
**Identified Gaps** (addressed):
- URL format: Full URL including path (user example: `/api/anthropic`)
- Timeout from config: Currently ignored — including in scope
- Validation: Simple `url::Url::parse()` check

---

## Work Objectives

### Core Objective
Enable users to configure custom API endpoints while maintaining Anthropic API compatibility.

### Concrete Deliverables
- `base_url: Option<String>` field in Config
- `AnthropicClient` uses config.base_url, config.model, config.max_tokens
- Agent bridge passes all config values to client

### Definition of Done
- [ ] `cargo test --workspace` passes
- [ ] Custom base_url works in config.toml
- [ ] Default (no base_url) still uses Anthropic API

### Must Have
- base_url config field with serde default
- Client uses all config values
- Backward compatible (existing configs work)

### Must NOT Have (Guardrails)
- Custom header support
- Multi-provider list
- Provider detection logic
- Over-engineered URL validation

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (49 tests, mockito)
- **Automated tests**: Tests after
- **Framework**: cargo test

### QA Policy
Each task includes agent-executed QA via cargo test and curl verification.

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — config + client changes):
├── Task 1: Add base_url to Config [quick]
├── Task 2: Update AnthropicClient for base_url [quick]
└── Task 3: Fix build_request_body to use config values [quick]

Wave 2 (After Wave 1 — integration):
├── Task 4: Update agent_bridge to pass config [quick]
└── Task 5: Add tests for base_url support [quick]

Critical Path: T1 → T2/T3 → T4 → T5
```

---

## TODOs

- [x] 1. Add base_url to Config struct

  **What to do**:
  - Add `base_url: Option<String>` field with `#[serde(default)]`
  - Add `fn default_base_url() -> Option<String> { None }`
  - Update `Config::default()` to include `base_url: None`

  **Must NOT do**: env var support, over-validation

  **Recommended Agent Profile**:
  - **Category**: `quick`

  **Parallelization**:
  - **Parallel Group**: Wave 1 (with T2, T3)
  - **Blocks**: T4

  **References**:
  - `limit-llm/src/config.rs:6-15` - Config struct
  - `limit-llm/src/config.rs:45-54` - Default impl

  **QA Scenarios**:
  ```
  Scenario: Config parses base_url
    Tool: Bash (cargo test)
    Steps: `cargo test --package limit-llm --lib config`
    Expected: All tests pass
  ```

  **Commit**: NO

- [x] 2. Update AnthropicClient for base_url

  **What to do**:
  - Modify `new()` to accept `Option<&str>` for base_url + timeout
  - Default: `https://api.anthropic.com/v1/messages`

  **Recommended Agent Profile**:
  - **Category**: `quick`

  **Parallelization**:
  - **Parallel Group**: Wave 1 (with T1, T3)
  - **Blocks**: T4

  **References**:
  - `limit-llm/src/client.rs:10-14` - struct
  - `limit-llm/src/client.rs:41-54` - new()

  **QA Scenarios**:
  ```
  Scenario: Client accepts base_url
    Tool: Bash
    Steps: `cargo test --package limit-llm --lib client`
    Expected: All tests pass
  ```

  **Commit**: NO

- [x] 3. Fix build_request_body to use config values

  **What to do**:
  - Add `model: &str`, `max_tokens: u32` params
  - Replace hardcoded values at lines 139-140
  - Update call site

  **Recommended Agent Profile**:
  - **Category**: `quick`

  **Parallelization**:
  - **Parallel Group**: Wave 1 (with T1, T2)
  - **Blocks**: T4

  **References**:
  - `limit-llm/src/client.rs:137-151` - function
  - `limit-llm/src/client.rs:68` - call site

  **QA Scenarios**:
  ```
  Scenario: Request uses config values
    Tool: Bash
    Steps: Add unit test, verify JSON
    Expected: Pass
  ```

  **Commit**: NO

- [x] 4. Update agent_bridge to pass config

  **What to do**:
  - Pass base_url, timeout, model, max_tokens from config
  - Run `lsp_find_references` on `AnthropicClient::new` first

  **Recommended Agent Profile**:
  - **Category**: `quick`

  **Parallelization**:
  - **Blocked By**: T1, T2, T3
  - **Blocks**: T5

  **References**:
  - `limit-cli/src/agent_bridge.rs:61` - client instantiation

  **QA Scenarios**:
  ```
  Scenario: CLI uses config
    Tool: Bash
    Steps: `cargo test --package limit-cli`
    Expected: All tests pass
  ```

  **Commit**: NO

- [x] 5. Add tests + update README

  **What to do**:
  - Test config with/without base_url
  - Update README with base_url example

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`effective-software-testing`]

  **Parallelization**:
  - **Blocked By**: T4

  **References**:
  - `limit-llm/src/config.rs:61-119` - test patterns
  - `README.md:36-43` - config section

  **QA Scenarios**:
  ```
  Scenario: All tests pass
    Tool: Bash
    Steps: `cargo test --workspace`
    Expected: 0 failures
    Evidence: .sisyphus/evidence/task-5-all-tests.txt
  ```

  **Commit**: YES
  - Message: `feat(llm): add base_url config for custom API endpoints`
  - Pre-commit: `cargo test --workspace && cargo clippy --workspace`

---

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Verify: base_url field exists, client uses it, tests pass. Check .sisyphus/evidence/.

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo test --workspace` + `cargo clippy`. Check for hardcoded URLs.

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Test with custom base_url in config.toml, verify request goes to correct URL.

- [ ] F4. **Scope Fidelity Check** — `deep`
  Verify no scope creep: no custom headers, no multi-provider.

---

## Commit Strategy

- **Single commit**: `feat(llm): add base_url config for custom API endpoints`

---

## Success Criteria

### Verification Commands
```bash
cargo test --workspace           # All tests pass
cargo clippy --workspace         # No warnings
```

### Final Checklist
- [x] Config has base_url field
- [x] Client uses config values (no hardcoded model/max_tokens)
- [x] Backward compatible (configs without base_url work)
- [x] All tests pass
- [ ] Config has base_url field
- [ ] Client uses config values (no hardcoded model/max_tokens)
- [ ] Backward compatible (configs without base_url work)
- [ ] All tests pass
