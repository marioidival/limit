# ADR: Team Command Implementation

> **Status:** ✅ Completed  
> **Date:** 2026-03-16  
> **Last Updated:** 2026-03-16  
> **Decision Makers:** Mário Idival  
> **ADR Number:** 001  

---

## Context and Problem Statement

### Current Situation

Limit is a Rust-based AI pair programmer CLI with:
- **17 built-in tools** (file I/O, bash, git, etc.)
- **Tool trait abstraction** (`limit-agent/src/tool.rs`)
- **Command system** (`limit-cli/src/tui/commands/`)
- **Multi-provider LLM support** (`limit-llm`)
- **Session persistence**
- **TUI + REPL modes**

### Problem

Users need to execute complex, multi-step tasks that require:
1. **Requirement analysis** (product vision)
2. **Technical planning** (architecture)
3. **Task breakdown** (specific implementation steps)
4. **Parallel execution** (multiple changes simultaneously)
5. **Validation** (testing, review)

Currently, a single agent handles all of this, which leads to:
- ❌ **Cognitive overload** (agent tries to do everything)
- ❌ **Sequential bottleneck** (no parallel execution)
- ❌ **No role specialization** (generic responses)
- ❌ **Harder debugging** (unclear which step failed)

### Proposed Solution

Implement a **`team` command** that creates a multi-agent system with specialized roles:

```
/team create --name "feature-team" --juniors 2
/team start --team "feature-team" --task "Add JWT authentication"
```

**Roles:**
- **PM** (Product Manager): Understands requirements, product vision
- **TL** (Tech Lead): Architecture, task breakdown, validation
- **Jr** (Junior Developer): Executes tasks with tools

---

## Decision

### ADR-001: Implement Multi-Agent Team System

**Status:** ✅ Implemented

**Decision:** Implement a `team` command using **Limit's existing architecture** (Tool trait, ToolRegistry, LLM providers) **WITHOUT external dependencies** (no Rig, LangChain, etc.).

**Rationale:**
1. ✅ **No new dependencies** (keep Limit lightweight)
2. ✅ **Consistent patterns** (use existing Tool trait)
3. ✅ **Full control** (own implementation, easier debugging)
4. ✅ **Lower learning curve** (team members already know Limit's code)
5. ✅ **Performance** (no framework overhead)

---

## Architecture Design

### 1. **Component Structure**

```
limit/
├── limit-cli/
│   └── src/
│       └── tui/
│           └── commands/
│               ├── mod.rs             # UPDATED: registered TeamCommand
│               ├── team.rs            # NEW: Team command (CLI integration)
│               └── builtin.rs         # UPDATED: /help includes team cmds
├── limit-agent/
│   └── src/
│       ├── team/
│       │   ├── mod.rs              # NEW: Team struct, TeamConfig, re-exports
│       │   ├── agent.rs            # NEW: TeamAgent (LLM streaming + tools)
│       │   ├── role.rs             # NEW: Role enum, RoleConfig, TeamSection
│       │   ├── workflow.rs         # NEW: execute_workflow pipeline
│       │   ├── orchestrator.rs     # NEW: Task, TaskResult, parse_tasks
│       │   ├── history.rs          # NEW: TeamHistory, TeamEvent, EventLevel
│       │   ├── persistence.rs      # NEW: TeamSnapshot, TeamStore (disk)
│       │   └── prompts/
│       │       ├── pm.md           # NEW: PM system prompt
│       │       ├── tl.md           # NEW: TL system prompt
│       │       └── jr.md           # NEW: Jr system prompt
│       ├── registry.rs             # UPDATED: added register_arc()
│       ├── error.rs                # UPDATED: added TeamError, LlmError
│       └── lib.rs                  # UPDATED: exports team module
└── limit-llm/
    └── src/
        └── config.rs               # UPDATED: team_raw field
```

---

### 2. **Core Abstractions**

#### **2.1 Role Enum** (`limit-agent/src/team/role.rs`)

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Role {
    PM,  // Product Manager
    TL,  // Tech Lead
    Jr,  // Junior Developer
}
```

Each role provides:
- `system_prompt()` — embedded via `include_str!("prompts/<role>.md")`
- `label()` — display name
- `default_model()` — recommended model (`gpt-4` for PM/TL, `gpt-4o-mini` for Jr)
- `default_tools()` — tool whitelist per role
- `config_key()` — TOML key for `[team.roles.<key>]`

#### **2.2 TeamAgent** (`limit-agent/src/team/agent.rs`)

Wraps an `LlmProvider` + `ToolRegistry` with role-specific system prompts.

```rust
pub struct TeamAgent {
    role: Role,
    provider: Box<dyn LlmProvider>,
    registry: Arc<ToolRegistry>,
    history: Vec<Message>,
}
```

Key methods:
- `new()` / `with_allowed_tools()` — create with optional tool whitelist
- `prompt()` — full response with automatic tool-call handling + retry (max 3, exponential backoff)
- `prompt_stream()` — streaming response with tool-call handling
- `clear_history()` — reset to system prompt only

#### **2.3 Team Struct** (`limit-agent/src/team/mod.rs`)

```rust
pub struct Team {
    pub name: String,
    pub pm: TeamAgent,
    pub tl: TeamAgent,
    pub jrs: Vec<TeamAgent>,
    pub history: Arc<RwLock<TeamHistory>>,
    pub config: TeamConfig,
}
```

- `Team::new()` — sync constructor, takes `Box<dyn LlmProvider>` + `Arc<ToolRegistry>`
- `Team::execute()` — delegates to `execute_workflow()`
- `Team::events()` / `Team::reset()` — history management

#### **2.4 Workflow Pipeline** (`limit-agent/src/team/workflow.rs`)

```
PM Analysis → TL Plan → TL Breakdown → Jr Parallel Execution → TL Validation → PM Delivery
```

- `execute_workflow()` — free function driving all 6 phases
- `execute_tasks_parallel()` — `futures::stream::buffer_unordered` with round-robin Jr assignment
- Jr tasks retry once on failure
- Graceful fallback if TL produces no parseable tasks

#### **2.5 History & Persistence**

- `TeamHistory` — append-only event log with `EventLevel` (Info/Warn/Error)
- `TeamEvent` — timestamp, role, action, content, level
- `TeamSnapshot` — serializable snapshot for disk storage
- `TeamStore` — JSON file storage in `~/.limit/teams/`

---

### 3. **Command Implementation** (`limit-cli/src/tui/commands/team.rs`)

```rust
pub struct TeamCommand {
    teams: Mutex<HashMap<String, TeamEntry>>,
    store: Mutex<TeamStore>,
}
```

Subcommands:
| Subcommand | Description |
|---|---|
| `create --name <n> [--juniors N]` | Register a new team |
| `delete <name>` | Remove a team (memory + disk) |
| `list` | Show all teams (loads from disk) |
| `status <name>` | Show team info |
| `start --team <n> --task <desc>` | Execute task via background thread |
| `history <name>` | Show event log with error/warning summary |

Key design decisions:
- Teams stored in `std::sync::Mutex` (not `tokio::sync::RwLock`) to avoid TUI crash
- `create` registers with a `NoopProvider`; `start` re-creates with real provider + tools
- Background execution via `std::thread::spawn` + `tokio::runtime::Runtime`
- Error messages include actionable hints (rate limit, auth, timeout)

#### **3.1 Command Registration** (`limit-cli/src/tui/commands/mod.rs`)

```rust
pub fn create_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    // ... other commands ...
    registry.register(Box::new(TeamCommand::new()));
    registry
}
```

#### **3.2 Help Integration** (`limit-cli/src/tui/commands/builtin.rs`)

`/help` output includes:
```
/team create --name <n> [--juniors N] - Create a team
/team start --team <n> --task <desc>  - Execute a team task
/team list|status|history|delete <n>  - Manage teams
```

---

### 4. **Configuration**

```toml
# ~/.limit/config.toml

[team]
default_juniors = 2
max_parallel_tasks = 4
enable_streaming = true

[team.roles]
[team.roles.pm]
tools = []                          # no tools

[team.roles.tl]
tools = ["bash"]                    # model = "gpt-4" (optional)

[team.roles.jr]
model = "gpt-4o-mini"              # optional model override
# tools = None                      # None = all tools
```

Parsing: `Config::team_raw` (`Option<toml::Value>`) → `TeamSection::from_raw()` → `TeamConfig::from_section()`

---

## Implementation Progress

### ✅ Phase 1: Core Infrastructure — COMPLETED

| Item | Status |
|------|--------|
| Create `limit-agent/src/team/` module | ✅ Done |
| Implement `Role` enum (PM/TL/Jr) with `Copy`, `Display`, serialization | ✅ Done |
| Implement `RoleConfig`, `TeamSection`, `TeamRolesSection` | ✅ Done |
| Implement `TeamAgent` struct (LLM streaming + tool execution) | ✅ Done |
| Implement `Task`, `TaskResult`, `TaskStatus`, `parse_tasks` | ✅ Done |
| Implement `TeamHistory`, `TeamEvent`, `EventLevel` (Info/Warn/Error) | ✅ Done |
| Implement `Team` struct + `TeamConfig` | ✅ Done |
| Implement `execute_workflow` pipeline (6 phases) | ✅ Done |
| Prompt templates (PM/TL/Jr system prompts) | ✅ Done |
| `TeamSnapshot` + `TeamStore` (persistence to `~/.limit/teams/`) | ✅ Done |
| Unit tests for all team module types | ✅ Done |

### ✅ Phase 2: Command Integration — COMPLETED

| Item | Status |
|------|--------|
| Implement `TeamCommand` in `limit-cli` | ✅ Done |
| Subcommands: create, delete, list, status, start, history | ✅ Done |
| Integrate with existing `CommandRegistry` | ✅ Done |
| Add command help text + `/help` integration | ✅ Done |
| `std::sync::Mutex` for team storage (TUI-safe) | ✅ Done |
| Background team execution (non-blocking TUI) | ✅ Done |
| `NoopProvider` placeholder for pre-start teams | ✅ Done |
| `build_tool_registry()` — same tools as main agent | ✅ Done |
| Unit tests for TeamCommand | ✅ Done |

### ✅ Phase 3: Advanced Features — COMPLETED

| Item | Status |
|------|--------|
| Parallel task execution via `buffer_unordered` | ✅ Done |
| `TeamHistory` event logging | ✅ Done |
| Streaming output (`prompt_stream` + `LlmProvider` streaming) | ✅ Done |
| Config-driven team setup (`[team]` section in `config.toml`) | ✅ Done |
| Per-role tool whitelists (`TeamAgent::with_allowed_tools()`) | ✅ Done |
| Per-role model override (`RoleConfig::model`) | ✅ Done |
| `TeamConfig::from_section()` — build from parsed TOML | ✅ Done |
| `ToolRegistry::register_arc()` — share tools via `Arc` | ✅ Done |
| Error recovery: exponential backoff (3 retries, 1s/2s/4s) | ✅ Done |
| Jr task retry-once on failure | ✅ Done |
| `EventLevel` for history events | ✅ Done |
| `TeamResult` stats: `failed_tasks`, `total_tasks` | ✅ Done |

### ✅ Phase 4: Polish & Testing — COMPLETED

| Item | Status |
|------|--------|
| Config section `[team]` in `limit-llm::Config` (`team_raw`) | ✅ Done |
| `RoleConfig`, `TeamSection`, `TeamRolesSection` types | ✅ Done |
| `toml = "0.8"` dependency in `limit-agent` | ✅ Done |
| Improved error messages with actionable hints | ✅ Done |
| `/help` includes team commands | ✅ Done |
| `/team history` shows summary header with error/warning counts | ✅ Done |
| `AgentError::TeamError` + `AgentError::LlmError` variants | ✅ Done |

### Build Status

```
✅ cargo check — clean (0 errors, 0 warnings)
```

---

## Divergences from Original Design

| Aspect | Original Design | Actual Implementation | Reason |
|--------|----------------|----------------------|--------|
| `Team::new()` | `async fn`, takes `Vec<Arc<dyn Tool>>` | Sync `fn`, takes `Arc<ToolRegistry>` | Consistency with existing architecture |
| Tool filtering | `filter_tools()` helper | `TeamAgent::with_allowed_tools()` + `ToolRegistry::register_arc()` | Cleaner encapsulation |
| Config format | `[team.models]` separate section | `[team.roles.<role>]` with `model` + `tools` | Simpler, single section per role |
| Config parsing | Direct deserialization in `Config` | Opaque `team_raw` + `TeamSection::from_raw()` | Avoids coupling `limit-llm` to `limit-agent` |
| `Role` variants | `Clone` only | `Copy` | Enums without data should be `Copy` |
| Command storage | `tokio::sync::RwLock` | `std::sync::Mutex` | Fixed TUI crash |
| Tool calls | `[TOOL_CALL: name {args}]` regex | Standard LLM function calling | Leverages existing tool infrastructure |
| Persistence | Not specified | `TeamSnapshot` + `TeamStore` (JSON, `~/.limit/teams/`) | Teams survive CLI restarts |

---

## Performance Considerations

| Operation | Calls | Optimization |
|-----------|-------|--------------|
| PM analysis | 1 | — |
| TL plan | 1 | — |
| TL breakdown | 1 | — |
| Jr execution | N (parallel) | `buffer_unordered` |
| TL validation | 1 | — |
| PM delivery | 1 | — |
| **Total** | **5 + N** | N in parallel |

| Phase | Latency | Parallel? |
|-------|---------|-----------|
| PM analysis | 2-5s | No |
| TL plan | 2-5s | No |
| TL breakdown | 1-3s | No |
| Jr execution | 5-20s total | **Yes** |
| TL validation | 2-5s | No |
| PM delivery | 1-3s | No |
| **Total** | **15-40s** | — |

---

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **LLM rate limits** | High | Medium | Exponential backoff (3 retries, 1s/2s/4s) |
| **Tool execution failures** | Medium | Medium | Retry logic, error handling |
| **Parallel execution race conditions** | Medium | Low | `std::sync::Mutex` around Jr agents |
| **Memory leaks (long-running teams)** | Low | Low | `Team::reset()`, history `clear()` |
| **Conflicting task assignments** | Medium | Medium | TL coordinates, Jr reports conflicts |

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Task completion rate** | >95% | Successful task executions |
| **Time to completion** | <60s for simple tasks | Average execution time |
| **Error rate** | <5% | Failed executions |
| **Parallelization efficiency** | >80% | Speedup vs sequential |

---

## Future Enhancements

### Phase 5: Advanced Orchestration

- [ ] Dynamic role assignment
- [ ] Task dependencies (DAG execution)
- [ ] Agent communication protocols
- [ ] Learning from past executions
- [ ] Cost tracking / token counting per role

### Phase 6: Team Templates

- [ ] Pre-configured team types (frontend, backend, devops)
- [ ] Custom role definitions

### Phase 7: Collaboration

- [ ] Multiple users per team
- [ ] Team sharing
- [ ] Remote team execution

### Phase 8: Quality

- [ ] Integration tests (E2E with real LLM)
- [ ] Documentation in README
- [ ] Example workflows

---

## References

- **Limit Repository:** https://github.com/marioidival/limit
- **Tool Trait:** `limit-agent/src/tool.rs`
- **Command System:** `limit-cli/src/tui/commands/`
- **LLM Providers:** `limit-llm/src/providers/`

---

## Approval

| Role | Name | Date | Status |
|------|------|------|--------|
| Author | OpenClaw | 2026-03-16 | ✅ Implemented |
| Reviewer | Mário Idival | - | Pending |
| Approver | Mário Idival | - | Pending |

---

**End of ADR**
