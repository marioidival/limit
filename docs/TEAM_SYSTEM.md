# Team System — Multi-Agent Orchestration

> **Status:** ✅ Stable  
> **Since:** v0.0.28  
> **ADR:** [limit-team-adr.md](../limit-team-adr.md)

---

## Overview

The team system lets Limit coordinate **specialized AI agents** (PM, TL, Jr) through a
structured workflow to tackle complex, multi-step tasks that a single agent would
struggle with.

Instead of one agent doing everything, responsibilities are split:

| Role | Responsibility | Tools |
|------|---------------|-------|
| **PM** (Product Manager) | Analyze requirements, deliver summary | None |
| **TL** (Tech Lead) | Plan architecture, break down tasks, validate results | `bash` |
| **Jr** (Junior Developer) | Execute tasks with full tool access | All tools |

---

## CLI Usage

### Creating a Team

```bash
/team create --name "feature-team" --juniors 2
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--name` | `-n` | required | Team identifier |
| `--juniors` | `-j` | `2` | Number of Jr agents to spawn |

### Running a Task

```bash
/team start --team "feature-team" --task "Add JWT authentication with refresh tokens"
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--team` | `-t` | required | Which team to use |
| `--task` | `-k` | required | Task description |

Execution runs in a **background thread** so the TUI stays responsive.
Phase-by-phase progress is displayed as the workflow advances.

### Managing Teams

```bash
# List all teams (loads from disk)
/team list

# Show team info
/team status "feature-team"

# View event log with error/warning summary
/team history "feature-team"

# Delete a team (memory + disk)
/team delete "feature-team"
```

---

## Workflow Pipeline

A team execution follows a **6-phase pipeline**:

```
┌─────────────┐    ┌─────────────┐    ┌──────────────┐
│ PM Analysis │───▶│  TL Plan    │───▶│ TL Breakdown │
└─────────────┘    └─────────────┘    └──────┬───────┘
                                               │
                                               ▼
┌─────────────┐    ┌��──────────────┐    ┌────────────┐
│ PM Delivery │◀───│ TL Validation │◀───│ Jr Execute │
└─────────────┘    └───────────────┘    └────────────┘
                                         (parallel)
```

### Phase Details

| Phase | Agent | Input | Output |
|-------|-------|-------|--------|
| **PmAnalysis** | PM | User request | Requirement analysis |
| **TlPlan** | TL | PM analysis | Technical architecture plan |
| **TlBreakdown** | TL | Technical plan | List of `TASK:` lines |
| **JrExecution** | Jr (parallel) | Individual tasks | Task results |
| **TlValidation** | TL | Jr results | Validation feedback |
| **PmDelivery** | PM | TL validation | Final summary for the user |

### Early Exit

If the TL produces no parseable `TASK:` lines, the workflow skips Jr execution
and TL validation, delivering the TL's plan directly as the PM's summary.

### Parallel Task Execution

Jr tasks are distributed **round-robin** across available Jr agents and executed
concurrently using `futures::stream::buffer_unordered`. The concurrency limit
is controlled by `max_parallel_tasks` (default: 4).

Failed Jr tasks are **retried once** before being marked as failed.

---

## Result Structure

After execution, a `TeamResult` is returned:

```rust
pub struct TeamResult {
    pub solution: String,          // PM's delivery summary
    pub duration: Duration,        // Wall-clock time
    pub events: Vec<TeamEvent>,    // Full event log
    pub total_retries: usize,      // Retried LLM calls
    pub failed_tasks: usize,       // Failed task count
    pub total_tasks: usize,        // Total task count
    pub files_modified: Vec<String>, // Extracted from Jr outputs
}
```

### Modified Files Extraction

Files are automatically extracted from Jr task outputs by scanning for JSON
patterns like `{"path": "src/main.rs"}`.

---

## Configuration

All team settings live in `~/.limit/config.toml` under the `[team]` section:

```toml
provider = "anthropic"

[providers.anthropic]
model = "claude-sonnet-4-20250514"

[team]
default_juniors = 2          # Number of Jr agents
max_parallel_tasks = 4       # Max concurrent tasks
enable_streaming = true      # Streaming output

[team.roles]
[team.roles.pm]
tools = []                    # No tools

[team.roles.tl]
tools = ["bash"]              # Can run commands for validation

[team.roles.jr]
provider = "openai"           # Use a different provider for this role
model = "gpt-4o-mini"        # Model on the overridden provider
max_tokens = 8192
# tools = None                # All tools (default)
```

### Per-Role Overrides

| Field | Type | Description |
|-------|------|-------------|
| `provider` | `Option<String>` | Override the LLM provider for this role (e.g. `"openai"`, `"anthropic"`, `"local"`) |
| `model` | `Option<String>` | Model to use. **Required** when `provider` is set. Optional override when using the main provider. |
| `base_url` | `Option<String>` | Custom endpoint URL for the role's provider |
| `max_tokens` | `Option<u32>` | Max tokens override for this role (default: 4096 when `provider` is set) |
| `tools` | `Option<Vec<String>>` | `None` = all tools, `Some([])` = no tools, `Some([...])` = whitelist |

### Per-Role Provider Selection

Each role can use a **different LLM provider** than the main `provider`. This enables
cross-provider mixing — e.g., PM on Claude, TL on GPT-4, Jr on a local model.

When `provider` is set on a role:
- A new provider instance is created independently (no config section needed)
- `model` is **required** (the system will error without it)
- The API key is resolved from environment variables:

  | Provider | Env Var |
  |----------|---------|
  | `anthropic` | `ANTHROPIC_API_KEY` |
  | `openai` | `OPENAI_API_KEY` |
  | `zai` | `ZAI_API_KEY` |
  | `local` / `ollama` / `lmstudio` / `vllm` | Not required |

- Default timeout is 60s
- Default `max_tokens` is 4096 (override with the `max_tokens` field)

When `provider` is **omitted**, the role uses the main provider (cloned) with an
optional `max_tokens` override.

#### Example: Cost-optimized team

```toml
provider = "anthropic"

[providers.anthropic]
model = "claude-sonnet-4-20250514"

[team.roles.pm]
# Uses main provider (Claude Sonnet) — no override needed

[team.roles.tl]
max_tokens = 16384            # Give TL more room for detailed plans

[team.roles.jr]
provider = "openai"           # Cheaper provider for task execution
model = "gpt-4o-mini"
max_tokens = 8192
```

#### Example: Local Jr with remote PM/TL

```toml
provider = "anthropic"

[providers.anthropic]
model = "claude-sonnet-4-20250514"

[team.roles.jr]
provider = "ollama"
model = "llama3"
base_url = "http://localhost:11434"
max_tokens = 4096
```

### Config Parsing Flow

```
config.toml
  └─ [team] section
       └─ Config::team (Option<toml::Value>)
            └─ TeamSection::from_raw()
                 └─ TeamConfig::from_section()
```

This indirection avoids coupling `limit-llm` (which owns the config parser) to
`limit-agent` (which owns the team module).

---

## Architecture

### Module Layout

```
limit-agent/src/team/
├── mod.rs              # Team struct, TeamConfig, re-exports
├── agent.rs            # TeamAgent (LLM + tool execution)
├── role.rs             # Role enum, RoleConfig, TeamSection
├── workflow.rs         # 6-phase pipeline, parallel execution
├── orchestrator.rs     # Task, TaskResult, parse_tasks
├── history.rs          # TeamHistory, TeamEvent, EventLevel
├── persistence.rs      # TeamSnapshot, TeamStore (disk)
└── prompts/
    ├── pm.md            # PM system prompt
    ├── tl.md            # TL system prompt
    └── jr.md            # Jr system prompt
```

### Core Types

#### `TeamAgent`

Wraps an `LlmProvider` + `ToolRegistry` with a role-specific system prompt.

```rust
pub struct TeamAgent {
    role: Role,
    provider: Box<dyn LlmProvider>,
    registry: Arc<ToolRegistry>,
    history: Vec<Message>,
}
```

Key capabilities:
- **`prompt()`** — Full response with automatic tool-call handling and retry
  (up to 3 attempts with exponential backoff: 1s, 2s, 4s).
- **`prompt_stream()`** — Streaming response with tool-call loop.
- **`with_allowed_tools()`** — Constructor that filters the registry to a
  tool whitelist.
- **`clear_history()`** — Reset conversation to system prompt only.

#### `Team`

Top-level orchestrator holding all agents and shared state.

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

- **`execute()`** — Drives the full 6-phase workflow.
- **`events()`** — Read the event log.
- **`reset()`** — Clear all agent histories and events.

#### `Role`

```rust
pub enum Role { PM, TL, Jr }
```

Each role provides:
- `system_prompt()` — loaded from embedded markdown files via `include_str!`
- `label()` — display name
- `config_key()` — TOML key (`"pm"`, `"tl"`, `"jr"`)
- `default_model()` — recommended model
- `default_tools()` — recommended tool whitelist

---

## Error Handling & Retry

### LLM Call Retries

Transient errors (rate limits, timeouts, 500/502/503, connection issues) are
automatically retried up to **3 times** with exponential backoff:

| Attempt | Delay |
|---------|-------|
| 1st retry | 1s |
| 2nd retry | 2s |
| 3rd retry | 4s |

On each retry, the failed assistant message is popped from history so the LLM
gets a clean retry.

### Jr Task Retries

Failed Jr tasks are retried **once**. If the retry also fails, the task is
marked as `success: false` and included in the results summary with the error
message.

### Error Messages

The CLI provides actionable hints for common failures:

| Error Pattern | Hint |
|--------------|------|
| Rate limit / 429 | "Try again in a moment or use a cheaper model for Jr agents" |
| API key | "Check your API key in ~/.limit/config.toml" |
| Timeout | "Try breaking the task into smaller pieces" |

---

## Event History

Every phase transition and agent action is recorded as a `TeamEvent`:

```rust
pub struct TeamEvent {
    pub timestamp: DateTime<Utc>,
    pub role: String,          // "PM", "TL", "Jr", or "system"
    pub action: String,        // "analysis", "plan", "phase:PmAnalysis", etc.
    pub content: String,       // Output or description
    pub level: EventLevel,     // Info, Warn, or Error
}
```

Use `/team history <name>` to view the full event log with a summary header
showing event, error, and warning counts.

---

## Persistence

Teams are persisted as JSON snapshots to `~/.limit/teams/`:

```
~/.limit/teams/
├── feature-team.json
├── backend-team.json
└── frontend-team.json
```

### `TeamSnapshot`

```rust
pub struct TeamSnapshot {
    pub name: String,
    pub config: TeamConfig,
    pub history: TeamHistory,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub run_count: usize,
}
```

### `TeamStore`

Disk-backed storage with CRUD operations:

| Method | Description |
|--------|-------------|
| `save(snapshot)` | Write to `<name>.json` |
| `load(name)` | Read from disk (returns `Option`) |
| `delete(name)` | Remove from disk |
| `list()` | List all team names |
| `exists(name)` | Check if snapshot exists |

Team names with `/` or `\` are automatically sanitized to `_` for safe filenames.

---

## Integration Test Suite

The `team_integration` test module uses `MockLlmProvider` to test the full
workflow without real LLM calls. The mock returns pre-configured responses for
each phase.

### Test Coverage

| Test | Validates |
|------|-----------|
| `test_full_team_workflow` | End-to-end execution, file extraction, task counting |
| `test_workflow_phases_in_order` | All 6 phases execute in correct order |
| `test_workflow_records_pm_analysis_event` | PM events (analysis, delivery) |
| `test_workflow_records_tl_events` | TL events (plan, tasks, validation) |
| `test_workflow_records_jr_events` | Jr execution events |
| `test_workflow_duration_is_positive` | Non-zero duration |
| `test_workflow_empty_tasks_returns_early` | Early exit with no parseable tasks |
| `test_team_create_and_events` | Team structure (roles, junior count) |
| `test_team_reset_clears_state` | Reset clears all history |
| `test_team_persistence_save_load` | Snapshot serialization round-trip |

---

## Performance

For a typical task with N subtasks:

| Phase | Latency | Parallel? |
|-------|---------|-----------|
| PM analysis | 2–5s | No |
| TL plan | 2–5s | No |
| TL breakdown | 1–3s | No |
| Jr execution | 5–20s total | **Yes** |
| TL validation | 2–5s | No |
| PM delivery | 1–3s | No |
| **Total** | **15–40s** | — |

Total LLM calls: **5 + N** (N calls run in parallel).

---

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| No external framework (Rig, LangChain) | Keep Limit lightweight, full control over implementation |
| `std::sync::Mutex` for team storage | Avoids TUI crash from `tokio::sync::RwLock` in Command trait |
| `tokio::sync::Mutex` for Jr agents | Async-aware lock avoids `await_holding_lock` clippy warning |
| `NoopProvider` for pre-start teams | `list`/`status` work without a real LLM provider |
| Background thread for execution | Non-blocking TUI via `std::thread::spawn` + `tokio::runtime::Runtime` |
| Opaque `team` in config | Decouples `limit-llm` config from `limit-agent` team types |
| Per-role tool whitelists | PM shouldn't edit files; Jr needs full access |

---

## Programmatic API

```rust
use limit_agent::team::{Team, TeamConfig};
use limit_agent::ToolRegistry;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = make_provider(); // Box<dyn LlmProvider>
    let tools = Arc::new(ToolRegistry::new());
    let config = TeamConfig::default();

    let mut team = Team::new("my-team".into(), provider, config, tools)?;
    let result = team.execute("Add JWT authentication").await?;

    println!("Solution: {}", result.solution);
    println!("Tasks: {}/{} succeeded", 
        result.total_tasks - result.failed_tasks, 
        result.total_tasks
    );
    println!("Duration: {:.1}s", result.duration.as_secs_f64());
    
    // Inspect events
    for event in &result.events {
        if event.level == limit_agent::EventLevel::Error {
            eprintln!("[ERROR] {}: {}", event.action, event.content);
        }
    }

    Ok(())
}
```

---

## Future Enhancements

- [ ] Dynamic role assignment
- [ ] Task dependencies (DAG execution)
- [ ] Agent-to-agent communication protocols
- [ ] Cost tracking / token counting per role
- [ ] Pre-configured team templates (frontend, backend, devops)
- [ ] Custom role definitions
- [ ] Team sharing across sessions
- [ ] End-to-end integration tests with real LLM providers

---

## See Also

- [ADR-001: Multi-Agent Team System](../limit-team-adr.md)
- [Configuration Guide](CONFIGURATION.md)
- [Tool System](limit-agent.md#tool-system)
- [Limit LLM Providers](limit-llm.md)
