# Warm() Rewrite Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rewrite TLDR warm() to eliminate duplicate parsing, add parallelism, incremental updates, and persistent semantic index.

**Architecture:** ParseCoordinator replaces ASTLayer::warm() as the orchestrator. Each file is parsed once by tree-sitter; all layers receive pre-built FileAnalysis. Semantic index persists to disk.

**Tech Stack:** Rust, tokio, tree-sitter, serde, rayon (for parallel parsing)

---

## Problems

1. **Duplicate parsing**: CallGraphLayer re-parses every file that AST already parsed
2. **Sequential parsing**: One file at a time — slow for large codebases
3. **No incremental updates**: Every warm() re-parses all files, even unchanged ones
4. **Semantic index rebuilt every session**: Re-indexes all functions from scratch
5. **Cache desync**: `file_hashes.json` and `parse_cache/` can get out of sync
6. **No progress reporting**: User sees nothing during warm

## Design

### 1. Shared Parse Tree

ParseCoordinator discovers files, checks cache, and parses each file once. During parsing, extracts:
- Functions, classes, imports (existing)
- Call expressions (new) — `CallExpression { caller, callee, line, file }`

Each layer receives `Vec<FileAnalysis>` directly — no re-parsing needed.

CallGraph no longer calls tree-sitter. It builds from `analysis.call_expressions`.

SemanticIndex no longer calls `ast.all_functions()`. It builds from `analysis.functions` + `analysis.classes`.

### 2. Parallel Parse + Incremental

```
ParseCoordinator::warm():
  files = discover_files(project_path)

  // Check hashes (sequential I/O, fast)
  file_states = files.map(|f| { content, hash, cache_fresh })

  // Parse only modified files (parallel CPU)
  modified = file_states.filter(not cache_fresh)
  analyses = par_map(modified, |f| tree_sitter_parse(f))
               .with_semaphore(num_cpus, max 8)

  // Load cached for unchanged files
  cached = file_states.filter(cache_fresh).map(|f| cache.load(f))

  all = merge(cached, analyses)
  cache.store_all(modified)

  emit progress(parsed=modified.len(), total=files.len())

  return all
```

- `par_map` uses `tokio::task::spawn_blocking` for CPU-bound tree-sitter parsing
- Semaphore caps concurrent parses at `available_parallelism()` (max 8)
- Progress via `tracing::info!("warm: {parsed}/{total} files")`

### 3. Cache Refactor + Semantic Persistence

**Current**: `file_hashes.json` (registry) + `parse_cache/<blake3>.json` (per-file) — can desync.

**New**: Single cache file per source file at `cache/<blake3_hash>.json`:
```json
{
  "version": "v3",
  "hash": "abc123",
  "file": "src/lib.rs",
  "analysis": { ... }
}
```

- Hash in filename = no separate registry needed
- Version inside each file = per-file invalidation on version bump
- On startup: delete cache files with old version
- Auto-cleanup: limit cache dir size (LRU eviction)

**Semantic index persistence**:
- Serialize as `cache/semantic_index.json`
- Include hash of all file_hashes — if none changed, load from cache
- Embeddings serialized alongside (Vec<Vec<f32>> via serde)
- Skip rebuild when cached version matches current file hashes

**CallGraph cache**: Eliminated. Rebuilds from `FileAnalysis.call_expressions` which is fast (no tree-sitter needed).

### 4. Data Types + Layer Interface

**New `FileAnalysis` field**:
```rust
pub struct FileAnalysis {
    pub file: PathBuf,
    pub language: Language,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<ImportInfo>,
    pub call_expressions: Vec<CallExpression>,  // NEW
}
```

**New type**:
```rust
pub struct CallExpression {
    pub caller: String,      // function name containing the call
    pub callee: String,      // function name being called
    pub line: usize,
    pub file: PathBuf,
}
```

**New `AnalysisLayer` trait**:
```rust
#[async_trait]
pub trait AnalysisLayer {
    async fn build(&mut self, analyses: &[FileAnalysis]) -> Result<()>;
    fn save(&self, cache_dir: &Path) -> Result<()> { Ok(()) }
    fn load(&mut self, cache_dir: &Path) -> Result<bool> { Ok(false) }
}
```

**New `ParseCoordinator`**:
```rust
pub struct ParseCoordinator {
    project_path: PathBuf,
    language: Language,
    cache: CacheManager,
}

impl ParseCoordinator {
    pub async fn warm(&mut self) -> Result<Vec<FileAnalysis>> {
        // discover → hash check → parallel parse → cache
    }

    fn discover_files(&self) -> Result<Vec<PathBuf>> { ... }

    async fn read_and_hash(&self, file: &Path) -> Result<(String, String)> { ... }

    fn tree_sitter_parse(&self, content: &str, path: &Path, lang: Language) -> Result<FileAnalysis> { ... }
}
```

**Simplified `TLDR::warm()`**:
```rust
pub async fn warm(&mut self) -> Result<()> {
    let analyses = self.coordinator.warm().await?;
    self.call_graph.build(&analyses).await?;
    self.semantic.build(&analyses).await?;
    if self.semantic.should_save() {
        self.semantic.save(&self.cache_dir)?;
    }
    Ok(())
}
```

### 5. CallExpression Extraction

During `tree_sitter_parse()`, add a new case in `walk_node()`:
```rust
// Call expressions
"call_expression" | "method_call_expression" => {
    if let Some(func) = self.extract_call(child, source, current_function) {
        analysis.call_expressions.push(func);
    }
}
```

Track `current_function` by passing it through the walk. When entering a `function_item`/`impl_item`, set `current_function`. When leaving, clear it.

`extract_call()`:
```rust
fn extract_call(&self, node: Node, source: &str, caller: &str) -> Option<CallExpression> {
    let function_node = node.child_by_field_name("function")?;
    let callee = node_text(function_node, source);
    // Skip self.method() — caller already known
    if callee == "self" { return None; }
    Some(CallExpression {
        caller: caller.to_string(),
        callee,
        line: node.start_position().row + 1,
        file: std::path::PathBuf::new(), // set by caller
    })
}
```

### 6. Migration

- Old cache (`file_hashes.json` + `parse_cache/`) auto-cleaned on first warm
- `CACHE_VERSION` bumped to "v3"
- `TldrTool::get_tldr()` unchanged — still uses OnceCell
- Existing `AnalysisLayer` trait is internal — doesn't affect public API
- `TLDR` public methods unchanged (find_function, search, etc.)

## Tasks

### Task 1: Add CallExpression type

**Files:**
- Modify: `limit-tldr/src/types.rs`

**Step 1:** Add `CallExpression` struct after `ImportInfo`
**Step 2:** Add `call_expressions: Vec<CallExpression>` to `FileAnalysis`
**Step 3:** Run tests

### Task 2: Extract call_expressions during tree-sitter parse

**Files:**
- Modify: `limit-tldr/src/parsers/tree_sitter.rs`

**Step 1:** Add `extract_call()` method
**Step 2:** Track `current_function` in `walk_node()` — set when entering `function_item`/`method_definition`/`arrow_function`, clear when exiting
**Step 3:** Add `"call_expression"` | `"method_call_expression"` case in `walk_node()`
**Step 4:** Add test: parse file with calls → verify call_expressions extracted
**Step 5:** Run tests

### Task 3: Build CallGraph from FileAnalysis instead of re-parsing

**Files:**
- Modify: `limit-tldr/src/layers/call_graph.rs`

**Step 1:** Add `build(&mut self, analyses: &[FileAnalysis])` method
**Step 2:** Iterate `analysis.call_expressions` → populate forward/backward maps
**Step 3:** Remove file reading and tree-sitter parsing from warm()
**Step 4:** Keep backward-compatible `warm()` that delegates to `build()`
**Step 5:** Add test: build from pre-built analyses
**Step 6:** Run tests

### Task 4: Build SemanticIndex from FileAnalysis

**Files:**
- Modify: `limit-tldr/src/semantic/mod.rs`

**Step 1:** Change `warm(ast, call_graph)` to `build(analyses)`
**Step 2:** Index functions and structs from `analyses` directly
**Step 3:** Remove dependency on ASTLayer and CallGraphLayer params
**Step 4:** Keep backward-compatible `warm()` method
**Step 5:** Run tests

### Task 5: Add AnalysisLayer trait

**Files:**
- Create: `limit-tldr/src/layers/mod.rs` (update)
- Create: `limit-tldr/src/layers/trait.rs`

**Step 1:** Define `AnalysisLayer` trait with `build()`, `save()`, `load()`
**Step 2:** Implement for CallGraphLayer and SemanticIndex
**Step 3:** Add test
**Step 4:** Run tests

### Task 6: Create ParseCoordinator

**Files:**
- Create: `limit-tldr/src/coordinator.rs`
- Modify: `limit-tldr/src/lib.rs`

**Step 1:** Implement `discover_files()` (move from ASTLayer)
**Step 2:** Implement `read_and_hash()` with parallel I/O
**Step 3:** Implement `tree_sitter_parse()` with call_expression extraction
**Step 4:** Implement `warm()` with semaphore-bounded parallel parsing
**Step 5:** Add progress logging
**Step 6:** Add test
**Step 7:** Run tests

### Task 7: Refactor CacheManager

**Files:**
- Modify: `limit-tldr/src/cache/cache_manager.rs`

**Step 1:** New cache format: single file per source, hash in filename
**Step 2:** Add `store(analysis: &FileAnalysis)` storing `{version, hash, analysis}`
**Step 3:** Add `load(file: &Path) -> Option<FileAnalysis>` with version check
**Step 4:** Add `is_fresh(file: &Path) -> bool` comparing hashes
**Step 5:** Add `cleanup()` to delete old-version cache files
**Step 6:** Migrate CACHE_VERSION to "v3"
**Step 7:** Add test
**Step 8:** Run tests

### Task 8: Semantic index persistence

**Files:**
- Modify: `limit-tldr/src/semantic/mod.rs`
- Modify: `limit-tldr/src/cache/cache_manager.rs`

**Step 1:** Add `save(cache_dir)` to SemanticIndex — serialize entries + embeddings
**Step 2:** Add `load(cache_dir) -> bool` — deserialize if version matches
**Step 3:** Add hash-based skip logic: compute file hash set, compare with cached
**Step 4:** Implement `should_save()` — only save if embeddings were generated
**Step 5:** Add test
**Step 6:** Run tests

### Task 9: Simplify TLDR::warm()

**Files:**
- Modify: `limit-tldr/src/lib.rs`

**Step 1:** Replace sequential warm logic with ParseCoordinator + layer builds
**Step 2:** Add cache cleanup on startup
**Step 3:** Keep all public API methods unchanged
**Step 4:** Run full test suite
**Step 5:** Commit

### Task 10: Update TldrTool

**Files:**
- Modify: `limit-agent/src/tools/tldr.rs`

**Step 1:** Update `get_tldr()` to use new warm flow
**Step 2:** Add progress event emission to TUI
**Step 3:** Test with real project
