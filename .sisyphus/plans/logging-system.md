# Add Debug/Production Logging System

## TL;DR
> Add tracing-based logging: debug in dev, zero in prod via compile-time features. Instrument critical events (tool exec, API calls, state changes, errors). Human-readable output to stderr.

## Context

### Original Request
Adicionar logs de debug em todas as crates. Dev: logs detalhados. Prod: nenhum log. Zero overhead em produção.

### Interview Summary
- **4 crates**: limit-cli (bin), limit-agent (lib), limit-llm (lib), limit-tui (lib)
- **Current**: Zero logging (apenas eprintln! em client.rs:192, main.rs:11, sandbox.rs:165)
- **Tool**: tracing + tracing-subscriber (Tokio team standard)
- **Verbosity**: Debug (dev), OFF (prod compile-time)
- **Format**: Human-readable, stderr output
- **Runtime**: RUST_LOG env var control

### Metis Review
**Identified Gaps** (addressed):
- **eprintln! conversion**: Found 3 critical debug prints to convert (client.rs, sandbox.rs, main.rs)
- **TUI guard**: Logs to stderr only (avoid corrupting TUI alternate screen)
- **Tool args truncation**: Max 500 chars to prevent log spam
- **Privacy**: Never log user message content, only metadata

---

## Work Objectives

### Core Objective
Add structured logging system with compile-time filtering (zero prod overhead) and runtime configurability.

### Concrete Deliverables
- Tracing deps in all crates (compile-time features in limit-cli)
- Subscriber init in main.rs (before Repl::new())
- Convert 3 eprintln! to tracing macros
- Instrument critical functions with #[instrument]
- Unit tests for logging behavior
- QA scenarios for runtime verification

### Definition of Done
- [x] `RUST_LOG=debug cargo run` shows debug logs
- [x] `cargo run --release` produces zero debug output
- [x] `strings target/release/limit | grep tracing::debug` returns 0 matches
- [x] All existing eprintln! debug statements converted
- [x] Tool execution logged with duration
- [x] API calls logged (request/response/retry)
- [x] Tests pass: `cargo test --workspace`

### Must Have
- tracing dependency in all 4 crates
- Compile-time filtering via features (max_level_debug, release_max_level_off)
- EnvFilter for RUST_LOG runtime control
- Instrument critical paths: tool execution, API calls, state changes, errors

### Must NOT Have (Guardrails)
- NO logging in TUI render() methods (60fps hot path)
- NO full tool args in logs (truncate to 500 chars)
- NO user message content in logs (privacy)
- NO JSON formatting (user wants human-readable)
- NO stdout logging (corrupts TUI) - stderr only
- NO custom log macros (use tracing::*)
- NO logging infrastructure beyond tracing-subscriber (no exporters, metrics)

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (built-in Rust test runner)
- **Automated tests**: YES - unit tests
- **Framework**: cargo test (built-in)
- **Agent QA**: YES - runtime scenario verification

### QA Policy
Every task includes agent-executed QA scenarios (see TODO template).
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Backend/CLI**: Bash (cargo run, RUST_LOG, strings, grep)
- **Tests**: Bash (cargo test)

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — deps + setup):
├── Task 1: Add tracing deps to all crates [quick]
├── Task 2: Create logging init module in limit-cli [quick]
└── Task 3: Convert existing eprintln! to tracing [quick]

Wave 2 (After Wave 1 — instrumentation):
├── Task 4: Instrument limit-llm (API calls, streaming) [quick]
├── Task 5: Instrument limit-agent (tool exec, sandbox) [quick]
├── Task 6: Instrument limit-cli (repl, session) [quick]
└── Task 7: Instrument limit-tui (lifecycle only) [quick]

Wave 3 (After Wave 2 — testing + verification):
├── Task 8: Add unit tests for logging [quick]
├── Task 9: QA - Dev mode logs work [quick]
├── Task 10: QA - Prod mode silent [quick]
└── Task 11: QA - Compile-time filtering verified [quick]

Critical Path: T1 → T2 → T3 → T4-T7 → T8-T11
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 3 (Waves 1 & 2)
```

### Dependency Matrix
- **1-3**: — — 4-7, 1
- **4**: 1, 3 — 8, 2
- **5**: 1, 3 — 8, 2
- **6**: 1, 3 — 8, 2
- **7**: 1, 3 — 8, 2
- **8**: 4-7 — 9-11, 3
- **9-11**: 8 — F1-F4, 4

### Agent Dispatch Summary
- **1-3**: quick (deps + conversion)
- **4-7**: quick (instrumentation per crate)
- **8-11**: quick (testing + QA)

---

## TODOs

- [x] 1. Add Tracing Dependencies

  **What to do**:
  - Add `tracing = "0.1"` to limit-agent/Cargo.toml, limit-llm/Cargo.toml, limit-tui/Cargo.toml
  - Add `tracing = { version = "0.1", features = ["max_level_debug", "release_max_level_off"] }` to limit-cli/Cargo.toml
  - Add `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` to limit-cli/Cargo.toml (NO json feature)
  - Run `cargo check --workspace` to verify deps compile

  **Must NOT do**:
  - Add json feature to tracing-subscriber
  - Add compile-time features to lib crates (only binary)
  - Add extra deps (tracing-appender, etc)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple dependency addition, no complex logic
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: NO (must complete first)
  - **Parallel Group**: Wave 1 (with Tasks 2, 3)
  - **Blocks**: 4-7 (instrumentation tasks)
  - **Blocked By**: None

  **References**:
  - `limit-cli/Cargo.toml` - Binary crate, add features here
  - `limit-llm/Cargo.toml`, `limit-agent/Cargo.toml`, `limit-tui/Cargo.toml` - Lib crates, simple dep
  - Research finding: "max_level_debug for dev, release_max_level_off for prod"

  **Acceptance Criteria**:
  - [ ] All 4 Cargo.toml files updated
  - [ ] `cargo check --workspace` succeeds
  - [ ] `cargo tree -p tracing` shows correct features in limit-cli

  **QA Scenarios**:

  ```
  Scenario: Dependencies compile correctly
    Tool: Bash
    Preconditions: Cargo.toml files updated
    Steps:
      1. cargo check --workspace
    Expected Result: Build succeeds, no errors
    Evidence: .sisyphus/evidence/task-01-deps-compile.txt
  ```

  **Commit**: NO (groups with 1-3)
  - Message: `chore(deps): add tracing dependencies`
  - Files: `*/Cargo.toml`

- [x] 2. Create Logging Init Module

  **What to do**:
  - Create `limit-cli/src/logging.rs` with `init_logging()` function
  - Configure EnvFilter with RUST_LOG fallback to "info"
  - Use fmt::layer() with .with_target(true).with_thread_ids(false)
  - Initialize in main.rs as FIRST line before Repl::new()
  - Use tracing::subscriber::set_global_default()

  **Must NOT do**:
  - Add JSON formatter
  - Log to stdout (use stderr)
  - Initialize inside Repl::new()

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple setup code, well-documented pattern
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3)
  - **Blocks**: 4-7 (need subscriber init)
  - **Blocked By**: Task 1 (deps)

  **References**:
  - `limit-cli/src/main.rs` - Call init_logging() before repl
  - Research pattern:
    ```rust
    use tracing_subscriber::{fmt, EnvFilter, prelude::*};

    pub fn init_logging() {
        let filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new("info"))
            .unwrap();

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_target(true))
            .init();
    }
    ```

  **Acceptance Criteria**:
  - [ ] `limit-cli/src/logging.rs` created with init_logging()
  - [ ] main.rs calls init_logging() before Repl::new()
  - [x] `RUST_LOG=debug cargo run` shows logs
  - [ ] Logs go to stderr, not stdout

  **QA Scenarios**:

  ```
  Scenario: Logging initialized in dev mode
    Tool: Bash
    Preconditions: Code compiled
    Steps:
      1. RUST_LOG=debug cargo run --package limit-cli 2>&1 | head -20
    Expected Result: Output contains tracing metadata (targets, timestamps)
    Evidence: .sisyphus/evidence/task-02-init-dev.txt

  Scenario: Default log level works without RUST_LOG
    Tool: Bash
    Steps:
      1. cargo run --package limit-cli 2>&1 | head -20
    Expected Result: Only info/warn/error logs shown (no debug)
    Evidence: .sisyphus/evidence/task-02-init-default.txt
  ```

  **Commit**: NO (groups with 1-3)

- [x] 3. Convert Existing eprintln! to Tracing

  **What to do**:
  - `limit-llm/src/client.rs:192`: Replace `eprintln!("[DEBUG] Chunk: ...")` with `tracing::debug!("Chunk received", chunk = %text[..500])`
  - `limit-agent/src/sandbox.rs:165`: Replace `println!("Docker available: ...")` with `tracing::info!("Docker availability checked", available = %available)`
  - `limit-cli/src/main.rs:11`: Replace `eprintln!("Error: {e}")` with `tracing::error!("Application error", error = %e)`
  - Keep all REPL user-facing println! in repl.rs (they're part of UI, not debug)
  - Keep test/example println! (not debug logs)

  **Must NOT do**:
  - Convert REPL user-facing println! (lines 26, 31, 48, etc in repl.rs)
  - Convert test output println!
  - Log full chunk text (truncate to 500 chars)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple find-and-replace with grep
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2)
  - **Blocks**: 4-7 (need base conversion done)
  - **Blocked By**: Task 1 (deps)

  **References**:
  - `limit-llm/src/client.rs:192` - SSE chunk debug output
  - `limit-agent/src/sandbox.rs:165` - Docker check info
  - `limit-cli/src/main.rs:11` - Main error handler
  - Grep output shows 60 matches - only convert these 3 critical ones

  **Acceptance Criteria**:
  - [ ] 3 eprintln!/println! converted to tracing macros
  - [ ] Chunk text truncated to 500 chars
  - [ ] REPL user output preserved (println! remains)
  - [ ] `grep -r "eprintln!" limit-llm/src/ limit-agent/src/ limit-cli/src/main.rs` returns 0 matches

  **QA Scenarios**:

  ```
  Scenario: SSE chunk logging works
    Tool: Bash
    Preconditions: API configured, code running
    Steps:
      1. RUST_LOG=limit_llm=debug cargo run --package limit-cli
      2. Send message to trigger API call
      3. Check stderr for "Chunk received" logs
    Expected Result: Debug logs show truncated chunk data
    Evidence: .sisyphus/evidence/task-03-chunk-log.txt

  Scenario: No eprintln in production code
    Tool: Bash
    Steps:
      1. grep -r "eprintln!" limit-llm/src/ limit-agent/src/ limit-cli/src/main.rs
    Expected Result: Exit code 1 (no matches)
    Evidence: .sisyphus/evidence/task-03-no-eprintln.txt
  ```

  **Commit**: YES
  - Message: `refactor(logging): convert eprintln to tracing`
  - Files: `limit-llm/src/client.rs`, `limit-agent/src/sandbox.rs`, `limit-cli/src/main.rs`
  - Pre-commit: `cargo check --workspace`

- [x] 4. Instrument limit-llm (API Layer)

  **What to do**:
  - Add `#[instrument(skip(self))]` to `client.rs` methods: `send_message`, `stream_response`
  - Log API request: `tracing::info!("API request", model = %model, max_tokens = %max_tokens)`
  - Log API response: `tracing::debug!("API response received", tokens = %usage)`
  - Log retry attempts: `tracing::warn!("API retry", attempt = %n, delay_ms = %delay)`
  - Log streaming errors: `tracing::error!("Stream error", error = %e)`

  **Must NOT do**:
  - Log full request/response bodies (privacy, size)
  - Log API keys
  - Instrument internal helper functions (only public API)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Straightforward instrumentation of async functions
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6, 7)
  - **Blocks**: 8 (tests)
  - **Blocked By**: 1, 2 (deps + init)

  **References**:
  - `limit-llm/src/client.rs` - Main API client
  - `limit-llm/src/tracking.rs` - Token tracking (log usage stats)
  - Research: `#[instrument(skip(self))]` pattern for methods

  **Acceptance Criteria**:
  - [ ] 5+ functions instrumented with #[instrument]
  - [x] API calls logged with model/tokens metadata
  - [ ] Retries logged at WARN level
  - [ ] Errors logged at ERROR level
  - [ ] `RUST_LOG=limit_llm=debug cargo run` shows API logs

  **QA Scenarios**:

  ```
  Scenario: API request logged with metadata
    Tool: Bash
    Steps:
      1. RUST_LOG=limit_llm=info cargo run --package limit-cli 2>&1 | tee log.txt
      2. Send message to trigger API call
      3. grep "API request" log.txt
    Expected Result: Log contains model name and token count
    Evidence: .sisyphus/evidence/task-04-api-request.txt

  Scenario: Retry attempts logged
    Tool: Bash
    Steps:
      1. Simulate API failure (network issue)
      2. RUST_LOG=limit_llm=warn cargo run --package limit-cli 2>&1
      3. Trigger retry scenario
    Expected Result: "API retry" log appears
    Evidence: .sisyphus/evidence/task-04-retry.txt
  ```

  **Commit**: NO (groups with 4-7)

- [x] 5. Instrument limit-agent (Tool Execution)

  **What to do**:
  - Add `#[instrument(skip(self))]` to `executor.rs`: `execute_tool`, `execute_tools_parallel`
  - Log tool start: `tracing::debug!("Tool execution started", tool = %name, args_truncated = %args[..500])`
  - Log tool complete: `tracing::info!("Tool execution completed", tool = %name, duration_ms = %elapsed)`
  - Log tool error: `tracing::error!("Tool execution failed", tool = %name, error = %e)`
  - Add `#[instrument]` to `sandbox.rs`: Docker operations
  - Log state changes in `state.rs`: `tracing::debug!("Agent state updated", state = ?new_state)`

  **Must NOT do**:
  - Log full tool arguments (truncate to 500 chars)
  - Instrument sandbox execution hot path
  - Log sensitive data from tool results

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Tool execution is well-structured, easy to instrument
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 6, 7)
  - **Blocks**: 8 (tests)
  - **Blocked By**: 1, 2

  **References**:
  - `limit-agent/src/executor.rs` - Tool execution logic
  - `limit-agent/src/sandbox.rs` - Docker operations
  - `limit-agent/src/state.rs` - Agent state management
  - Metis guard: "Truncate tool args to 500 chars"

  **Acceptance Criteria**:
  - [x] Tool execution logged with name + duration
  - [ ] Tool args truncated to 500 chars
  - [ ] Parallel tool execution traced separately
  - [ ] Sandbox operations logged at INFO
  - [ ] State changes logged at DEBUG

  **QA Scenarios**:

  ```
  Scenario: Tool execution logged with duration
    Tool: Bash
    Steps:
      1. RUST_LOG=limit_agent=debug cargo run --package limit-cli 2>&1 | tee log.txt
      2. Trigger tool execution (e.g., "read file X")
      3. grep -A2 "Tool execution started" log.txt
    Expected Result: Log shows tool name, truncated args, duration
    Evidence: .sisyphus/evidence/task-05-tool-exec.txt

  Scenario: Tool args truncated
    Tool: Bash
    Steps:
      1. RUST_LOG=limit_agent=debug cargo run --package limit-cli 2>&1
      2. Execute tool with large arguments
      3. Verify args logged are max 500 chars
    Expected Result: Truncation visible in logs
    Evidence: .sisyphus/evidence/task-05-args-truncated.txt
  ```

  **Commit**: NO (groups with 4-7)

- [x] 6. Instrument limit-cli (REPL + Session)

  **What to do**:
  - Add `#[instrument(skip(self))]` to `repl.rs`: `run`, `process_message`, `handle_command`
  - Log REPL start: `tracing::info!("REPL started", session_id = %id)`
  - Log message processing: `tracing::debug!("Processing message", length = %msg.len())`
  - Log session save: `tracing::debug!("Session saved", messages = %count)`
  - Log session load: `tracing::info!("Session loaded", session_id = %id, messages = %count)`
  - Add `#[instrument]` to `session.rs`: save/load operations
  - Add `#[instrument]` to `agent_bridge.rs`: agent interactions

  **Must NOT do**:
  - Log user message content (privacy - only metadata like length)
  - Log API keys or secrets
  - Instrument TUI rendering code (separate crate)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: REPL flow is linear and well-defined
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5, 7)
  - **Blocks**: 8 (tests)
  - **Blocked By**: 1, 2

  **References**:
  - `limit-cli/src/repl.rs` - REPL loop
  - `limit-cli/src/session.rs` - Session persistence
  - `limit-cli/src/agent_bridge.rs` - Agent interaction
  - Metis guard: "Never log user message content"

  **Acceptance Criteria**:
  - [ ] REPL lifecycle events logged (start/stop)
  - [ ] Message processing logged (metadata only)
  - [ ] Session save/load logged with counts
  - [ ] No user message content in logs
  - [ ] Agent interactions logged

  **QA Scenarios**:

  ```
  Scenario: REPL lifecycle logged
    Tool: Bash
    Steps:
      1. RUST_LOG=limit_cli=info cargo run --package limit-cli 2>&1 | tee log.txt
      2. Start REPL, send one message, exit
      3. grep "REPL started\|Session saved" log.txt
    Expected Result: Both lifecycle events logged
    Evidence: .sisyphus/evidence/task-06-repl-lifecycle.txt

  Scenario: No user content in logs
    Tool: Bash
    Steps:
      1. RUST_LOG=limit_cli=debug cargo run --package limit-cli 2>&1 | tee log.txt
      2. Send unique test message "UNIQUE_TEST_STRING_12345"
      3. grep "UNIQUE_TEST_STRING_12345" log.txt
    Expected Result: No matches (content not logged)
    Evidence: .sisyphus/evidence/task-06-no-content.txt
  ```

  **Commit**: NO (groups with 4-7)

- [x] 7. Instrument limit-tui (Lifecycle Only)

  **What to do**:
  - Add `#[instrument(skip(self))]` to `lib.rs`: `init`, `run` (if exists)
  - Log TUI init: `tracing::info!("TUI initialized")`
  - Log component lifecycle: `tracing::debug!("Component created", component = %name)`
  - Add minimal instrumentation to `backend.rs`, `layout.rs` (lifecycle only)

  **Must NOT do**:
  - Instrument render() methods (60fps hot path)
  - Log on every frame
  - Add spans in rendering code

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Minimal instrumentation, avoiding hot paths
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5, 6)
  - **Blocks**: 8 (tests)
  - **Blocked By**: 1, 2

  **References**:
  - `limit-tui/src/lib.rs` - Entry point
  - `limit-tui/src/backend.rs` - Terminal backend
  - `limit-tui/src/layout.rs` - Layout engine
  - Metis guard: "DON'T instrument render() methods"

  **Acceptance Criteria**:
  - [ ] TUI init logged at INFO
  - [ ] Component lifecycle logged at DEBUG
  - [ ] NO instrumentation in render methods
  - [ ] Logs go to stderr (not stdout)

  **QA Scenarios**:

  ```
  Scenario: TUI init logged
    Tool: Bash
    Steps:
      1. RUST_LOG=limit_tui=info cargo run --package limit-cli 2>&1
      2. Trigger TUI mode (if available)
      3. Check for "TUI initialized" log
    Expected Result: TUI init log appears
    Evidence: .sisyphus/evidence/task-07-tui-init.txt

  Scenario: No render spam
    Tool: Bash
    Steps:
      1. RUST_LOG=limit_tui=trace cargo run --package limit-cli 2>&1 | wc -l
      2. Run TUI for 10 seconds
      3. Verify log count is low (<100 lines, not thousands)
    Expected Result: Minimal logs despite continuous rendering
    Evidence: .sisyphus/evidence/task-07-no-render-spam.txt
  ```

  **Commit**: YES
  - Message: `feat(logging): instrument all crates with tracing`
  - Files: `limit-llm/src/**/*.rs`, `limit-agent/src/**/*.rs`, `limit-cli/src/**/*.rs`, `limit-tui/src/**/*.rs`
  - Pre-commit: `cargo test --workspace && cargo clippy --workspace`

- [x] 8. Add Unit Tests for Logging

  **What to do**:
  - Create `limit-cli/src/logging_test.rs` with tests:
    - Test: Subscriber initialized correctly
    - Test: EnvFilter parses RUST_LOG correctly
    - Test: Compile-time filtering works (via cfg macro)
  - Create `limit-llm/tests/logging_test.rs`:
    - Test: API calls emit info logs
    - Test: Errors emit error logs
  - Use `tracing_subscriber::fmt::TestWriter` for capturing logs in tests
  - Run `cargo test --workspace` to verify

  **Must NOT do**:
  - Add test frameworks (use built-in)
  - Mock HTTP for logging tests (test log emission, not HTTP)
  - Test log formatting details

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple unit tests with tracing testing utilities
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: NO (final verification)
  - **Parallel Group**: Wave 3 (with Tasks 9, 10, 11)
  - **Blocks**: 9-11 (QA scenarios need tests passing)
  - **Blocked By**: 4-7 (instrumentation)

  **References**:
  - `limit-cli/tests/e2e_test.rs` - Existing test patterns
  - `tracing_subscriber::fmt::TestWriter` - Capture logs in tests
  - Research: Testing tracing with dedicated test writer

  **Acceptance Criteria**:
  - [ ] 3+ unit tests added
  - [ ] Tests verify log levels and content
  - [ ] `cargo test --workspace` passes
  - [ ] No test framework added (use built-in)

  **QA Scenarios**:

  ```
  Scenario: Unit tests pass
    Tool: Bash
    Steps:
      1. cargo test --workspace 2>&1 | tee test-output.txt
    Expected Result: All tests pass, 0 failures
    Failure Indicators: "test result: FAILED" or compilation errors
    Evidence: .sisyphus/evidence/task-08-tests-pass.txt
  ```

  **Commit**: YES
  - Message: `test(logging): add unit tests for logging behavior`
  - Files: `limit-cli/src/logging_test.rs`, `limit-llm/tests/logging_test.rs`
  - Pre-commit: `cargo test --workspace`

- [x] 9. QA - Dev Mode Logs Work

  **What to do**:
  - Run `RUST_LOG=debug cargo run --package limit-cli`
  - Send test message to trigger API call
  - Verify debug logs appear with correct format
  - Verify logs go to stderr (not stdout)
  - Check log metadata: targets, timestamps

  **Must NOT do**:
  - Use production build
  - Check specific log content (only verify presence)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple manual verification
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 8, 10, 11)
  - **Blocks**: None
  - **Blocked By**: 8 (tests must pass)

  **References**:
  - Main verification scenario from requirements

  **Acceptance Criteria**:
  - [ ] Debug logs appear in dev mode
  - [ ] Logs human-readable (not JSON)
  - [ ] Logs on stderr
  - [ ] Multiple crates emit logs (limit_llm, limit_agent, limit_cli)

  **QA Scenarios**:

  ```
  Scenario: Dev mode shows debug logs
    Tool: Bash
    Steps:
      1. RUST_LOG=debug cargo run --package limit-cli 2>&1 | tee dev-logs.txt
      2. Send message "hello"
      3. grep -c "DEBUG" dev-logs.txt
    Expected Result: Count > 0 (debug logs present)
    Failure Indicators: Count = 0 or no output
    Evidence: .sisyphus/evidence/task-09-dev-mode.txt
  ```

  **Commit**: NO (QA only)

- [x] 10. QA - Production Mode Silent

  **What to do**:
  - Run `cargo run --package limit-cli --release`
  - Send test message
  - Verify NO debug logs appear
  - Check stderr is empty (or only errors if any)

  **Must NOT do**:
  - Use RUST_LOG env var (test default behavior)
  - Run in dev mode

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple verification
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 8, 9, 11)
  - **Blocks**: None
  - **Blocked By**: 8

  **References**:
  - Critical requirement: zero logs in production

  **Acceptance Criteria**:
  - [ ] Release build runs without debug output
  - [ ] Stderr empty (or only errors if critical)
  - [ ] User-facing REPL output still works (println!)

  **QA Scenarios**:

  ```
  Scenario: Production mode is silent
    Tool: Bash
    Steps:
      1. cargo run --package limit-cli --release 2>&1 | tee prod-logs.txt
      2. Send message "hello"
      3. grep -c "DEBUG" prod-logs.txt
    Expected Result: Count = 0 (no debug logs)
    Failure Indicators: Count > 0
    Evidence: .sisyphus/evidence/task-10-prod-silent.txt
  ```

  **Commit**: NO (QA only)

- [x] 11. QA - Compile-Time Filtering Verified

  **What to do**:
  - Run `cargo build --package limit-cli --release`
  - Run `strings target/release/limit | grep -c "tracing::debug"`
  - Verify count is 0 (tracing debug calls compiled out)
  - Run `nm target/release/limit | grep tracing` to check symbols
  - Verify binary size is reasonable

  **Must NOT do**:
  - Use dev build
  - Check specific symbol names (just count)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Binary inspection commands
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 8, 9, 10)
  - **Blocks**: None
  - **Blocked By**: 8

  **References**:
  - Metis recommendation: "Verify compile-time filtering with strings"
  - Critical for zero overhead claim

  **Acceptance Criteria**:
  - [ ] `strings` shows 0 matches for "tracing::debug"
  - [ ] Binary size reasonable (<50MB)
  - [ ] Release build completes successfully

  **QA Scenarios**:

  ```
  Scenario: Compile-time filtering removes debug code
    Tool: Bash
    Steps:
      1. cargo build --package limit-cli --release
      2. strings target/release/limit | grep -c "tracing::debug"
    Expected Result: 0 (no debug strings in binary)
    Failure Indicators: Count > 0
    Evidence: .sisyphus/evidence/task-11-compile-filter.txt
  ```

  **Commit**: NO (QA only)

---

## Final Verification Wave (MANDATORY)

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read plan end-to-end. Verify each "Must Have": tracing in all crates, compile-time features, EnvFilter, instrumented paths. Verify each "Must NOT Have": no JSON, no stdout logs, no render instrumentation, no full args. Check evidence files exist.
  Output: `Must Have [4/4] | Must NOT Have [6/6] | Tasks [11/11] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --workspace --all-targets` + `cargo test --workspace`. Review all changed files for: `as any`, `unwrap()` without error handling, unused imports, commented code. Check AI slop: excessive comments, generic names.
  Output: `Clippy [PASS/FAIL] | Tests [PASS/FAIL] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Execute scenarios from Tasks 9-11: dev mode logs, prod mode silent, compile-time filtering. Test integration: API call → tool execution → response. Test edge cases: large tool args, API failure. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [3/3 pass] | Integration [1/1] | Edge Cases [2 tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify 1:1 — everything in spec was built, nothing beyond spec. Check "Must NOT do" compliance. Detect cross-task contamination.
  Output: `Tasks [11/11 compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **1-3**: `chore(deps): add tracing dependencies` — */Cargo.toml, limit-cli/src/logging.rs, limit-cli/src/main.rs, limit-llm/src/client.rs, limit-agent/src/sandbox.rs
- **4-7**: `feat(logging): instrument all crates with tracing` — limit-llm/src/**/*.rs, limit-agent/src/**/*.rs, limit-cli/src/**/*.rs, limit-tui/src/**/*.rs
- **8**: `test(logging): add unit tests for logging behavior` — limit-cli/src/logging_test.rs, limit-llm/tests/logging_test.rs

---

## Success Criteria

### Verification Commands
```bash
# Dev mode shows logs
RUST_LOG=debug cargo run --package limit-cli 2>&1 | grep -c DEBUG
# Expected: > 0

# Production is silent
cargo run --package limit-cli --release 2>&1 | grep -c DEBUG
# Expected: 0

# Compile-time filtering
cargo build --package limit-cli --release && strings target/release/limit | grep -c "tracing::debug"
# Expected: 0

# Tests pass
cargo test --workspace
# Expected: all pass

# EnvFilter works
RUST_LOG=limit_llm=trace cargo run --package limit-cli 2>&1 | grep "limit_llm"
# Expected: contains limit_llm traces
```

### Final Checklist
- [x] All "Must Have" present (4 items)
- [x] All "Must NOT Have" absent (6 items)
- [ ] All tests pass
- [x] Evidence files exist in .sisyphus/evidence/

---

## Unresolved Questions
None - all requirements clarified during interview.
