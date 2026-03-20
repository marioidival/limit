# Pre-warm TLDR on Startup

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:writing-plans to create implementation plan.

**Goal:** Run TLDR warm() on startup instead of lazily on first tool call, with smart cache detection to skip warm when nothing changed.

**Approach:** Hash-based project detection — compare saved project metadata against current state to decide whether to warm or skip.

---

## How It Works

1. On agent activation, `TldrTool::new()` spawns `pre_warm()` in background
2. `pre_warm()` checks `WarmGuard::is_fresh()` — compares project path + file count
3. Fresh (< 5 min, same file count) → skip warm entirely
4. Not fresh → `TLDR::warm()` in background, save metadata, notify waiters
5. `get_tldr()` returns cached instance or waits for background warm to finish

## Components

### 1. WarmGuard (new, in limit-agent)

```rust
struct WarmMeta {
    project_path: String,
    file_count: usize,
    timestamp: u64,
}
```

- `meta_path()` → `~/.limit/projects/<hash>/tldr/cache/.warm_meta`
- `is_fresh(project_path) -> bool`
  - Load meta, compare project_path
  - If timestamp < 5 min → return true (skip walkdir)
  - Else → walkdir count files, compare with saved count
- `save(project_path, file_count)` → serialize meta to disk

### 2. TldrTool (modified, in limit-agent)

- New field: `warm_handle: Arc<tokio::sync::Notify>`
- Constructor: `tokio::spawn(pre_warm(project_path, notify.clone()))`
- `pre_warm()`:
  - Check `WarmGuard::is_fresh()` → skip if fresh
  - `TLDR::new()` + `warm()`
  - `WarmGuard::save(project_path, file_count)`
  - `notify.notify_waiters()`
- `get_tldr()`:
  - OnceCell ready → return `Arc<TLDR>`
  - Still warming → `notify.notified().await` → return `Arc<TLDR>`

### 3. No changes to limit-tldr

All pre-warm logic lives in TldrTool. The TLDR library API is unchanged.

## Error Handling

- `pre_warm()` fails → log warning, OnceCell stays empty
- Next `get_tldr()` falls back to lazy creation (current behavior)
- `WarmGuard::is_fresh()` fails (corrupt meta) → treat as not fresh

## Smart Detect Logic

| Condition | Action |
|-----------|--------|
| Meta file missing | Warm |
| project_path different | Warm + save new meta |
| timestamp < 5 min | Skip (trust recent warm) |
| timestamp >= 5 min, file_count matches | Skip |
| timestamp >= 5 min, file_count differs | Warm + save new meta |

## Tasks

### Task 1: Create WarmGuard

**Files:**
- Create: `limit-agent/src/tools/warm_guard.rs`

**Steps:**
1. Define `WarmMeta` struct with serde
2. Implement `is_fresh(project_path) -> bool`
3. Implement `save(project_path, file_count)`
4. Implement file counting via walkdir (reuse exclusion logic from ParseCoordinator)
5. Add test
6. Run tests

### Task 2: Modify TldrTool for pre-warm

**Files:**
- Modify: `limit-agent/src/tools/tldr.rs`

**Steps:**
1. Add `warm_handle: Arc<tokio::sync::Notify>` field
2. Implement `pre_warm()` async function
3. Call `tokio::spawn(pre_warm())` in constructor
4. Update `get_tldr()` to wait on notify if OnceCell not ready
5. Add test
6. Run tests
