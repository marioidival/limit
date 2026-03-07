# limit - Rust Code Agent MVP

## TL;DR

> **Quick Summary**: Build a Rust-based code agent with 4 independent crates: limit-llm (Anthropic API), limit-agent (runtime with Docker sandbox), limit-cli (REPL interface), limit-tui (terminal UI with Ratatui+VDOM).
>
> **Deliverables**:
> - `limit-llm` crate: Unified LLM API with Anthropic support, SQLite tracking, binary persistence
> - `limit-agent` crate: Agent runtime with conditional tool execution, Docker sandbox (optional), event streaming
> - `limit-cli` crate: REPL interface with File/Bash/Git tools, markdown rendering, persistent sessions
> - `limit-tui` crate: Terminal UI with Virtual DOM rendering, flexbox layout, chat/diff views
>
> **Estimated Effort**: XL (4 crates, 20+ features)
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: Wave 1 → Wave 2 → Wave 3 → Wave 4

---

## Context

### Original Request
User wants to build a code agent in Rust called "limit" as alternative to TypeScript-based agents. Reference architecture is Pi (https://github.com/badlogic/pi-mono). MVP consists of 4 crates: limit-llm, limit-agent, limit-cli, limit-tui.

### Interview Summary
**Key Discussions**:
- **limit-llm**: Anthropic only (MVP), hard-coded models, SQLite tracking, binary persistence, resume/compact hand-off
- **limit-cli**: REPL-style, File/Bash/Git/Code analysis tools, config file, markdown rendering, persistent sessions
- **limit-agent**: Conditional tool execution, Docker sandbox (optional), state persistence, event streaming, agent-decides error recovery
- **limit-tui**: Virtual DOM with Ratatui, flexbox layout, Unix-only, no mouse/split-views

**Research Findings**:
- Pi architecture successfully mapped
- Ratatui is mature Rust TUI library
- Anthropic Claude API supports tool calling

### Metis Review
**Identified Gaps** (addressed):
- **MVP Scope**: Confirmed 4 crates (user's explicit request), structured in waves
- **Docker Dependency**: Made optional with fallback to host execution
- **TUI Architecture**: Ratatui + custom VDOM layer on top
- **Config Location**: ~/.limit/config.toml (simple, not XDG)
- **Agent Recovery**: Decision tree with LLM fallback
- **Performance SLAs**: Tool <5s, LLM first token <2s, UI >30 FPS
- **Error Handling**: Fail fast with clear error messages
- **File Limits**: Max 50MB reads, reject binary files
- **Git Subset**: clone, status, diff, log, add, commit, push, pull only
- **Tool Timeout**: 60s default, per-tool override
- **Max Iterations**: 50 tool calls per session

---

## Work Objectives

### Core Objective
Build a production-ready code agent in Rust with 4 independent crates that can read/write files, execute bash commands, analyze code, and perform git operations through a REPL interface with TUI rendering.

### Concrete Deliverables
- `limit-llm` crate published to crates.io
- `limit-agent` crate published to crates.io
- `limit-cli` crate published to crates.io
- `limit-tui` crate published to crates.io
- Working CLI binary that can chat with Anthropic and execute tools
- Config file support (~/.limit/config.toml)
- Session persistence (.limit/session.db)
- Docker sandbox support (optional)

### Definition of Done
- [ ] All 4 crates compile with `cargo build --release`
- [ ] All unit tests pass with `cargo test --all`
- [ ] CLI binary runs: `cargo run --package limit-cli`
- [ ] Can read a file: `> read src/main.rs`
- [ ] Can execute bash: `> bash echo hello`
- [ ] Can git status: `> git status`
- [ ] Can chat with Anthropic: `> what is 2+2?`
- [ ] Session persists across restarts
- [ ] Config file loaded from ~/.limit/config.toml
- [ ] TUI renders chat messages and diffs
- [ ] Docker sandbox works (if Docker available)

### Must Have
- Anthropic API integration with streaming
- SQLite tracking for token/cost
- Binary context persistence
- File read/write/edit operations
- Bash execution with timeout
- Git operations (subset)
- Config file support
- Session persistence
- TUI with chat and diff views
- Event streaming (agent → TUI)
- Error recovery with decision tree

### Must NOT Have (Guardrails)
- ❌ NO OpenAI/Gemini support in MVP (Anthropic only)
- ❌ NO generic provider abstraction layer
- ❌ NO model auto-detection
- ❌ NO prompt caching
- ❌ NO response caching
- ❌ NO vim/emacs keybindings
- ❌ NO syntax highlighting (plain text)
- ❌ NO auto-completion for REPL
- ❌ NO full terminal emulator
- ❌ NO custom markdown parser (use termimad)
- ❌ NO multi-container orchestration
- ❌ NO podman/runc support (Docker only)
- ❌ NO persistent containers (create → run → destroy)
- ❌ NO agent self-modification
- ❌ NO Windows support (Unix-only TUI)
- ❌ NO mouse support
- ❌ NO tabs/windows in TUI
- ❌ NO theming/color schemes
- ❌ NO web UI
- ❌ NO Slack bot

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: NO (Rust project from scratch)
- **Automated tests**: TDD for core (limit-llm, limit-agent, limit-cli), QA-only for limit-tui
- **Framework**: `cargo test` (built-in Rust test framework)
- **TDD workflow**: Each core task follows RED (failing test) → GREEN (minimal impl) → REFACTOR

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Frontend/UI (TUI)**: Use interactive_bash (tmux) — Run binary, send keystrokes, validate output, screenshot
- **CLI/Backend**: Use Bash (cargo test, cargo run) — Build, run, assert exit code and output
- **API**: Use Bash (curl) — Test Anthropic API integration, assert response
- **Library/Module**: Use Bash (cargo test) — Run unit tests, assert pass/fail

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — foundation + scaffolding):
├── Task 1: Workspace setup + crate scaffolding [quick]
├── Task 2: Config schema + loading [quick]
├── Task 3: Error types + thiserror setup [quick]
├── Task 4: Event schema definition [quick]
├── Task 5: limit-llm types (Message, Tool, Response) [quick]
└── Task 6: limit-llm Anthropic client (streaming) [deep]

Wave 2 (After Wave 1 — core implementation):
├── Task 7: limit-llm SQLite tracking [deep]
├── Task 8: limit-llm binary persistence [deep]
├── Task 9: limit-llm model hand-off (resume/compact) [deep]
├── Task 10: limit-agent tool trait + registry [deep]
├── Task 11: limit-agent tool execution (conditional) [deep]
├── Task 12: limit-agent Docker sandbox (optional) [deep]
└── Task 13: limit-agent state management [deep]

Wave 3 (After Wave 2 — integration + CLI):
├── Task 14: limit-cli REPL interface [deep]
├── Task 15: limit-cli file tools (read/write/edit) [deep]
├── Task 16: limit-cli bash tool [quick]
├── Task 17: limit-cli git tools [deep]
├── Task 18: limit-cli code analysis tools (grep/ast-grep/LSP) [deep]
├── Task 19: limit-cli markdown rendering (termimad) [quick]
├── Task 20: limit-cli session persistence [deep]
└── Task 21: limit-agent ↔ limit-cli integration [deep]

Wave 4 (After Wave 3 — TUI + final integration):
├── Task 22: limit-tui Virtual DOM core [deep]
├── Task 23: limit-tui Ratatui integration [deep]
├── Task 24: limit-tui flexbox layout [deep]
├── Task 25: limit-tui chat view component [deep]
├── Task 26: limit-tui diff view component [deep]
├── Task 27: limit-tui progress indicators [quick]
├── Task 28: limit-tui interactive prompts [deep]
├── Task 29: limit-cli ↔ limit-tui integration [deep]
└── Task 30: End-to-end integration test [deep]

Critical Path: T1 → T6 → T10 → T14 → T21 → T29 → T30
Parallel Speedup: ~65% faster than sequential
Max Concurrent: 6 (Wave 2)
```

### Dependency Matrix

- **1**: — — 2-6, 1
- **2**: 1 — 14, 1
- **3**: 1 — All crates, 1
- **4**: 1 — 13, 21, 29, 1
- **5**: 1 — 6-9, 1
- **6**: 1, 5 — 7-9, 21, 2
- **7**: 1, 6 — 21, 3
- **8**: 1, 6 — 20, 4
- **9**: 1, 6, 8 — 21, 5
- **10**: 1, 3 — 11-13, 6
- **11**: 1, 10 — 21, 7
- **12**: 1, 10 — 21, 8
- **13**: 1, 10, 11 — 21, 9
- **14**: 1, 2, 3 — 15-20, 10
- **15**: 14 — 21, 11
- **16**: 14 — 21, 12
- **17**: 14 — 21, 13
- **18**: 14 — 21, 14
- **19**: 14 — 21, 15
- **20**: 8, 14 — 21, 16
- **21**: 6-14, 15-20 — 29, 17
- **22**: 1 — 23-28, 18
- **23**: 22 — 29, 19
- **24**: 22 — 29, 20
- **25**: 22, 24 — 29, 21
- **26**: 22, 24 — 29, 22
- **27**: 22, 24 — 29, 23
- **28**: 22, 24 — 29, 24
- **29**: 4, 21, 23-28 — 30, 25
- **30**: 29 — FINAL, 26

### Agent Dispatch Summary

- **1**: **6** — T1-T4 → `quick`, T5 → `quick`, T6 → `deep`
- **2**: **7** — T7 → `deep`, T8 → `deep`, T9 → `deep`, T10 → `deep`, T11 → `deep`, T12 → `deep`, T13 → `deep`
- **3**: **8** — T14 → `deep`, T15 → `deep`, T16 → `quick`, T17 → `deep`, T18 → `deep`, T19 → `quick`, T20 → `deep`, T21 → `deep`
- **4**: **9** — T22 → `deep`, T23 → `deep`, T24 → `deep`, T25 → `deep`, T26 → `deep`, T27 → `quick`, T28 → `deep`, T29 → `deep`, T30 → `deep`

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task MUST have: Recommended Agent Profile + Parallelization info + QA Scenarios.

- [x] 1. Workspace Setup + Crate Scaffolding

  **What to do**:
  - Create Cargo workspace with 4 crates: limit-llm, limit-agent, limit-cli, limit-tui
  - Add Cargo.toml with workspace members
  - Create basic lib.rs for limit-llm, limit-agent, limit-tui
  - Create basic main.rs for limit-cli
  - Add .gitignore (target/, .limit/, *.db, *.bin)
  - Add README.md with project overview

  **Must NOT do**:
  - Don't add dependencies yet (separate task)
  - Don't create source files beyond lib.rs/main.rs

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Scaffolding task, straightforward setup
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: NO (foundation task)
  - **Parallel Group**: Wave 1
  - **Blocks**: All tasks
  - **Blocked By**: None

  **References**:
  - Cargo workspace docs: https://doc.rust-lang.org/cargo/reference/workspaces.html
  - Pi structure: https://github.com/badlogic/pi-mono/tree/main/packages

  **Acceptance Criteria**:
  - [ ] Cargo.toml exists with workspace members
  - [ ] `cargo build` succeeds (may have warnings)
  - [ ] `cargo test --all` succeeds (0 tests)
  - [ ] .gitignore exists
  - [ ] README.md exists

  **QA Scenarios**:
  ```
  Scenario: Workspace builds successfully
    Tool: Bash
    Preconditions: Fresh clone
    Steps:
      1. cargo build --workspace
      2. Verify exit code 0
    Expected Result: Build completes without errors
    Evidence: .sisyphus/evidence/task-01-workspace-build.txt

  Scenario: All crates are workspace members
    Tool: Bash
    Preconditions: Cargo.toml exists
    Steps:
      1. cargo metadata --format-version=1 | jq '.workspace_members | length'
      2. Verify output is "4"
    Expected Result: 4 workspace members found
    Evidence: .sisyphus/evidence/task-01-workspace-members.txt
  ```

  **Commit**: YES
  - Message: `chore: initial workspace setup with 4 crates`
  - Files: Cargo.toml, limit-llm/Cargo.toml, limit-agent/Cargo.toml, limit-cli/Cargo.toml, limit-tui/Cargo.toml, .gitignore, README.md

- [x] 2. Config Schema + Loading

  **What to do**:
  - Add dependencies: `toml`, `serde`, `dirs` to limit-llm
  - Create `limit-llm/src/config.rs` with Config struct
  - Fields: api_key (String), model (String), max_tokens (usize), timeout (u64)
  - Implement load() to read from ~/.limit/config.toml
  - Add default values if file missing
  - Document config format in README

  **Must NOT do**:
  - Don't support YAML/JSON (TOML only)
  - Don't use XDG spec (~/.limit only)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4, 5)
  - **Blocks**: Task 14 (CLI needs config)
  - **Blocked By**: Task 1 (workspace setup)

  **References**:
  - TOML crate: https://docs.rs/toml/latest/toml/
  - dirs crate: https://docs.rs/dirs/latest/dirs/

  **Acceptance Criteria**:
  - [ ] Config struct defined
  - [ ] load() reads from ~/.limit/config.toml
  - [ ] Default values provided
  - [ ] Unit test: parse valid config
  - [ ] Unit test: handle missing file

  **QA Scenarios**:
  ```
  Scenario: Config file loads correctly
    Tool: Bash
    Preconditions: ~/.limit/config.toml exists with api_key = "test-key"
    Steps:
      1. cargo test test_config_load --package limit-llm
      2. Verify test passes
    Expected Result: Test passes, config loaded
    Evidence: .sisyphus/evidence/task-02-config-load.txt
  ```

  **Commit**: NO (groups with Task 6)

- [x] 3. Error Types + thiserror Setup

  **What to do**:
  - Add dependency: `thiserror` to all crates
  - Create `limit-llm/src/error.rs` with LlmError enum
  - Variants: ApiError(String), NetworkError(String), ConfigError(String), PersistenceError(String)
  - Create `limit-agent/src/error.rs` with AgentError enum
  - Variants: ToolError(String), StateError(String), SandboxError(String)
  - Create `limit-cli/src/error.rs` with CliError enum
  - Variants: IoError(String), ConfigError(String), AgentError(String)
  - Implement From<> traits for interop

  **Must NOT do**:
  - Don't use anyhow (use thiserror for explicit error types)
  - Don't add excessive error variants

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: All crates
  - **Blocked By**: Task 1

  **References**:
  - thiserror crate: https://docs.rs/thiserror/latest/thiserror/

  **Acceptance Criteria**:
  - [ ] LlmError enum defined
  - [ ] AgentError enum defined
  - [ ] CliError enum defined
  - [ ] From<> traits implemented
  - [ ] cargo build succeeds

  **QA Scenarios**:
  ```
  Scenario: Error types compile
    Tool: Bash
    Steps:
      1. cargo build --workspace
      2. Verify exit code 0
    Expected Result: Build succeeds
    Evidence: .sisyphus/evidence/task-03-error-types.txt
  ```

  **Commit**: NO (groups with Task 6)

- [x] 4. Event Schema Definition

  **What to do**:
  - Create `limit-agent/src/events.rs` with Event enum
  - Variants: Thinking, ToolCall{name, args}, ToolResult{output}, FileChange{path, diff}, Error{message}, Done
  - Add serde derives for JSON serialization
  - Add version field (u32) for future compatibility
  - Document schema in README

  **Must NOT do**:
  - Don't use complex nested structures
  - Don't skip version field

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 13, 21, 29
  - **Blocked By**: Task 1

  **References**:
  - serde docs: https://serde.rs/

  **Acceptance Criteria**:
  - [ ] Event enum defined
  - [ ] serde derives added
  - [ ] version field present
  - [ ] Unit test: serialize/deserialize event

  **QA Scenarios**:
  ```
  Scenario: Event schema serializes to JSON
    Tool: Bash
    Steps:
      1. cargo test test_event_serialization --package limit-agent
      2. Verify test passes
    Expected Result: Test passes, JSON valid
    Evidence: .sisyphus/evidence/task-04-event-schema.txt
  ```

  **Commit**: NO (groups with Task 10)

- [x] 5. limit-llm Types (Message, Tool, Response)

  **What to do**:
  - Create `limit-llm/src/types.rs`
  - Define Message struct: role (Role enum), content (String), tool_calls (Option<Vec<ToolCall>>)
  - Define Role enum: User, Assistant, System
  - Define ToolCall struct: id (String), name (String), arguments (serde_json::Value)
  - Define Tool struct: name (String), description (String), parameters (serde_json::Value)
  - Define Response struct: content (String), tool_calls (Option<Vec<ToolCall>>), usage (Usage)
  - Define Usage struct: input_tokens (u64), output_tokens (u64)
  - Add serde derives

  **Must NOT do**:
  - Don't include fields not in Anthropic API
  - Don't use String for role (use enum)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 6
  - **Blocked By**: Task 1

  **References**:
  - Anthropic API docs: https://docs.anthropic.com/claude/reference/messages_post

  **Acceptance Criteria**:
  - [ ] Message struct defined
  - [ ] Role enum defined
  - [ ] ToolCall struct defined
  - [ ] Tool struct defined
  - [ ] Response struct defined
  - [ ] Usage struct defined
  - [ ] Unit tests: serialize/deserialize all types

  **QA Scenarios**:
  ```
  Scenario: Types serialize correctly
    Tool: Bash
    Steps:
      1. cargo test test_types_serialization --package limit-llm
      2. Verify all tests pass
    Expected Result: Tests pass
    Evidence: .sisyphus/evidence/task-05-types.txt
  ```

  **Commit**: NO (groups with Task 6)

- [x] 6. limit-llm Anthropic Client (Streaming)

  **What to do**:
  - Add dependencies: `reqwest`, `tokio`, `serde_json`, `async-stream`
  - Create `limit-llm/src/client.rs` with AnthropicClient struct
  - Implement new(api_key: String) -> Self
  - Implement send(messages: Vec<Message>, tools: Vec<Tool>) -> impl Stream<Result<ResponseChunk, LlmError>>
  - Use Anthropic Messages API with streaming
  - Handle HTTP errors (429 rate limit, 500 server error)
  - Implement retry with exponential backoff (3 attempts: 1s, 2s, 4s)
  - Timeout: 30s connect, 300s read
  - Parse streaming chunks into ResponseChunk enum: ContentDelta(String), ToolCallDelta(ToolCall), Done(Usage)

  **Must NOT do**:
  - Don't add caching layer
  - Don't support batch requests
  - Don't implement request queuing

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (core dependency)
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 7-9, 21
  - **Blocked By**: Tasks 1, 5

  **References**:
  - Anthropic streaming: https://docs.anthropic.com/claude/reference/messages-streaming
  - reqwest streaming: https://docs.rs/reqwest/latest/reqwest/

  **Acceptance Criteria**:
  - [ ] AnthropicClient struct defined
  - [ ] send() returns stream
  - [ ] Handles streaming chunks correctly
  - [ ] Retry logic works (test with mock server)
  - [ ] Timeout enforced
  - [ ] Unit test: successful stream
  - [ ] Unit test: retry on 429
  - [ ] Unit test: timeout on slow response

  **QA Scenarios**:
  ```
  Scenario: Anthropic API streaming works
    Tool: Bash
    Preconditions: ANTHROPIC_API_KEY env var set
    Steps:
      1. cargo test test_anthropic_streaming --package limit-llm -- --ignored
      2. Verify test passes with real API call
    Expected Result: Stream receives tokens, completes successfully
    Evidence: .sisyphus/evidence/task-06-anthropic-streaming.txt

  Scenario: Retry on rate limit
    Tool: Bash
    Steps:
      1. cargo test test_retry_on_429 --package limit-llm
      2. Verify retry happens
    Expected Result: Retries 3 times with backoff
    Evidence: .sisyphus/evidence/task-06-retry-429.txt
  ```

  **Commit**: YES
  - Message: `feat(limit-llm): add Anthropic streaming client`
  - Files: limit-llm/src/client.rs, limit-llm/Cargo.toml
  - Pre-commit: `cargo test --package limit-llm`

- [x] 7. limit-llm SQLite Tracking

  **What to do**:
  - Add dependency: `rusqlite`
  - Create `limit-llm/src/tracking.rs` with TrackingDb struct
  - Initialize database: ~/.limit/tracking.db
  - Schema: requests(id, timestamp, model, input_tokens, output_tokens, cost, duration_ms)
  - Implement track_request(model, input_tokens, output_tokens, cost, duration)
  - Implement get_usage_stats(days: u32) -> UsageStats
  - UsageStats: total_requests, total_tokens, total_cost, avg_duration
  - Auto-create tables on first run
  - Handle concurrent access with SQLite locking

  **Must NOT do**:
  - Don't cache responses (only tracking)
  - Don't implement request queuing
  - Don't add compaction logic (defer to future)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 8, 9, 10)
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 1, 6

  **References**:
  - rusqlite: https://docs.rs/rusqlite/latest/rusqlite/

  **Acceptance Criteria**:
  - [ ] TrackingDb struct defined
  - [ ] track_request() works
  - [ ] get_usage_stats() calculates correctly
  - [ ] Database auto-created
  - [ ] Handles concurrent access
  - [ ] Unit test: track and retrieve
  - [ ] Unit test: concurrent access

  **QA Scenarios**:
  ```
  Scenario: Tracking persists to SQLite
    Tool: Bash
    Steps:
      1. cargo test test_tracking_persists --package limit-llm
      2. Verify database created at ~/.limit/tracking.db
    Expected Result: Database exists with tracked request
    Evidence: .sisyphus/evidence/task-07-tracking.txt
  ```

  **Commit**: NO (groups with Task 9)

- [x] 8. limit-llm Binary Persistence

  **What to do**:
  - Add dependency: `bincode`
  - Create `limit-llm/src/persistence.rs` with StatePersistence struct
  - Implement save(session_id: &str, messages: &[Message]) -> Result<(), LlmError>
  - Implement load(session_id: &str) -> Result<Vec<Message>, LlmError>
  - Store at: ~/.limit/sessions/{session_id}.bin
  - Use bincode for binary serialization
  - Add version field to handle future format changes
  - Handle corrupted files gracefully

  **Must NOT do**:
  - Don't use JSON (binary only for efficiency)
  - Don't compress (defer to future)
  - Don't implement encryption (out of scope)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 20
  - **Blocked By**: Tasks 1, 6

  **References**:
  - bincode: https://docs.rs/bincode/latest/bincode/

  **Acceptance Criteria**:
  - [ ] StatePersistence struct defined
  - [ ] save() works
  - [ ] load() works
  - [ ] Version field present
  - [ ] Handles corrupted files
  - [ ] Unit test: save and load
  - [ ] Unit test: corrupted file recovery

  **QA Scenarios**:
  ```
  Scenario: Binary persistence works
    Tool: Bash
    Steps:
      1. cargo test test_binary_persistence --package limit-llm
      2. Verify file created at ~/.limit/sessions/
    Expected Result: File exists and loads correctly
    Evidence: .sisyphus/evidence/task-08-binary-persistence.txt
  ```

  **Commit**: NO (groups with Task 9)

- [x] 9. limit-llm Model Hand-off (Resume/Compact)

  **What to do**:
  - Create `limit-llm/src/handoff.rs` with ModelHandoff struct
  - Implement compact_messages(messages: &[Message], target_tokens: usize) -> Vec<Message>
  - Strategy: Keep system message + last N messages, summarize middle if needed
  - Implement handoff_to_model(from_model: &str, to_model: &str, messages: &[Message]) -> Vec<Message>
  - Call compact_messages if token count exceeds target
  - Token counting: Use tiktoken-rs or similar for accurate counting
  - Trigger: When user requests model change via CLI

  **Must NOT do**:
  - Don't implement AI-based summarization (rule-based only for MVP)
  - Don't handoff mid-tool-call (wait for completion)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 1, 6, 8

  **References**:
  - tiktoken-rs: https://docs.rs/tiktoken-rs/latest/tiktoken_rs/

  **Acceptance Criteria**:
  - [ ] ModelHandoff struct defined
  - [ ] compact_messages() works
  - [ ] handoff_to_model() works
  - [ ] Token counting accurate within 5%
  - [ ] Unit test: compact preserves context
  - [ ] Unit test: handoff between models

  **QA Scenarios**:
  ```
  Scenario: Model handoff compacts context
    Tool: Bash
    Steps:
      1. cargo test test_handoff_compacts --package limit-llm
      2. Verify token count reduced
    Expected Result: Context compacted successfully
    Evidence: .sisyphus/evidence/task-09-handoff.txt
  ```

  **Commit**: YES
  - Message: `feat(limit-llm): add SQLite tracking, binary persistence, and model handoff`
  **Commit**: YES
  - Message: `feat(limit-llm): add SQLite tracking, binary persistence, and model handoff`
  - Files: limit-llm/src/tracking.rs, limit-llm/src/persistence.rs, limit-llm/src/handoff.rs
  - Pre-commit: `cargo test --package limit-llm`

- [x] 10. limit-agent Tool Trait + Registry

  **What to do**:
  - Create `limit-agent/src/tool.rs` with Tool trait
  - Define trait methods: name(&self) -> &str, execute(&self, args: Value) -> Result<Value, AgentError>
  - Create `limit-agent/src/registry.rs` with ToolRegistry struct
  - Implement register(tool: Box<dyn Tool>)
  - Implement get(name: &str) -> Option<&dyn Tool>
  - Implement list() -> Vec<&str>
  - Store tools in HashMap<String, Box<dyn Tool>>
  - Add example tool: EchoTool for testing

  **Must NOT do**:
  - Don't implement all tools yet (just registry)
  - Don't add tool validation logic

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 11-13
  - **Blocked By**: Tasks 1, 3

  **References**:
  - Rust trait objects: https://doc.rust-lang.org/book/ch17-02-trait-objects.html

  **Acceptance Criteria**:
  - [ ] Tool trait defined
  - [ ] ToolRegistry struct defined
  - [ ] register() works
  - [ ] get() works
  - [ ] list() works
  - [ ] EchoTool example works
  - [ ] Unit test: register and execute

  **QA Scenarios**:
  ```
  Scenario: Tool registry works
    Tool: Bash
    Steps:
      1. cargo test test_tool_registry --package limit-agent
      2. Verify test passes
    Expected Result: Registry registers and retrieves tools
    Evidence: .sisyphus/evidence/task-10-tool-registry.txt
  ```

  **Commit**: NO (groups with Task 12)

- [x] 11. limit-agent Tool Execution (Conditional)

  **What to do**:
  - Create `limit-agent/src/executor.rs` with ToolExecutor struct
  - Implement execute_tools(calls: Vec<ToolCall>) -> Vec<Result<ToolResult, AgentError>>
  - Conditional execution logic:
    - Analyze dependencies between tool calls
    - Independent tools → execute in parallel (tokio::join!)
    - Dependent tools → execute sequentially
  - Timeout per tool: 60s default, configurable per tool
  - Max concurrent tools: 5 (configurable)
  - Capture tool output and errors

  **Must NOT do**:
  - Don't implement agent reasoning (just execution)
  - Don't allow unlimited parallelism

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 1, 10

  **References**:
  - tokio::join: https://docs.rs/tokio/latest/tokio/macro.join.html

  **Acceptance Criteria**:
  - [ ] ToolExecutor struct defined
  - [ ] execute_tools() works
  - [ ] Parallel execution for independent tools
  - [ ] Sequential for dependent tools
  - [ ] Timeout enforced
  - [ ] Unit test: parallel execution
  - [ ] Unit test: timeout

  **QA Scenarios**:
  ```
  Scenario: Parallel tool execution
    Tool: Bash
    Steps:
      1. cargo test test_parallel_execution --package limit-agent
      2. Verify tools run in parallel
    Expected Result: Execution time < sum of individual times
    Evidence: .sisyphus/evidence/task-11-parallel-execution.txt
  ```

  **Commit**: NO (groups with Task 12)

- [x] 12. limit-agent Docker Sandbox (Optional)

  **What to do**:
  - Add dependency: `bollard` (Docker SDK)
  - Create `limit-agent/src/sandbox.rs` with DockerSandbox struct
  - Implement check_docker_available() -> bool
  - Implement create_container(image: &str) -> Result<ContainerId, AgentError>
  - Implement execute_in_container(container: &str, cmd: &[String]) -> Result<String, AgentError>
  - Implement cleanup_container(container: &str)
  - Default image: limit-rust-sandbox:latest
  - Volume mount: project directory read-only
  - Network: disabled by default
  - Memory limit: 512MB
  - Timeout: 60s
  - If Docker not available: return error, caller decides fallback

  **Must NOT do**:
  - Don't require Docker (make optional)
  - Don't support podman/runc
  - Don't allow network access in containers

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 1, 10

  **References**:
  - bollard: https://docs.rs/bollard/latest/bollard/
  - Docker SDK: https://docs.docker.com/engine/api/

  **Acceptance Criteria**:
  - [ ] DockerSandbox struct defined
  - [ ] check_docker_available() works
  - [ ] create_container() works
  - [ ] execute_in_container() works
  - [ ] cleanup_container() works
  - [ ] Memory limit enforced
  - [ ] Network disabled
  - [ ] Unit test: Docker available check
  - [ ] Integration test: execute in container (requires Docker)

  **QA Scenarios**:
  ```
  Scenario: Docker sandbox works
    Tool: Bash
    Preconditions: Docker installed and running
    Steps:
      1. cargo test test_docker_sandbox --package limit-agent -- --ignored
      2. Verify container created and executed
    Expected Result: Command runs in container, output returned
    Evidence: .sisyphus/evidence/task-12-docker-sandbox.txt

  Scenario: Docker not available
    Tool: Bash
    Preconditions: Docker not installed
    Steps:
      1. cargo test test_docker_unavailable --package limit-agent
      2. Verify returns false
    Expected Result: check_docker_available() returns false
    Evidence: .sisyphus/evidence/task-12-docker-unavailable.txt
  ```

  **Commit**: YES
  - Message: `feat(limit-agent): add tool registry, executor, and Docker sandbox`
  - Files: limit-agent/src/tool.rs, limit-agent/src/registry.rs, limit-agent/src/executor.rs, limit-agent/src/sandbox.rs
  - Pre-commit: `cargo test --package limit-agent`

- [x] 13. limit-agent State Management

  **What to do**:
  - Create `limit-agent/src/state.rs` with AgentState struct
  - Fields: messages (Vec<Message>), tool_results (HashMap<String, Value>), decisions (Vec<Decision>), todos (Vec<Todo>), iteration (u32)
  - Decision struct: timestamp, action, reason
  - Todo struct: id, content, status (Pending/InProgress/Done)
  - Implement save_state(session_id: &str) -> Result<(), AgentError>
  - Implement load_state(session_id: &str) -> Result<AgentState, AgentError>
  - Store at: .limit/agent-state.bin (use bincode)
  - Implement max_iterations: 50 tool calls per session
  - Implement loop_detection: reject same tool+args 3 times in a row

  **Must NOT do**:
  - Don't track file changes (out of scope)
  - Don't persist across sessions without explicit save

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 1, 10, 11

  **References**:
  - bincode: https://docs.rs/bincode/latest/bincode/

  **Acceptance Criteria**:
  - [ ] AgentState struct defined
  - [ ] save_state() works
  - [ ] load_state() works
  - [ ] Max iterations enforced
  - [ ] Loop detection works
  - [ ] Unit test: save and load
  - [ ] Unit test: max iterations
  - [ ] Unit test: loop detection

  **QA Scenarios**:
  ```
  Scenario: State persists correctly
    Tool: Bash
    Steps:
      1. cargo test test_state_persists --package limit-agent
      2. Verify state saved to .limit/agent-state.bin
    Expected Result: State saved and loaded correctly
    Evidence: .sisyphus/evidence/task-13-state.txt
  ```

  **Commit**: YES
  - Message: `feat(limit-agent): add state management with persistence and loop detection`
  - Files: limit-agent/src/state.rs
  - Pre-commit: `cargo test --package limit-agent`

- [x] 14. limit-cli REPL Interface

  **What to do**:
  - Add dependencies: `rustyline`, `crossterm`
  - Create `limit-cli/src/repl.rs` with Repl struct
  - Implement new() -> Self
  - Implement run(&mut self) -> Result<(), CliError>
  - Read input with rustyline (history support)
  - Parse commands: chat messages (no prefix), /exit, /clear, /help
  - Display prompt: "limit> "
  - Handle Ctrl+C gracefully (no crash)
  - Event loop: read → process → render

  **Must NOT do**:
  - Don't implement all commands yet (just basics)
  - Don't add auto-completion
  - Don't add vim/emacs keybindings

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (core CLI)
  - **Parallel Group**: Wave 3
  - **Blocks**: Tasks 15-21
  - **Blocked By**: Tasks 1, 2, 3

  **References**:
  - rustyline: https://docs.rs/rustyline/latest/rustyline/
  - crossterm: https://docs.rs/crossterm/latest/crossterm/

  **Acceptance Criteria**:
  - [ ] Repl struct defined
  - [ ] run() loop works
  - [ ] Reads input correctly
  - [ ] Handles /exit, /clear, /help
  - [ ] Handles Ctrl+C
  - [ ] Unit test: basic command parsing
  - [ ] Integration test: REPL starts and exits

  **QA Scenarios**:
  ```
  Scenario: REPL starts and accepts input
    Tool: interactive_bash
    Steps:
      1. cargo run --package limit-cli
      2. Send: "hello"
      3. Send: "/exit"
    Expected Result: REPL starts, accepts input, exits cleanly
    Evidence: .sisyphus/evidence/task-14-repl-starts.png
  ```

  **Commit**: NO (groups with Task 21)

- [x] 15. limit-cli File Tools (Read/Write/Edit)

  **What to do**:
  - Create `limit-cli/src/tools/file.rs`
  - Implement FileReadTool: read file path, max 50MB, reject binary files
  - Implement FileWriteTool: write content to path
  - Implement FileEditTool: use diff-based editing (similar to edit tool)
  - Register tools in ToolRegistry
  - Error handling: file not found, permission denied, too large, binary file

  **Must NOT do**:
  - Don't implement file search (separate task)
  - Don't support >50MB files
  - Don't implement auto-backup

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 16, 17, 18, 19, 20)
  - **Blocks**: Task 21
  - **Blocked By**: Task 14

  **References**:
  - std::fs: https://doc.rust-lang.org/std/fs/
  - similar (diff): https://docs.rs/similar/latest/similar/

  **Acceptance Criteria**:
  - [ ] FileReadTool defined
  - [ ] FileWriteTool defined
  - [ ] FileEditTool defined
  - [ ] Max 50MB enforced
  - [ ] Binary file rejection
  - [ ] Unit tests: read, write, edit
  - [ ] Integration test: tool registered

  **QA Scenarios**:
  ```
  Scenario: File read tool works
    Tool: Bash
    Preconditions: Test file exists with content "hello world"
    Steps:
      1. cargo test test_file_read_tool --package limit-cli
      2. Verify content matches
    Expected Result: File content returned
    Evidence: .sisyphus/evidence/task-15-file-read.txt

  Scenario: File too large rejected
    Tool: Bash
    Preconditions: Test file >50MB exists
    Steps:
      1. cargo test test_file_too_large --package limit-cli
      2. Verify error returned
    Expected Result: Error: file too large
    Evidence: .sisyphus/evidence/task-15-file-toolarge.txt
  ```

  **Commit**: NO (groups with Task 21)

- [x] 16. limit-cli Bash Tool

  **What to do**:
  - Create `limit-cli/src/tools/bash.rs`
  - Implement BashTool: execute command with args
  - Timeout: 60s default
  - Capture stdout, stderr, exit code
  - Working directory: current project directory
  - Environment: inherit from parent process
  - Security: block dangerous commands (rm -rf /, :(){ :|:& };:, etc.)
  - Register tool in ToolRegistry

  **Must NOT do**:
  - Don't allow unlimited execution time
  - Don't run as root/sudo
  - Don't execute without timeout

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 21
  - **Blocked By**: Task 14

  **References**:
  - std::process::Command: https://doc.rust-lang.org/std/process/struct.Command.html

  **Acceptance Criteria**:
  - [ ] BashTool defined
  - [ ] Timeout enforced
  - [ ] Captures stdout/stderr/exit code
  - [ ] Blocks dangerous commands
  - [ ] Unit test: execute command
  - [ ] Unit test: timeout
  - [ ] Integration test: tool registered

  **QA Scenarios**:
  ```
  Scenario: Bash tool executes command
    Tool: Bash
    Steps:
      1. cargo test test_bash_tool --package limit-cli
      2. Verify "echo hello" returns "hello"
    Expected Result: Command output captured
    Evidence: .sisyphus/evidence/task-16-bash-tool.txt
  ```

  **Commit**: NO (groups with Task 21)

- [x] 17. limit-cli Git Tools

  **What to do**:
  - Create `limit-cli/src/tools/git.rs`
  - Implement GitStatusTool: git status
  - Implement GitDiffTool: git diff
  - Implement GitLogTool: git log -n 10
  - Implement GitAddTool: git add <files>
  - Implement GitCommitTool: git commit -m <message>
  - Implement GitPushTool: git push
  - Implement GitPullTool: git pull
  - Implement GitCloneTool: git clone <url>
  - All tools wrap git CLI (require git in PATH)
  - Error if git not found
  - Register all tools in ToolRegistry

  **Must NOT do**:
  - Don't implement all git commands (just these 8)
  - Don't use libgit2 (wrap CLI)
  - Don't support git <2.0

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`git-master`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 21
  - **Blocked By**: Task 14

  **References**:
  - git CLI: https://git-scm.com/docs

  **Acceptance Criteria**:
  - [ ] All 8 git tools defined
  - [ ] Error if git not in PATH
  - [ ] Unit test: each tool works
  - [ ] Integration test: all tools registered

  **QA Scenarios**:
  ```
  Scenario: Git status tool works
    Tool: Bash
    Preconditions: In git repository
    Steps:
      1. cargo test test_git_status --package limit-cli
      2. Verify status returned
    Expected Result: Git status output
    Evidence: .sisyphus/evidence/task-17-git-tools.txt
  ```

  **Commit**: NO (groups with Task 21)

- [x] 18. limit-cli Code Analysis Tools (Grep/ast-grep/LSP)

  **What to do**:
  - Create `limit-cli/src/tools/analysis.rs`
  - Implement GrepTool: search files with regex pattern
  - Implement AstGrepTool: AST-aware code search (use ast-grep crate)
  - Implement LspTool: LSP integration (go-to-definition, find-references)
  - GrepTool: use grep crate, max results 1000, context lines 3
  - AstGrepTool: support Rust, TypeScript, Python (initial set)
  - LspTool: support rust-analyzer, typescript-language-server
  - Register all tools in ToolRegistry

  **Must NOT do**:
  - Don't implement all LSP features (just go-to-def, find-refs)
  - Don't support all languages (just Rust, TS, Python)
  - Don't run LSP server if not installed

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 21
  - **Blocked By**: Task 14

  **References**:
  - grep crate: https://docs.rs/grep/latest/grep/
  - ast-grep: https://ast-grep.github.io/guide/introduction.html

  **Acceptance Criteria**:
  - [ ] GrepTool defined
  - [ ] AstGrepTool defined
  - [ ] LspTool defined
  - [ ] Max results enforced
  - [ ] Unit test: each tool works
  - [ ] Integration test: all tools registered

  **QA Scenarios**:
  ```
  Scenario: Grep tool finds pattern
    Tool: Bash
    Steps:
      1. cargo test test_grep_tool --package limit-cli
      2. Verify pattern found
    Expected Result: Matches returned
    Evidence: .sisyphus/evidence/task-18-grep-tool.txt
  ```

  **Commit**: NO (groups with Task 21)

- [x] 19. limit-cli Markdown Rendering (termimad)

  **What to do**:
  - Add dependency: `termimad`
  - Create `limit-cli/src/render.rs` with MarkdownRenderer struct
  - Implement render(&self, markdown: &str) -> String
  - Support: headers, bold, italic, code blocks, lists, links
  - Syntax highlighting: no (plain text for code blocks)
  - Terminal width: auto-detect
  - Colors: use terminal default

  **Must NOT do**:
  - Don't implement syntax highlighting
  - Don't use custom markdown parser (use termimad)
  - Don't add custom themes

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 21
  - **Blocked By**: Task 14

  **References**:
  - termimad: https://docs.rs/termimad/latest/termimad/

  **Acceptance Criteria**:
  - [ ] MarkdownRenderer defined
  - [ ] render() works
  - [ ] Handles code blocks
  - [ ] Terminal width auto-detected
  - [ ] Unit test: render markdown

  **QA Scenarios**:
  ```
  Scenario: Markdown renders correctly
    Tool: Bash
    Steps:
      1. cargo test test_markdown_render --package limit-cli
      2. Verify formatted output
    Expected Result: Markdown formatted with colors/styles
    Evidence: .sisyphus/evidence/task-19-markdown-render.txt
  ```

  **Commit**: NO (groups with Task 21)

- [x] 20. limit-cli Session Persistence

  **What to do**:
  - Create `limit-cli/src/session.rs` with SessionManager struct
  - Implement save_session(session_id: &str, messages: &[Message]) -> Result<(), CliError>
  - Implement load_session(session_id: &str) -> Result<Vec<Message>, CliError>
  - Implement list_sessions() -> Result<Vec<SessionInfo>, CliError>
  - Store at: .limit/session.db (SQLite for metadata + binary for messages)
  - SessionInfo: id, created_at, last_accessed, message_count
  - Auto-load last session on startup
  - Auto-save on exit

  **Must NOT do**:
  - Don't implement session deletion (defer to future)
  - Don't store messages in SQLite (use binary)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 8, 14

  **References**:
  - Reuse binary persistence from limit-llm

  **Acceptance Criteria**:
  - [ ] SessionManager defined
  - [ ] save_session() works
  - [ ] load_session() works
  - [ ] list_sessions() works
  - [ ] Auto-load on startup
  - [ ] Auto-save on exit
  - [ ] Unit test: save and load

  **QA Scenarios**:
  ```
  Scenario: Session persists across restarts
    Tool: interactive_bash
    Steps:
      1. cargo run --package limit-cli
      2. Send: "hello"
      3. Send: "/exit"
      4. cargo run --package limit-cli
      5. Send: "/history"
    Expected Result: Previous "hello" message visible
    Evidence: .sisyphus/evidence/task-20-session-persistence.png
  ```

  **Commit**: NO (groups with Task 21)

- [x] 21. limit-agent ↔ limit-cli Integration

  **What to do**:
  - Create `limit-cli/src/agent_bridge.rs` with AgentBridge struct
  - Connect limit-cli REPL to limit-agent executor
  - Wire tool registry: all tools from limit-cli into limit-agent
  - Connect limit-llm client to limit-agent
  - Implement message flow: user input → llm → tool calls → tool execution → response
  - Stream events from limit-agent to limit-cli
  - Display events in REPL: Thinking..., Tool: file_read, Result: success
  - Error handling: LLM errors, tool errors, timeout
  - Integration test: full conversation with tool execution

  **Must NOT do**:
  - Don't implement agent reasoning (just wire components)
  - Don't add custom error recovery (use agent's decision tree)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (integration task)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 29
  - **Blocked By**: Tasks 6-14, 15-20

  **References**:
  - Reuse components from limit-llm, limit-agent, limit-cli

  **Acceptance Criteria**:
  - [ ] AgentBridge defined
  - [ ] Tool registry connected
  - [ ] LLM client connected
  - [ ] Message flow works
  - [ ] Events streamed to REPL
  - [ ] Errors handled
  - [ ] Integration test: chat with tool execution

  **QA Scenarios**:
  ```
  Scenario: Chat with tool execution
    Tool: interactive_bash
    Steps:
      1. cargo run --package limit-cli
      2. Send: "Read the file src/main.rs"
      3. Verify tool execution and response
    Expected Result: Tool executed, file content returned
    Evidence: .sisyphus/evidence/task-21-integration.png
  ```

  **Commit**: YES
  - Message: `feat(limit-cli): add REPL, tools, session, and agent integration`
  - Files: limit-cli/src/repl.rs, limit-cli/src/tools/*.rs, limit-cli/src/render.rs, limit-cli/src/session.rs, limit-cli/src/agent_bridge.rs
  - Pre-commit: `cargo test --package limit-cli`

- [x] 22. limit-tui Virtual DOM Core

  **What to do**:
  - Create `limit-tui/src/vdom.rs` with VNode enum
  - VNode variants: Text(String), Element{tag, attrs, children}
  - Implement render(vnode: &VNode) -> String (terminal output)
  - Implement diff(old: &VNode, new: &VNode) -> Vec<Patch>
  - Patch enum: Replace, UpdateAttrs, InsertChild, RemoveChild
  - Implement apply(node: &mut VNode, patches: Vec<Patch>)
  - Unit tests: render, diff, apply

  **Must NOT do**:
  - Don't implement full HTML DOM (just terminal primitives)
  - Don't add event handling yet

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: NO (foundation)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 23-28
  - **Blocked By**: Task 1

  **References**:
  - Virtual DOM concept: https://reactjs.org/docs/implementation-notes.html

  **Acceptance Criteria**:
  - [ ] VNode enum defined
  - [ ] render() works
  - [ ] diff() works
  - [ ] apply() works
  - [ ] Unit test: diff two trees
  - [ ] Unit test: apply patches

  **QA Scenarios**:
  ```
  Scenario: Virtual DOM diff works
    Tool: Bash
    Steps:
      1. cargo test test_vdom_diff --package limit-tui
      2. Verify patches generated
    Expected Result: Correct patches for tree changes
    Evidence: .sisyphus/evidence/task-22-vdom-core.txt
  ```

  **Commit**: NO (groups with Task 23)

- [x] 23. limit-tui Ratatui Integration

  **What to do**:
  - Add dependency: `ratatui`
  - Create `limit-tui/src/backend.rs` with RatatuiBackend struct
  - Implement render_vdom_to_ratatui(vnode: &VNode) -> ratatui::layout::Layout
  - Map VNode tags to Ratatui widgets: "text" → Paragraph, "box" → Block
  - Implement event loop: poll events → update state → render
  - Terminal setup: enable raw mode, alternate screen (optional)
  - Terminal cleanup: disable raw mode on exit
  - Performance: 60 FPS target

  **Must NOT do**:
  - Don't use alternate screen (per user request)
  - Don't implement mouse support

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 29
  - **Blocked By**: Task 22

  **References**:
  - Ratatui: https://docs.rs/ratatui/latest/ratatui/

  **Acceptance Criteria**:
  - [ ] RatatuiBackend defined
  - [ ] render_vdom_to_ratatui() works
  - [ ] Event loop works
  - [ ] Terminal setup/cleanup works
  - [ ] 60 FPS achieved
  - [ ] Unit test: render to ratatui
  - [ ] Integration test: event loop

  **QA Scenarios**:
  ```
  Scenario: Ratatui renders at 60 FPS
    Tool: interactive_bash
    Steps:
      1. cargo run --example ratatui_demo --package limit-tui
      2. Verify smooth rendering
    Expected Result: No flickering, smooth updates
    Evidence: .sisyphus/evidence/task-23-ratatui-integration.png
  ```

  **Commit**: YES
  - Message: `feat(limit-tui): add Virtual DOM core and Ratatui integration`
  - Files: limit-tui/src/vdom.rs, limit-tui/src/backend.rs
  - Pre-commit: `cargo test --package limit-tui`

- [x] 24. limit-tui Flexbox Layout

  **What to do**:
  - Create `limit-tui/src/layout.rs` with FlexboxLayout struct
  - Implement flexbox algorithm: main axis, cross axis, flex-grow, flex-shrink
  - Properties: direction (row/column), justify_content, align_items, gap
  - Implement calculate(node: &VNode, constraints: Rect) -> Vec<Rect>
  - Use stretch crate or custom implementation
  - Terminal constraints: width × height

  **Must NOT do**:
  - Don't implement full CSS flexbox (just basics)
  - Don't support all flex properties

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 25-28
  - **Blocked By**: Task 22

  **References**:
  - CSS Flexbox: https://css-tricks.com/snippets/css/a-guide-to-flexbox/

  **Acceptance Criteria**:
  - [ ] FlexboxLayout defined
  - [ ] calculate() works
  - [ ] Supports row/column
  - [ ] Handles justify_content, align_items
  - [ ] Unit test: simple layout
  - [ ] Unit test: nested layout

  **QA Scenarios**:
  ```
  Scenario: Flexbox layout calculates correctly
    Tool: Bash
    Steps:
      1. cargo test test_flexbox_layout --package limit-tui
      2. Verify layout matches expected rects
    Expected Result: Correct positions and sizes
    Evidence: .sisyphus/evidence/task-24-flexbox-layout.txt
  ```

  **Commit**: NO (groups with Task 29)

- [x] 25. limit-tui Chat View Component

  **What to do**:
  - Create `limit-tui/src/components/chat.rs` with ChatView component
  - Render list of messages (user + assistant)
  - Message rendering: role badge, content (markdown), timestamp
  - Scrollable: handle large message lists
  - Auto-scroll to bottom on new message
  - Use Virtual DOM for message list (efficient updates)

  **Must NOT do**:
  - Don't implement message input (separate task)
  - Don't add syntax highlighting (plain text)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 29
  - **Blocked By**: Tasks 22, 24

  **References**:
  - Chat UI patterns: Slack, Discord

  **Acceptance Criteria**:
  - [ ] ChatView component defined
  - [ ] Renders messages correctly
  - [ ] Scrollable
  - [ ] Auto-scroll works
  - [ ] Unit test: render messages
  - [ ] Integration test: scroll behavior

  **QA Scenarios**:
  ```
  Scenario: Chat view renders messages
    Tool: interactive_bash
    Steps:
      1. cargo run --example chat_demo --package limit-tui
      2. Verify messages displayed
    Expected Result: Messages visible, scrollable
    Evidence: .sisyphus/evidence/task-25-chat-view.png
  ```

  **Commit**: NO (groups with Task 29)

- [x] 26. limit-tui Diff View Component

  **What to do**:
  - Create `limit-tui/src/components/diff.rs` with DiffView component
  - Render unified diff format
  - Color coding: additions (green), deletions (red), context (white)
  - Line numbers displayed
  - Scrollable: handle 10K+ line diffs
  - Performance: render visible lines only (virtualization)

  **Must NOT do**:
  - Don't implement side-by-side diff (unified only)
  - Don't add syntax highlighting

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 29
  - **Blocked By**: Tasks 22, 24

  **References**:
  - Unified diff format: https://www.gnu.org/software/diffutils/manual/html_node/Detailed-Unified.html

  **Acceptance Criteria**:
  - [ ] DiffView component defined
  - [ ] Renders diff correctly
  - [ ] Color coding works
  - [ ] Handles 10K+ lines
  - [ ] Virtualization works
  - [ ] Unit test: render diff
  - [ ] Performance test: 10K lines

  **QA Scenarios**:
  ```
  Scenario: Diff view handles large file
    Tool: interactive_bash
    Steps:
      1. cargo run --example diff_demo --package limit-tui
      2. Load 10K line diff
      3. Verify smooth scrolling
    Expected Result: No lag, smooth render
    Evidence: .sisyphus/evidence/task-26-diff-view.png
  ```

  **Commit**: NO (groups with Task 29)

- [x] 27. limit-tui Progress Indicators

  **What to do**:
  - Create `limit-tui/src/components/progress.rs` with ProgressBar and Spinner components
  - ProgressBar: percentage, label, animated fill
  - Spinner: rotating chars (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏), label
  - Use Ratatui widgets: Gauge for progress, custom animation for spinner
  - Performance: animation at 10 FPS
  - Components reusable across app

  **Must NOT do**:
  - Don't add multiple spinner styles (one style)
  - Don't implement progress bar cancellation

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 29
  - **Blocked By**: Tasks 22, 24

  **References**:
  - Ratatui Gauge: https://docs.rs/ratatui/latest/ratatui/widgets/struct.Gauge.html

  **Acceptance Criteria**:
  - [ ] ProgressBar component defined
  - [ ] Spinner component defined
  - [ ] Animation works
  - [ ] Reusable
  - [ ] Unit test: progress bar render
  - [ ] Unit test: spinner animation

  **QA Scenarios**:
  ```
  Scenario: Progress indicators render
    Tool: interactive_bash
    Steps:
      1. cargo run --example progress_demo --package limit-tui
      2. Verify spinner and progress bar visible
    Expected Result: Animated components
    Evidence: .sisyphus/evidence/task-27-progress-indicators.png
  ```

  **Commit**: NO (groups with Task 29)

- [x] 28. limit-tui Interactive Prompts

  **What to do**:
  - Create `limit-tui/src/components/prompt.rs` with InputPrompt and SelectPrompt components
  - InputPrompt: text input with cursor, placeholder, validation
  - SelectPrompt: list of options, arrow key navigation, selection
  - Event handling: keyboard input, enter to confirm, escape to cancel
  - Cursor: visible and movable
  - Validation: error message display

  **Must NOT do**:
  - Don't implement multi-line input (single line only)
  - Don't add auto-completion

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 29
  - **Blocked By**: Tasks 22, 24

  **References**:
  - Ratatui input handling: https://docs.rs/ratatui/latest/ratatui/

  **Acceptance Criteria**:
  - [ ] InputPrompt component defined
  - [ ] SelectPrompt component defined
  - [ ] Keyboard handling works
  - [ ] Cursor visible
  - [ ] Validation works
  - [ ] Unit test: input prompt
  - [ ] Unit test: select prompt

  **QA Scenarios**:
  ```
  Scenario: Interactive prompts work
    Tool: interactive_bash
    Steps:
      1. cargo run --example prompt_demo --package limit-tui
      2. Type text, press enter
      3. Navigate list, select option
    Expected Result: Input captured, selection made
    Evidence: .sisyphus/evidence/task-28-interactive-prompts.png
  ```

  **Commit**: NO (groups with Task 29)

- [x] 29. limit-cli ↔ limit-tui Integration

  **What to do**:
  - Create `limit-cli/src/tui_bridge.rs` with TuiBridge struct
  - Connect limit-cli REPL to limit-tui rendering
  - Wire event stream from limit-agent to limit-tui
  - Display agent events in TUI: Thinking (spinner), Tool execution (progress), Result (chat)
  - Layout: chat view (main area), input prompt (bottom), status bar (top)
  - Error handling: TUI errors, render errors
  - Integration test: full conversation flow with TUI rendering

  **Must NOT do**:
  - Don't implement split views (single layout)
  - Don't add custom themes

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: NO (integration task)
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 30
  - **Blocked By**: Tasks 4, 21, 23-28

  **References**:
  - Reuse components from limit-tui, limit-cli, limit-agent

  **Acceptance Criteria**:
  - [ ] TuiBridge defined
  - [ ] Event stream connected
  - [ ] Agent events rendered
  - [ ] Layout works
  - [ ] Errors handled
  - [ ] Integration test: full flow with TUI

  **QA Scenarios**:
  ```
  Scenario: TUI renders agent conversation
    Tool: interactive_bash
    Steps:
      1. cargo run --package limit-cli
      2. Send: "Read the file src/main.rs"
      3. Verify TUI shows: spinner → tool execution → result
    Expected Result: Smooth TUI updates, chat visible
    Evidence: .sisyphus/evidence/task-29-tui-integration.png
  ```

  **Commit**: YES
  - Message: `feat(limit-tui): add components and integrate with limit-cli`
  - Files: limit-tui/src/components/*.rs, limit-tui/src/layout.rs, limit-cli/src/tui_bridge.rs
  - Pre-commit: `cargo test --workspace`

- [x] 30. End-to-End Integration Test

  **What to do**:
  - Create integration test: `tests/e2e_test.rs`
  - Test scenario 1: chat with Anthropic (mock API)
  - Test scenario 2: read file, verify content
  - Test scenario 3: bash command, verify output
  - Test scenario 4: git status, verify parsing
  - Test scenario 5: session save/load, verify persistence
  - Test scenario 6: TUI rendering, verify components
  - All tests use real components (not mocks) except LLM API
  - Test coverage: all 4 crates working together

  **Must NOT do**:
  - Don't test with real Anthropic API (use mock)
  - Don't skip integration tests

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (final integration)
  - **Parallel Group**: Wave 4
  - **Blocks**: FINAL
  - **Blocked By**: Task 29

  **References**:
  - Rust integration testing: https://doc.rust-lang.org/book/ch11-03-test-organization.html#integration-tests

  **Acceptance Criteria**:
  - [ ] e2e_test.rs created
  - [ ] All 6 scenarios tested
  - [ ] All tests pass
  - [ ] cargo test --workspace succeeds
  - [ ] No regressions

  **QA Scenarios**:
  ```
  Scenario: E2E tests pass
    Tool: Bash
    Steps:
      1. cargo test --workspace --test e2e_test
      2. Verify all tests pass
    Expected Result: 6/6 tests pass
    Evidence: .sisyphus/evidence/task-30-e2e-tests.txt

  Scenario: Full workspace tests pass
    Tool: Bash
    Steps:
      1. cargo test --all
      2. Verify no failures
    Expected Result: All unit + integration tests pass
    Evidence: .sisyphus/evidence/task-30-all-tests.txt
  ```

  **Commit**: YES
  - Message: `test: add end-to-end integration tests`
  - Files: tests/e2e_test.rs
  - Pre-commit: `cargo test --workspace`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Rejection → fix → re-run.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, curl endpoint, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --all` + `cargo test --all` + `cargo fmt --check`. Review all changed files for: unwrap() without error handling, panic!(), TODO/FIXME comments, dead code, unused dependencies. Check AI slop: excessive comments, over-abstraction, generic names (data/result/item/temp).
  Output: `Clippy [PASS/FAIL] | Tests [N pass/N fail] | Format [PASS/FAIL] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Execute EVERY QA scenario from EVERY task — follow exact steps, capture evidence. Test cross-task integration (features working together, not isolation). Test edge cases: empty file, 50MB file, git conflict, Docker not installed. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Detect cross-task contamination: Task N touching Task M's files. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **Wave 1**: `feat: add workspace setup` — after Task 1
- **Wave 2**: `feat(limit-llm): add Anthropic client` — after Tasks 6-9
- **Wave 3**: `feat(limit-agent): add tool execution` — after Tasks 10-13
- **Wave 4**: `feat(limit-cli): add REPL interface` — after Tasks 14-21
- **Wave 5**: `feat(limit-tui): add terminal UI` — after Tasks 22-29
- **Final**: `chore: integration complete` — after Task 30

---

## Success Criteria

### Verification Commands
```bash
cargo build --release          # Expected: build succeeds
cargo test --all               # Expected: all tests pass
cargo run --package limit-cli  # Expected: REPL starts
cargo clippy --all             # Expected: no warnings
cargo fmt --check              # Expected: formatted
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass
- [ ] CLI runs and responds to input
- [ ] Can chat with Anthropic
- [ ] Can execute file operations
- [ ] Can execute bash commands
- [ ] Can execute git operations
- [ ] Session persists across restarts
- [ ] TUI renders correctly
- [ ] Docker sandbox works (optional)
