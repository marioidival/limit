# Crates.io Publishing Plan

## TL;DR

> **Quick Summary**: Publish all 4 Limit crates (limit-llm, limit-agent, limit-tui, limit-cli) to crates.io at version 0.0.18. Add missing metadata, create LICENSE and READMEs, validate, publish in dependency order, verify installation.
> 
> **Deliverables**:
> - All 4 crates published to crates.io
> - LICENSE file created
> - README.md files for each crate
> - Git tag v0.0.18 created
> - Installation via `cargo install limit-cli` working
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 3 waves with parallel tasks
> **Critical Path**: Pre-flight → Metadata → Validation → Publish (sequential) → Verify

---

## Context

### Original Request
Create a plan to publish the Limit project to crates.io. All 4 crates should be published at version 0.0.18 with proper metadata, validation, and post-publish verification.

### Interview Summary
**Key Discussions**:
- **Version**: Keep current 0.0.18 across all crates
- **Scope**: All 4 crates (limit-llm, limit-agent, limit-tui, limit-cli)
- **Validation**: Full dry-run before actual publish
- **Dependencies**: Keep path deps locally (workspace pattern)
- **Metadata**: Auto-generate descriptions and keywords, use git config author
- **Repository**: https://github.com/marioidival/limit

**Research Findings**:
- All crate names available on crates.io (verified)
- All crates missing required metadata (description, authors, license, repository)
- No LICENSE file in project root
- No individual README.md files per crate
- Dependency order: limit-llm + limit-tui (parallel, no deps) → limit-agent (depends on limit-llm) → limit-cli (depends on all)

### Metis Review
**Identified Gaps** (addressed):
- **Git dependencies check**: Added pre-flight verification task
- **crates.io token verification**: Added pre-flight verification task
- **README content strategy**: Use minimal 3-line template with repo link
- **MSRV policy**: Will add rust-version field to Cargo.toml
- **Scope creep risks**: Guardrails added to prevent CI/CD, extensive docs, refactoring
- **Edge cases**: Documented partial publish failure mitigation

---

## Work Objectives

### Core Objective
Publish the entire Limit ecosystem to crates.io so users can install via `cargo install limit-cli` with all dependencies available on crates.io.

### Concrete Deliverables
- LICENSE file (MIT) in project root
- README.md files in each crate directory (limit-llm, limit-agent, limit-tui, limit-cli)
- Updated Cargo.toml files with required metadata
- All 4 crates published to crates.io at version 0.0.18
- Git tag v0.0.18 created and pushed
- Verified installation via `cargo install limit-cli`

### Definition of Done
- [ ] All 4 crates visible on crates.io
- [ ] `cargo install limit-cli --version 0.0.18` succeeds
- [ ] `limit --version` outputs 0.0.18
- [ ] Git tag v0.0.18 exists and pushed
- [ ] All QA scenarios pass

### Must Have
- All required metadata added (description, authors, license, repository, keywords)
- LICENSE file created
- Minimal README per crate
- All crates compile and pass tests
- All crates publish successfully
- Installation verified

### Must NOT Have (Guardrails)
- ❌ Do NOT change version numbers (already at 0.0.18)
- ❌ Do NOT modify existing dependencies
- ❌ Do NOT add new dependencies
- ❌ Do NOT refactor source code (even if clippy suggests)
- ❌ Do NOT add CI/CD workflows
- ❌ Do NOT write extensive documentation beyond minimal READMEs
- ❌ Do NOT add examples or tutorials
- ❌ Do NOT create CHANGELOG unless explicitly requested
- ❌ Do NOT touch any .rs files except for metadata-related comments
- ❌ Do NOT run cargo clippy --fix (only report, don't auto-fix)
- ❌ Do NOT create CI workflows

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES (cargo test)
- **Automated tests**: Tests-after (run existing tests to verify nothing broke)
- **Framework**: cargo test
- **If TDD**: N/A (metadata changes only, no code changes)

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Cargo commands**: Use Bash — Run cargo commands, parse output, assert fields
- **File operations**: Use Bash — Check file existence, verify content
- **API verification**: Use Bash (curl) — Query crates.io API, assert JSON fields

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — pre-flight validation):
├── Task 1: Check for git/external path dependencies [quick]
└── Task 2: Verify crates.io token availability [quick]

Wave 2 (After Wave 1 — metadata preparation):
├── Task 3: Create LICENSE file [quick]
├── Task 4: Add metadata to limit-llm [quick]
├── Task 5: Add metadata to limit-tui [quick]
├── Task 6: Add metadata to limit-agent [quick]
└── Task 7: Add metadata to limit-cli [quick]

Wave 3 (After Wave 2 — path dependencies):
├── Task 8: Add version to path dependencies [quick]

Wave 4 (After Wave 3 — documentation):
├── Task 9: Create README for limit-llm [quick]
├── Task 10: Create README for limit-tui [quick]
├── Task 11: Create README for limit-agent [quick]
└── Task 12: Create README for limit-cli [quick]

Wave 5 (After Wave 4 — validation):
├── Task 13: Run cargo fmt check [quick]
├── Task 14: Run cargo clippy [quick]
├── Task 15: Run cargo test [quick]
└── Task 16: Run cargo publish --dry-run for all crates [quick]

Wave 6 (After Wave 5 — publish, SEQUENTIAL with delays):
├── Task 17: Publish limit-llm [quick]
├── Task 18: Publish limit-tui [quick] (can parallel with 17)
├── Task 19: Publish limit-agent [quick]
└── Task 20: Publish limit-cli [quick]

Wave 7 (After Wave 6 — verification):
├── Task 21: Verify crates on crates.io [quick]
├── Task 22: Verify cargo install limit-cli [quick]
└── Task 23: Create git tag v0.0.18 [quick]

Wave FINAL (After ALL tasks — independent review, 3 parallel):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Metadata quality review (unspecified-high)
└── Task F3: Scope fidelity check (deep)

Critical Path: Task 1,2 → Task 3-7 → Task 8 → Task 9-12 → Task 13-16 → Task 17-20 → Task 21-23 → F1-F3
Parallel Speedup: ~35% faster than sequential
Max Concurrent: 5 (Wave 2)

⚠️ IMPORTANT: Cargo requires path dependencies to also specify version for crates.io publishing.
See: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations
```

### Dependency Matrix

- **1-2**: → 3-7
- **3-7**: → 8 (path deps)
- **8**: → 9-12
- **9-12**: → 13-16
- **13-16**: → 17-20
- **17-18**: → 19 (limit-agent needs limit-llm published)
- **19**: → 20 (limit-cli needs all others published)
- **20**: → 21-23
- **21-23**: → F1-F3

### Agent Dispatch Summary

- **Wave 1**: 2 tasks — T1-T2 → `quick`
- **Wave 2**: 5 tasks — T3-T7 → `quick`
- **Wave 3**: 1 task — T8 → `quick`
- **Wave 4**: 4 tasks — T9-T12 → `quick`
- **Wave 5**: 4 tasks — T13-T16 → `quick`
- **Wave 6**: 4 tasks — T17-T20 → `quick`
- **Wave 7**: 3 tasks — T21-T23 → `quick`
- **FINAL**: 3 tasks — F1 → `oracle`, F2 → `unspecified-high`, F3 → `deep`

**Total Tasks**: 23 implementation + 3 verification = 26

---

## TODOs

- [x] 1. Check for git/external path dependencies

  **What to do**:
  - Grep all Cargo.toml files for `git = "` dependencies
  - Grep all Cargo.toml files for `path = "../../` external path dependencies
  - Report findings to user
  - If any found, user must decide to remove or accept

  **Must NOT do**:
  - Do NOT modify any files
  - Do NOT remove dependencies without user approval

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple grep operations, fast execution
  - **Skills**: []
    - No special skills needed for file grepping

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 2)
  - **Parallel Group**: Wave 1 (with Task 2)
  - **Blocks**: All Wave 2+ tasks if issues found
  - **Blocked By**: None

  **References**:
  - Root Cargo.toml — Workspace structure
  - All crate Cargo.toml files — Check dependency declarations

  **Acceptance Criteria**:
  - [ ] Grep commands executed and output captured
  - [ ] Report provided: git deps found (Y/N), external path deps found (Y/N)
  - [ ] If issues found: user decision documented

  **QA Scenarios**:
  ```
  Scenario: Check for git dependencies
    Tool: Bash
    Steps:
      1. Run: grep -r 'git = "' --include='Cargo.toml' .
      2. Capture output
    Expected Result: Empty output OR list of git dependencies
    Failure Indicators: Command fails
    Evidence: .sisyphus/evidence/task-01-git-deps.txt

  Scenario: Check for external path dependencies
    Tool: Bash
    Steps:
      1. Run: grep -r 'path = "\.\./\.\.' --include='Cargo.toml' .
      2. Capture output
    Expected Result: Empty output OR list of external path deps
    Failure Indicators: Command fails
    Evidence: .sisyphus/evidence/task-01-path-deps.txt
  ```

  **Evidence to Capture**:
  - [ ] task-01-git-deps.txt — Output of git dependency check
  - [ ] task-01-path-deps.txt — Output of path dependency check

  **Commit**: NO

- [x] 2. Verify crates.io token availability

  **What to do**:
  - Check if cargo has crates.io token configured
  - Command: cargo registry token crates-io 2>/dev/null || echo "NEED TO SET: cargo login <token>"
  - If no token: inform user to run cargo login

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 1)
  - **Parallel Group**: Wave 1 (with Task 1)
  - **Blocks**: Wave 5 (publish tasks)
  - **Blocked By**: None

  **References**:
  - Cargo documentation: https://doc.rust-lang.org/cargo/reference/publishing.html

  **QA Scenarios**:
  ```
  Scenario: Verify token exists
    Tool: Bash
    Steps:
      1. Run: cargo registry token crates-io 2>/dev/null
      2. Check exit code and output
    Expected Result: Exit code 0 OR message indicating token needed
    Evidence: .sisyphus/evidence/task-02-token-check.txt
  ```

  **Commit**: NO

- [x] 3. Create LICENSE file (MIT)

  **What to do**:
  - Create LICENSE file in project root with MIT license text
  - Copyright: "2024-2025 Mário Idival"

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 4-7)
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Task 1 (if git deps found)

  **References**:
  - MIT License template: https://opensource.org/licenses/MIT

  **QA Scenarios**:
  ```
  Scenario: Verify LICENSE file created
    Tool: Bash
    Steps:
      1. Run: test -f LICENSE && echo "EXISTS" || echo "MISSING"
      2. Run: head -1 LICENSE
    Expected Result: File exists, first line contains "MIT License"
    Evidence: .sisyphus/evidence/task-03-license.txt
  ```

  **Commit**: NO (part of final commit)

- [x] 4. Add metadata to limit-llm/Cargo.toml

  **What to do**:
  - Add to [package] section:
    - description = "Multi-provider LLM client for Rust with streaming support. Supports Anthropic Claude, OpenAI, and z.ai."
    - authors = ["Mário Idival <marioidival@gmail.com>"]
    - license = "MIT"
    - repository = "https://github.com/marioidival/limit"
    - keywords = ["ai", "llm", "openai", "anthropic", "claude"]
    - categories = ["api-bindings", "development-tools"]
    - readme = "README.md"

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 3, 5, 6, 7)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 15 (dry-run)
  - **Blocked By**: Task 1

  **References**:
  - limit-llm/Cargo.toml — Current metadata
  - crates.io docs: https://doc.rust-lang.org/cargo/reference/manifest.html

  **QA Scenarios**:
  ```
  Scenario: Verify metadata added
    Tool: Bash
    Steps:
      1. Run: cargo metadata --format-version 1 | jq '.packages[] | select(.name=="limit-llm") | {name, description, authors, license}'
    Expected Result: JSON with all fields populated
    Evidence: .sisyphus/evidence/task-04-llm-metadata.json
  ```

  **Commit**: NO

- [x] 5. Add metadata to limit-tui/Cargo.toml

  **What to do**:
  - Same metadata pattern as Task 4
  - description = "Terminal UI components with Virtual DOM rendering for Rust applications. Built with Ratatui."
  - keywords = ["tui", "terminal", "ui", "ratatui", "vdom"]
  - categories = ["command-line-interface", "gui"]

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 3, 4, 6, 7)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 15
  - **Blocked By**: Task 1

  **QA Scenarios**:
  ```
  Scenario: Verify metadata added
    Tool: Bash
    Steps:
      1. Run: cargo metadata --format-version 1 | jq '.packages[] | select(.name=="limit-tui") | {name, description, authors, license}'
    Expected Result: JSON with all fields populated
    Evidence: .sisyphus/evidence/task-05-tui-metadata.json
  ```

  **Commit**: NO

- [x] 6. Add metadata to limit-agent/Cargo.toml

  **What to do**:
  - Same metadata pattern
  - description = "Agent runtime for AI applications with tool registry, parallel execution, and Docker sandbox support."
  - keywords = ["ai", "agent", "automation", "docker", "sandbox"]
  - categories = ["development-tools", "asynchronous"]

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 3, 4, 5, 7)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 15
  - **Blocked By**: Task 1

  **QA Scenarios**:
  ```
  Scenario: Verify metadata added
    Tool: Bash
    Steps:
      1. Run: cargo metadata --format-version 1 | jq '.packages[] | select(.name=="limit-agent") | {name, description, authors, license}'
    Expected Result: JSON with all fields populated
    Evidence: .sisyphus/evidence/task-06-agent-metadata.json
  ```

  **Commit**: NO

- [x] 7. Add metadata to limit-cli/Cargo.toml

  **What to do**:
  - Same metadata pattern
  - description = "AI-powered terminal coding assistant with REPL and TUI. Multi-provider LLM support, session persistence, and built-in tools."
  - keywords = ["cli", "ai", "assistant", "coding", "terminal"]
  - categories = ["command-line-utilities", "development-tools"]

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 3, 4, 5, 6)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 15
  - **Blocked By**: Task 1

  **QA Scenarios**:
  ```
  Scenario: Verify metadata added
    Tool: Bash
    Steps:
      1. Run: cargo metadata --format-version 1 | jq '.packages[] | select(.name=="limit-cli") | {name, description, authors, license}'
    Expected Result: JSON with all fields populated
    Evidence: .sisyphus/evidence/task-07-cli-metadata.json
  ```

  **Commit**: NO

- [x] 8. Add version to path dependencies

  **What to do**:
  - Update all internal workspace path dependencies to include version
  - Changes required:
    - limit-agent/Cargo.toml: `limit-llm = { path = "../limit-llm" }` → `limit-llm = { path = "../limit-llm", version = "0.0.18" }`
    - limit-cli/Cargo.toml: `limit-agent = { path = "../limit-agent" }` → `limit-agent = { path = "../limit-agent", version = "0.0.18" }`
    - limit-cli/Cargo.toml: `limit-llm = { path = "../limit-llm" }` → `limit-llm = { path = "../limit-llm", version = "0.0.18" }`
    - limit-cli/Cargo.toml: `limit-tui = { path = "../limit-tui" }` → `limit-tui = { path = "../limit-tui", version = "0.0.18" }`
  - Changes required:
    - limit-agent/Cargo.toml: `limit-llm = { path = "../limit-llm" }` → `limit-llm = { path = "../limit-llm", version = "0.0.18" }`
    - limit-cli/Cargo.toml: `limit-agent = { path = "../limit-agent" }` → `limit-agent = { path = "../limit-agent", version = "0.0.18" }`
    - limit-cli/Cargo.toml: `limit-llm = { path = "../limit-llm" }` → `limit-llm = { path = "../limit-llm", version = "0.0.18" }`
    - limit-cli/Cargo.toml: `limit-tui = { path = "../limit-tui" }` → `limit-tui = { path = "../limit-tui", version = "0.0.18" }`

  **Why this is needed**:
  - Cargo requires path dependencies to also specify version for crates.io publishing
  - During local development: Cargo uses `path` (workspace member)
  - When publishing: Cargo strips `path` and uses `version` from registry
  - Reference: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (sequential after metadata)
  - **Blocks**: Tasks 9-12 (READMEs need correct deps)
  - **Blocked By**: Tasks 6, 7 (metadata must be added first)

  **References**:
  - limit-agent/Cargo.toml line 14
  - limit-cli/Cargo.toml lines 17, 42, 43
  - Cargo documentation on multiple locations

  **QA Scenarios**:
  ```
  Scenario: Verify path deps have versions
    Tool: Bash
    Steps:
      1. Run: grep -E 'limit-(llm|agent|tui)' limit-agent/Cargo.toml limit-cli/Cargo.toml
    Expected Result: All internal deps have both path and version
    Evidence: .sisyphus/evidence/task-08-path-deps.txt
  ```

  **Commit**: NO

- [x] 9. Create README.md for limit-llm

  **What to do**:
  - Create minimal README with 3-line template:
    """# limit-llm
    Multi-provider LLM client for Rust with streaming support.
    Part of the [Limit](https://github.com/marioidival/limit) ecosystem.
    """

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 10-12)
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 16
  - **Blocked By**: Tasks 4, 8

  **QA Scenarios**:
  ```
  Scenario: Verify README created
    Tool: Bash
    Steps:
      1. Run: test -f limit-llm/README.md && grep -q "github.com/marioidival/limit" limit-llm/README.md
    Expected Result: Exit code 0
    Evidence: .sisyphus/evidence/task-09-llm-readme.txt
  ```

  **Commit**: NO

- [x] 10. Create README.md for limit-tui

  **What to do**:
  - Create minimal README:
    """# limit-tui
    Terminal UI components with Virtual DOM rendering.
    Part of the [Limit](https://github.com/marioidival/limit) ecosystem.
    """

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 8, 10, 11)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 15
  - **Blocked By**: Task 5

  **QA Scenarios**:
  ```
  Scenario: Verify README created
    Tool: Bash
    Steps:
      1. Run: test -f limit-tui/README.md && grep -q "github.com/marioidival/limit" limit-tui/README.md
    Expected Result: Exit code 0
    Evidence: .sisyphus/evidence/task-09-tui-readme.txt
  ```

  **Commit**: NO

- [x] 10. Create README.md for limit-agent

  **What to do**:
  - Create minimal README:
    """# limit-agent
    Agent runtime for AI applications with tool registry and Docker sandbox.
    Part of the [Limit](https://github.com/marioidival/limit) ecosystem.
    """

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 8, 9, 11)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 15
  - **Blocked By**: Task 6

  **QA Scenarios**:
  ```
  Scenario: Verify README created
    Tool: Bash
    Steps:
      1. Run: test -f limit-agent/README.md && grep -q "github.com/marioidival/limit" limit-agent/README.md
    Expected Result: Exit code 0
    Evidence: .sisyphus/evidence/task-10-agent-readme.txt
  ```

  **Commit**: NO

- [x] 11. Create README.md for limit-cli

  **What to do**:
  - Create minimal README:
    """# limit-cli
    AI-powered terminal coding assistant with REPL and TUI.
    Part of the [Limit](https://github.com/marioidival/limit) ecosystem.
    """

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 8-10)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 15
  - **Blocked By**: Task 7

  **QA Scenarios**:
  ```
  Scenario: Verify README created
    Tool: Bash
    Steps:
      1. Run: test -f limit-cli/README.md && grep -q "github.com/marioidival/limit" limit-cli/README.md
    Expected Result: Exit code 0
    Evidence: .sisyphus/evidence/task-11-cli-readme.txt
  ```

  **Commit**: NO

- [ ] 12. Run cargo fmt check

  **What to do**:
  - Run: cargo fmt --all -- --check
  - If formatting issues, report but do NOT auto-fix (guardrails)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 13-15)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 17-20 (publish)
  - **Blocked By**: Tasks 9-12
  **QA Scenarios**:
  ```
  Scenario: Check formatting
    Tool: Bash
    Steps:
      1. Run: cargo fmt --all -- --check
    Expected Result: Exit code 0 OR list of files needing formatting
    Evidence: .sisyphus/evidence/task-12-fmt-check.txt
  ```

  **Commit**: NO

- [ ] 13. Run cargo clippy

  **What to do**:
  - Run: cargo clippy --all-targets -- -D warnings
  - Report any warnings but do NOT run --fix (guardrails)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 12, 14, 15)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 17-20
  - **Blocked By**: Tasks 9-12
  **QA Scenarios**:
  ```
  Scenario: Run clippy
    Tool: Bash
    Steps:
      1. Run: cargo clippy --all-targets -- -D warnings
    Expected Result: Exit code 0 OR warnings reported
    Evidence: .sisyphus/evidence/task-13-clippy.txt
  ```

  **Commit**: NO

- [ ] 14. Run cargo test

  **What to do**:
  - Run: cargo test --workspace
  - All tests must pass before publishing

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 12, 13, 15)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 17-20
  - **Blocked By**: Tasks 9-12
  **QA Scenarios**:
  ```
  Scenario: Run tests
    Tool: Bash
    Steps:
      1. Run: cargo test --workspace
    Expected Result: Exit code 0, all tests pass
    Failure Indicators: Test failures
    Evidence: .sisyphus/evidence/task-14-tests.txt
  ```

  **Commit**: NO

- [ ] 15. Run cargo publish --dry-run for all crates

  **What to do**:
  - Run dry-run for each crate in order:
    - cd limit-llm && cargo publish --dry-run
    - cd limit-tui && cargo publish --dry-run
    - cd limit-agent && cargo publish --dry-run
    - cd limit-cli && cargo publish --dry-run
  - All must succeed before actual publish

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 12-14)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 17-20
  - **Blocked By**: Tasks 9-12
  **QA Scenarios**:
  ```
  Scenario: Dry-run all crates
    Tool: Bash
    Steps:
      1. Run: cd limit-llm && cargo publish --dry-run
      2. Run: cd limit-tui && cargo publish --dry-run
      3. Run: cd limit-agent && cargo publish --dry-run
      4. Run: cd limit-cli && cargo publish --dry-run
    Expected Result: All succeed with exit code 0
    Failure Indicators: Any dry-run failure
    Evidence: .sisyphus/evidence/task-15-dry-run.txt
  ```

  **Commit**: NO

- [ ] 16. Publish limit-llm

  **What to do**:
  - Run: cd limit-llm && cargo publish
  - Wait for publish to complete (check crates.io)
  - Space 30 seconds before next publish

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`git-master`]
    - git-master: For understanding workspace structure

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 17)
  - **Parallel Group**: Wave 5
  - **Blocks**: Task 18 (limit-agent)
  - **Blocked By**: Task 16
  **QA Scenarios**:
  ```
  Scenario: Publish limit-llm
    Tool: Bash
    Steps:
      1. Run: cd limit-llm && cargo publish
      2. Wait 10 seconds
      3. Run: curl -s https://crates.io/api/v1/crates/limit-llm | jq '.crate.latest_version'
    Expected Result: "0.0.18" returned
    Failure Indicators: Publish fails, 404 on API
    Evidence: .sisyphus/evidence/task-16-publish-llm.txt
  ```

  **Commit**: NO

- [ ] 17. Publish limit-tui

  **What to do**:
  - Run: cd limit-tui && cargo publish
  - Wait for publish to complete

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`git-master`]

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 16)
  - **Parallel Group**: Wave 5
  - **Blocks**: None
  - **Blocked By**: Task 16
  **QA Scenarios**:
  ```
  Scenario: Publish limit-tui
    Tool: Bash
    Steps:
      1. Run: cd limit-tui && cargo publish
      2. Wait 10 seconds
      3. Run: curl -s https://crates.io/api/v1/crates/limit-tui | jq '.crate.latest_version'
    Expected Result: "0.0.18" returned
    Evidence: .sisyphus/evidence/task-17-publish-tui.txt
  ```

  **Commit**: NO

- [ ] 18. Publish limit-agent

  **What to do**:
  - Run: cd limit-agent && cargo publish
  - Wait for publish to complete
  - Depends on limit-llm being available

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`git-master`]

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 5 (sequential after 16,17)
  - **Blocks**: Task 20
  - **Blocked By**: Tasks 17, 18
  **QA Scenarios**:
  ```
  Scenario: Publish limit-agent
    Tool: Bash
    Steps:
      1. Run: cd limit-agent && cargo publish
      2. Wait 10 seconds
      3. Run: curl -s https://crates.io/api/v1/crates/limit-agent | jq '.crate.latest_version'
    Expected Result: "0.0.18" returned
    Evidence: .sisyphus/evidence/task-18-publish-agent.txt
  ```

  **Commit**: NO

- [ ] 20. Publish limit-cli

  **What to do**:
  - Run: cd limit-cli && cargo publish
  - Wait for publish to complete
  - Depends on all other crates being available

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`git-master`]

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 6 (sequential after 19)
  - **Blocks**: Tasks 21-23
  - **Blocked By**: Task 19

  **QA Scenarios**:
  ```
  Scenario: Publish limit-cli
    Tool: Bash
    Steps:
      1. Run: cd limit-cli && cargo publish
      2. Wait 10 seconds
      3. Run: curl -s https://crates.io/api/v1/crates/limit-cli | jq '.crate.latest_version'
    Expected Result: "0.0.18" returned
    Evidence: .sisyphus/evidence/task-20-publish-cli.txt
  ```

  **Commit**: NO

- [ ] 21. Verify all crates on crates.io

  **What to do**:
  - Query crates.io API for each crate
  - Verify version 0.0.18 is published
  - Verify metadata is correct

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 22, 23)
  - **Parallel Group**: Wave 7
  - **Blocks**: None
  - **Blocked By**: Task 20

  **QA Scenarios**:
  ```
  Scenario: Verify all crates published
    Tool: Bash
    Steps:
      1. Run: curl -s https://crates.io/api/v1/crates/limit-llm | jq '.crate.latest_version'
      2. Run: curl -s https://crates.io/api/v1/crates/limit-agent | jq '.crate.latest_version'
      3. Run: curl -s https://crates.io/api/v1/crates/limit-tui | jq '.crate.latest_version'
      4. Run: curl -s https://crates.io/api/v1/crates/limit-cli | jq '.crate.latest_version'
    Expected Result: All return "0.0.18"
    Evidence: .sisyphus/evidence/task-21-verify-crates.txt
  ```

  **Commit**: NO

- [ ] 22. Verify cargo install limit-cli

  **What to do**:
  - Run: cargo install limit-cli --version 0.0.18
  - Run: limit --version
  - Verify binary works

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 21, 23)
  - **Parallel Group**: Wave 7
  - **Blocks**: None
  - **Blocked By**: Task 20

  **QA Scenarios**:
  ```
  Scenario: Install and verify binary
    Tool: Bash
    Steps:
      1. Run: cargo install limit-cli --version 0.0.18 --force
      2. Run: limit --version
    Expected Result: "limit 0.0.18" output
    Failure Indicators: Install fails, command not found, wrong version
    Evidence: .sisyphus/evidence/task-22-install-verify.txt
  ```

  **Commit**: NO

- [ ] 23. Create git tag v0.0.18

  **What to do**:
  - Run: git tag -a v0.0.18 -m "Release v0.0.18 - First crates.io publication"
  - Run: git push origin v0.0.18

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`git-master`]
    - git-master: For proper git operations

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 21, 22)
  - **Parallel Group**: Wave 7
  - **Blocks**: None
  - **Blocked By**: Task 20

  **QA Scenarios**:
  ```
  Scenario: Create and push tag
    Tool: Bash
    Steps:
      1. Run: git tag -a v0.0.18 -m "Release v0.0.18"
      2. Run: git push origin v0.0.18
      3. Run: git tag -l "v0.0.18"
    Expected Result: "v0.0.18" returned
    Evidence: .sisyphus/evidence/task-23-tag.txt
  ```

  **Commit**: YES (final commit)
  - Message: `chore: prepare crates for crates.io publishing v0.0.18`
  - Files: LICENSE, limit-llm/Cargo.toml, limit-llm/README.md, limit-agent/Cargo.toml, limit-agent/README.md, limit-tui/Cargo.toml, limit-tui/README.md, limit-cli/Cargo.toml, limit-cli/README.md
  - Pre-commit: `cargo fmt && cargo clippy --all-targets && cargo test --workspace`
---
## Final Verification Wave (MANDATORY — after ALL implementation tasks)

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read plan, verify each "Must Have" exists (read file, curl endpoint). Check "Must NOT Have" absent (grep codebase). Verify evidence files. Compare deliverables.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Metadata Quality Review** — `unspecified-high`
  Run `cargo metadata --format-version 1` for each crate. Check: description present and clear, authors valid, license valid, repository URL correct, keywords relevant, categories appropriate, readme exists, documentation links work.
  Output: `Metadata [N/N complete] | Links [N/N valid] | VERDICT`

- [ ] F3. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", run git diff. Verify only Cargo.toml, LICENSE, README.md files changed. No .rs files modified. No dependencies changed. No version bumps. Flag any unaccounted changes.
  Output: `Files [N/N expected] | Violations [CLEAN/N issues] | VERDICT`

---

## Commit Strategy

- **Single commit after all changes**: `chore: prepare crates for crates.io publishing v0.0.18`
- Files: LICENSE, limit-llm/Cargo.toml, limit-llm/README.md, limit-agent/Cargo.toml, limit-agent/README.md, limit-tui/Cargo.toml, limit-tui/README.md, limit-cli/Cargo.toml, limit-cli/README.md
- Pre-commit: `cargo fmt && cargo clippy --all-targets && cargo test --workspace`

---

## Success Criteria

### Verification Commands
```bash
# All crates published
curl -s https://crates.io/api/v1/crates/limit-llm | jq '.crate.latest_version'
# Expected: "0.0.18"

curl -s https://crates.io/api/v1/crates/limit-agent | jq '.crate.latest_version'
# Expected: "0.0.18"

curl -s https://crates.io/api/v1/crates/limit-tui | jq '.crate.latest_version'
# Expected: "0.0.18"

curl -s https://crates.io/api/v1/crates/limit-cli | jq '.crate.latest_version'
# Expected: "0.0.18"

# Installation works
cargo install limit-cli --version 0.0.18
limit --version
# Expected: limit 0.0.18

# Git tag exists
git tag -l "v0.0.18"
# Expected: v0.0.18
```

### Final Checklist
- [ ] All 4 crates visible on crates.io
- [ ] LICENSE file exists
- [ ] All crates have README.md
- [ ] All crates have complete metadata
- [ ] All tests pass
- [ ] cargo install limit-cli works
- [ ] limit binary runs correctly
- [ ] Git tag v0.0.18 created and pushed
