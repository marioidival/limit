# Token Efficiency Design for Limit

> Investigation of pi-mono's token optimization techniques and how to apply them to Limit.

## Executive Summary

pi-mono achieves **40-70% token reduction** through three main strategies:

1. **Context Compaction** - Summarizes old messages while keeping recent context
2. **Prompt Caching (API-level)** - Reduces input token costs via Anthropic/OpenAI caching
3. **Tree-based Sessions** - O(1) branching with full history preserved in JSONL

This document explains each technique and provides a roadmap for implementation in Limit.

---

## Table of Contents

1. [Current Limit Architecture](#1-current-limit-architecture)
2. [pi-mono Token Efficiency Techniques](#2-pi-mono-token-efficiency-techniques)
3. [Implementation Roadmap for Limit](#3-implementation-roadmap-for-limit)
4. [Data Structures](#4-data-structures)
5. [Code Reference](#5-code-reference)

---

## 1. Current Limit Architecture

### Session Management

**Location**: `limit-cli/src/session.rs`

```
Storage: SQLite (~/.limit/session.db) + Binary files (~/.limit/sessions/*.bin)
Structure: Linear message list (no tree/branching)
Token Tracking: Accumulates counts but doesn't enforce limits
```

**Current flow**:
```
User message → Append to Vec<Message> → Save to .bin file
Session restore → Load ALL messages from .bin → Pass to LLM
```

**Problem**: No truncation or summarization - full history sent to LLM every time.

### LLM Integration

**Location**: `limit-llm/src/`

```
Providers: AnthropicClient, OpenAiProvider, ZaiProvider, LocalProvider
Flow: Vec<Message> → Provider.send() → Streaming response
```

**Message types** (`limit-llm/src/types.rs`):
```rust
pub struct Message {
    pub role: Role,           // User, Assistant, System, Tool
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}
```

### Existing Token Management

| Component | Location | Status |
|-----------|----------|--------|
| `ModelHandoff` | `limit-llm/src/handoff.rs` | **EXISTS but UNUSED** |
| `TrackingDb` | `limit-llm/src/tracking.rs` | Records stats only |
| TLDR counter | `limit-tldr/src/utils/token_counter.rs` | Simple heuristic (inaccurate) |

**Key finding**: `ModelHandoff::compact_messages()` already exists with:
- `tiktoken_rs::cl100k_base` tokenizer
- Message compaction logic
- Token counting utilities

But it's **not integrated** into the main CLI flow!

---

## 2. pi-mono Token Efficiency Techniques

### 2.1 Context Compaction

**How it works**:
1. Walk backwards from newest messages
2. Accumulate token estimates until `>= keepRecentTokens` (default: 20k)
3. Find valid "cut point" (user or assistant message, never tool result)
4. Summarize everything before cut point via LLM
5. Replace old messages with summary message

**Token savings**: 50k tokens → 20k (kept) + 0.5k (summary) = **60% reduction**

**Cut point algorithm** (`compaction.ts:379-441`):
```typescript
function findCutPoint(
  entries: SessionEntry[],
  startIndex: number,
  endIndex: number,
  keepRecentTokens: number,
): CutPointResult {
  // Walk backwards, accumulating token estimates
  let accumulatedTokens = 0;
  for (let i = endIndex - 1; i >= startIndex; i--) {
    accumulatedTokens += estimateTokens(entry.message);
    if (accumulatedTokens >= keepRecentTokens) {
      // Find closest valid cut point
      cutIndex = findValidCutPoint(i);
      break;
    }
  }
  return { firstKeptEntryIndex: cutIndex, isSplitTurn, turnStartIndex };
}
```

**Token estimation** (`compaction.ts:225-283`):
```typescript
function estimateTokens(message: AgentMessage): number {
  // Uses chars/4 heuristic (conservative, overestimates)
  let chars = content.length;
  return Math.ceil(chars / 4);
  // Images: ~1200 tokens
}
```

**Summarization prompts**:

**Initial summary**:
```markdown
## Goal
[What is the user trying to accomplish?]

## Constraints & Preferences
- [Any constraints or requirements]

## Progress
### Done
- [x] [Completed tasks]

### In Progress
- [ ] [Current work]

### Blocked
- [Issues preventing progress]

## Key Decisions
- **[Decision]**: [Brief rationale]

## Next Steps
1. [Ordered list of what should happen next]

## Critical Context
- [Data, examples, or references needed]
```

**Update summary** (iterative):
```markdown
Update the existing structured summary with new information. RULES:
- PRESERVE all existing information
- ADD new progress, decisions, and context
- UPDATE Progress: move items from "In Progress" to "Done" when completed
- UPDATE "Next Steps" based on what was accomplished
```

### 2.2 Prompt Caching (API-level)

**Anthropic** (`anthropic.ts`):
```typescript
// Add cache_control to messages
{
  role: "user",
  content: [{ type: "text", text: "...", cache_control: { type: "ephemeral", ttl: "1h" } }]
}

// Applied to:
// 1. System prompt (last text block)
// 2. Last user message (text/image/tool_result)
```

**OpenAI** (`openai-responses.ts`):
```typescript
{
  prompt_cache_key: sessionId,        // Session-based caching
  prompt_cache_retention: "24h"       // When PI_CACHE_RETENTION=long
}
```

**Cache retention settings**:

| Setting | Anthropic | OpenAI |
|---------|-----------|--------|
| `short` (default) | 5 min | in-memory |
| `long` (`PI_CACHE_RETENTION=long`) | 1 hour | 24 hours |

**Usage tracking**:
```typescript
interface Usage {
  input: number;
  output: number;
  cacheRead: number;    // Tokens read from cache (cheap!)
  cacheWrite: number;   // Tokens written to cache
  totalTokens: number;
}
```

**Cost savings**: Cache read tokens cost ~10% of input tokens.

### 2.3 Tree-based Session Structure

**Entry structure** (`session-manager.ts`):
```typescript
interface SessionEntryBase {
  type: string;
  id: string;           // 8-char hex ID
  parentId: string | null;  // Tree structure!
  timestamp: string;
}
```

**Entry types**:

| Type | Purpose | In LLM Context? |
|------|---------|-----------------|
| `message` | User/assistant/tool message | Yes |
| `compaction` | Summary of old messages | Yes (as summary message) |
| `branch_summary` | Context when switching branches | Yes |
| `model_change` | Model switch mid-session | No |
| `custom` | Extension state | No |
| `custom_message` | Extension message | Yes |

**Context building** (`buildSessionContext`):
```typescript
function buildSessionContext(entries, leafId) {
  // 1. Walk from leaf to root using parentId chain
  const path = [];
  let current = leaf;
  while (current) {
    path.unshift(current);
    current = byId.get(current.parentId);
  }
  
  // 2. Handle compaction: emit summary + skip old messages
  if (compaction) {
    messages.push(createCompactionSummaryMessage(compaction.summary));
    // Only emit messages from firstKeptEntryId onwards
  }
  
  return { messages, thinkingLevel, model };
}
```

**Branching** = O(1) operation:
- Just move `leafId` pointer to earlier entry
- Next append creates child of that entry
- Full history preserved in JSONL

**JSONL format**:
```jsonl
{"type":"session","version":3,"id":"uuid","timestamp":"...","cwd":"..."}
{"type":"message","id":"a1b2c3d4","parentId":null,"timestamp":"...","message":{...}}
{"type":"message","id":"b2c3d4e5","parentId":"a1b2c3d4","timestamp":"...","message":{...}}
{"type":"compaction","id":"c3d4e5f6","parentId":"b2c3d4e5","summary":"...","firstKeptEntryId":"b2c3d4e5"}
{"type":"message","id":"d4e5f6g7","parentId":"c3d4e5f6","timestamp":"...","message":{...}}
```

### 2.4 File Operation Tracking

Tracks which files were read/modified across conversation:

```typescript
interface FileOperations {
  read: Set<string>;
  written: Set<string>;
  edited: Set<string>;
}

// Extracted from tool calls in messages
function extractFileOpsFromMessage(message, fileOps) {
  if (block.name === "read") fileOps.read.add(path);
  if (block.name === "write") fileOps.written.add(path);
  if (block.name === "edit") fileOps.edited.add(path);
}

// Appended to summary
<read-files>
src/main.rs
src/lib.rs
</read-files>

<modified-files>
src/main.rs
</modified-files>
```

---

## 3. Implementation Roadmap for Limit

### Phase 1: Integrate Existing Compaction ✅ COMPLETED

**Status**: Implemented in v0.0.40+

**What was done**:
1. Added `CompactionSettings` struct to `limit-llm/src/config.rs`:
   - `enabled: bool` (default: true)
   - `reserve_tokens: u32` (default: 16384)
   - `keep_recent_tokens: u32` (default: 20000)

2. Integrated `ModelHandoff::compact_messages()` in `agent_bridge.rs`:
```rust
if self.config.compaction.enabled {
    let context_window: usize = 200_000;
    let target_tokens = context_window.saturating_sub(self.config.compaction.reserve_tokens as usize);
    let current_tokens = self.handoff.count_total_tokens(_messages);
    
    if current_tokens > target_tokens {
        let compacted = self.handoff.compact_messages(_messages, target_tokens);
        *_messages = compacted;
    }
}
```

3. Configuration in `~/.limit/config.toml`:
```toml
[compaction]
enabled = true
reserve_tokens = 16384
```

4. Removed old message-count-based `truncate_context()` function

**Effort**: ~1 hour

### Phase 2: Add Summarization (Medium) - NOT STARTED

**Goal**: Replace truncated messages with LLM-generated summary.

**New components**:
```rust
// limit-llm/src/compaction.rs
pub struct CompactionResult {
    pub summary: String,
    pub first_kept_message_id: usize,
    pub tokens_before: u64,
}

pub async fn generate_summary(
    messages: &[Message],
    model: &dyn LlmProvider,
    previous_summary: Option<&str>,
) -> Result<String, LlmError>;
```

**Summary message format**:
```rust
Message {
    role: Role::User,  // or custom "summary" role
    content: Some(summary_text),
    tool_calls: None,
    tool_call_id: None,
}
```

**Effort**: 1-2 days

### Phase 3: Tree-based Sessions ✅ COMPLETED

**Status**: Implemented in v0.0.40+

**What was done**:
1. Created `limit-cli/src/session_tree.rs` with:
   - `SessionEntry` struct with `id`, `parent_id`, `timestamp`, `entry_type`
   - `SessionEntryType` enum: `Session`, `Message`, `Compaction`, `BranchSummary`
   - `SerializableMessage` for JSON serialization of `limit_llm::Message`
   - `generate_entry_id()` for 8-char hex IDs

2. Implemented `SessionTree` with:
   - HashMap-based entry storage indexed by ID
   - `build_context(leaf_id)` - walks parent chain to build message list
   - `append(entry)` - adds entry as child of current leaf
   - `branch_from(entry_id)` - O(1) branching by moving leaf pointer

3. JSONL storage methods:
   - `save_to_file(path)` - writes entries in DFS order
   - `load_from_file(path)` - parses JSONL into SessionTree
   - `append_to_file(path, entry)` - incremental append

4. Integrated with `SessionManager`:
   - `create_tree_session(session_id, cwd)` - creates new JSONL session
   - `load_tree_session(session_id)` - loads existing tree
   - `append_tree_entry(session_id, entry)` - appends to tree file
   - `save_tree_session(session_id, tree)` - saves complete tree
   - `has_tree_session(session_id)` - checks if JSONL exists
   - `migrate_to_tree(session_id)` - migrates .bin to .jsonl

5. Added branching commands in `limit-cli/src/tui/commands/branch.rs`:
   - `branch_from(ctx, entry_id)` - creates branch from entry
   - `list_branches(ctx)` - lists all branch endpoints
   - `BranchInfo` struct with `leaf_id`, `depth`, `is_current`

**Storage format**:
```
~/.limit/sessions/<uuid>.jsonl
```

**Effort**: ~4 hours

### Phase 4: Prompt Caching (API-level) ✅ COMPLETED

**Status**: Implemented in v0.0.40+

**What was done**:
1. Added `CacheControl` struct to `limit-llm/src/types.rs`:
   ```rust
   pub struct CacheControl {
       pub r#type: String,  // "ephemeral"
   }
   ```

2. Extended `Message` with `cache_control: Option<CacheControl>` field

3. Extended `Usage` with cache token tracking:
   ```rust
   pub struct Usage {
       pub input_tokens: u64,
       pub output_tokens: u64,
       pub cache_read_tokens: u64,   // Tokens read from cache (90% cheaper)
       pub cache_write_tokens: u64,  // Tokens written to cache
   }
   ```

4. Added `CacheSettings` to configuration:
   ```rust
   pub struct CacheSettings {
       pub retention: String,  // "none", "short", "long"
   }
   ```

5. Implemented cache token parsing for:
   - **Anthropic**: `cache_read_input_tokens`, `cache_creation_input_tokens`
   - **OpenAI**: `prompt_tokens_details.cached_tokens`

6. Added `apply_cache_control()` helper in `limit-llm/src/cache.rs`:
   - Applies `cache_control` to system prompt (last block)
   - Applies `cache_control` to last user message

7. Updated `TrackingDb` to record cache token usage:
   ```sql
   cache_read_tokens INTEGER NOT NULL DEFAULT 0,
   cache_write_tokens INTEGER NOT NULL DEFAULT 0
   ```

**Configuration** (`~/.limit/config.toml`):
```toml
[cache]
retention = "short"  # "none", "short", "long"
```

**Cost savings**: Cache read tokens cost ~10% of regular input tokens.

---

## 4. Data Structures

### Compaction Settings

```rust
pub struct CompactionSettings {
    pub enabled: bool,
    pub reserve_tokens: u32,      // Default: 16384
    pub keep_recent_tokens: u32,  // Default: 20000
}

impl Default for CompactionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        }
    }
}
```

### Session Entry Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEntryType {
    #[serde(rename = "message")]
    Message { message: Message },
    
    #[serde(rename = "compaction")]
    Compaction {
        summary: String,
        first_kept_id: String,
        tokens_before: u64,
        details: Option<CompactionDetails>,
    },
    
    #[serde(rename = "branch_summary")]
    BranchSummary {
        from_id: String,
        summary: String,
    },
    
    #[serde(rename = "model_change")]
    ModelChange {
        provider: String,
        model_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionDetails {
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
}
```

### Usage with Cache Tracking

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
}

impl Usage {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + 
        self.cache_read_tokens + self.cache_write_tokens
    }
}
```

---

## 5. Code Reference

### pi-mono Key Files

| File | Purpose |
|------|---------|
| `packages/coding-agent/src/core/compaction/compaction.ts` | Main compaction algorithm |
| `packages/coding-agent/src/core/compaction/utils.ts` | File tracking, serialization |
| `packages/coding-agent/src/core/session-manager.ts` | Tree structure, context building |
| `packages/ai/src/providers/anthropic.ts` | Anthropic cache_control |
| `packages/ai/src/providers/openai-responses.ts` | OpenAI prompt caching |
| `packages/ai/src/types.ts` | CacheRetention, Usage types |

### Limit Key Files

| File | Purpose | Status |
|------|---------|--------|
| `limit-llm/src/handoff.rs` | Token counting, compaction | EXISTS, needs integration |
| `limit-cli/src/session.rs` | Session management | Needs tree structure |
| `limit-llm/src/types.rs` | Message types | Needs cache fields |
| `limit-llm/src/anthropic_provider.rs` | Anthropic client | Needs cache_control |
| `limit-llm/src/openai_provider.rs` | OpenAI client | Needs prompt caching |

---

## Summary

### Implementation Status

| Phase | Status | Description |
|-------|--------|-------------|
| **Phase 1** | ✅ COMPLETED | Token-aware compaction with `ModelHandoff` |
| **Phase 2** | 🔲 NOT STARTED | LLM-generated summaries |
| **Phase 3** | ✅ COMPLETED | Tree-based sessions with JSONL |
| **Phase 4** | ✅ COMPLETED | API-level prompt caching |

### Recommended Implementation Order

1. ~~Phase 1 (Quick win): Integrate `ModelHandoff::compact_messages()` - 1-2 hours~~ ✅ DONE
2. ~~**Phase 4** (API caching): Add prompt caching - 1 day~~ ✅ DONE
3. ~~**Phase 3** (Tree sessions): Branching support - 2-3 days~~ ✅ DONE
4. **Phase 2** (Summarization): LLM-generated summaries - 1-2 days

### Expected Token Savings

| Technique | Savings | Complexity |
|-----------|---------|------------|
| Simple truncation | 30-40% | Low |
| Summarization | 40-70% | Medium |
| Prompt caching | 90% on cached tokens | Low |
| Tree sessions | Enables recovery | High |

### Configuration Example

```toml
# ~/.limit/config.toml

[compaction]
enabled = true
reserve_tokens = 16384
keep_recent_tokens = 20000

[caching]
retention = "long"  # "none", "short", "long"
```
