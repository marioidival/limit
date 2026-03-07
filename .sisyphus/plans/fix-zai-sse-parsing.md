# Fix z.ai SSE Parsing & REPL Response Display

## TL;DR

> **Quick Summary**: Fix 3 issues preventing REPL from displaying LLM responses with z.ai API: SSE line format, tool_use detection, and partial JSON accumulation.
>
> **Deliverables**:
> - Working SSE parsing for z.ai's `event:` + `data:` on separate lines
> - Correct tool_use block detection
> - Incremental tool argument accumulation
>
> **Estimated Effort**: Short
> **Parallel Execution**: NO - sequential fixes in same file
> **Critical Path**: SSE fix → tool_use fix → partial JSON accumulation

---

## Context

### Original Request
REPL não exibe respostas - mostra "Thinking..." e retorna ao prompt.

### Root Causes Identified
Comparando pi-mono (TypeScript) com limit (Rust):

1. **SSE Format Mismatch**: z.ai envia `event:` em uma linha, `data:` na próxima. Parser atual espera tudo na mesma linha.
2. **Wrong tool_use Detection**: Código checa `content_block.get("tool_use")` mas z.ai envia `content_block.type === "tool_use"`.
3. **No Partial JSON Accumulation**: Argumentos de tool chegam em chunks via `input_json_delta.partial_json`. Precisa acumular e re-parsear.

### z.ai SSE Format Example
```
event: content_block_start
data: {"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "call_123", "name": "file_read", "input": {}}}

event: content_block_delta
data: {"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"path\":\"README.md\"}"}}
```

---

## Work Objectives

### Core Objective
Fazer o REPL funcionar corretamente com z.ai API, permitindo que o LLM chame tools e exiba respostas.

### Concrete Deliverables
- `limit-llm/src/client.rs` - SSE parsing corrigido
- Tool calls detectados e acumulados corretamente
- Respostas exibidas no REPL

### Definition of Done
- [ ] `cargo test -p limit-llm` passa
- [ ] REPL exibe resposta do LLM
- [ ] Tool calls funcionam (ex: `file_read`)

---

## TODOs

- [x] 1. Fix SSE Line Parsing for z.ai Format

  **What to do**:
  - `parse_sse_line` deve skipar linhas `event:` e processar linhas `data:` separadamente
  - Atual: espera `data:` na mesma linha
  - Necessário: `event:` em uma linha, `data:` na próxima é válido

  **File**: `limit-llm/src/client.rs:261-285`

  **Current Code**:
  ```rust
  // Skip event: lines (we only care about data)
  if line.starts_with("event:") {
      continue;
  }
  ```

  **Fix**: Already skips `event:` lines correctly. Verify `data:` parsing handles both:
  - `data: {...}\n` (Anthropic format)
  - Separate lines (z.ai format) - should work since we skip `event:` lines

  **Acceptance Criteria**:
  - [ ] Debug output shows `data:` lines being parsed
  - [ ] `event:` lines are skipped without errors

- [x] 2. Fix tool_use Detection in content_block_start

  **What to do**:
  - z.ai sends `content_block.type = "tool_use"`, not `content_block.tool_use`
  - Extract `id` and `name` from `content_block` directly

  **File**: `limit-llm/src/client.rs:217-235`

  **Current Code**:
  ```rust
  if let Some(tool_use) = content_block.get("tool_use") {
      let id = tool_use.get("id")...
  ```

  **Fix**:
  ```rust
  let block_type = content_block.get("type").and_then(|v| v.as_str());
  if block_type == Some("tool_use") {
      let id = content_block.get("id").and_then(|v| v.as_str())...
      let name = content_block.get("name").and_then(|v| v.as_str())...
  ```

  **Acceptance Criteria**:
  - [ ] Tool calls detected from `content_block.type == "tool_use"`
  - [ ] `id` and `name` extracted from `content_block` root

- [x] 3. Add Partial JSON Accumulation for Tool Arguments

  **What to do**:
  - Tool arguments arrive in chunks via `input_json_delta.partial_json`
  - Need to: accumulate by tool call ID, parse incrementally

  **File**: `limit-llm/src/client.rs:173-259` (parse_sse_stream function)

  **Implementation**:
  ```rust
  // Add state tracking at top of stream
  let mut tool_partial_json: std::collections::HashMap<String, String> = std::collections::HashMap::new();
  let mut tool_calls_by_id: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();

  // In content_block_delta handler for input_json_delta:
  } else if let Some(delta_type) = delta.get("type").and_then(|v| v.as_str()) {
      if delta_type == "input_json_delta" {
          if let Some(partial_json) = delta.get("partial_json").and_then(|v| v.as_str()) {
              // Find tool call by index, accumulate JSON
              if let Some(index) = parsed.get("index").and_then(|v| v.as_u64()) {
                  let tool_id = format!("tool_{}", index);
                  tool_partial_json.entry(tool_id.clone())
                      .or_insert_with(String::new)
                      .push_str(partial_json);
                  
                  // Try to parse accumulated JSON
                  if let Some((name, _)) = tool_calls_by_id.get(&tool_id) {
                      let args = parse_partial_json(tool_partial_json.get(&tool_id).unwrap());
                      yield Ok(ResponseChunk::ToolCallDelta {
                          id: tool_id.clone(),
                          name: name.clone(),
                          arguments: args,
                      });
                  }
              }
          }
      }
  }
  ```

  **Acceptance Criteria**:
  - [ ] `partial_json` chunks accumulated per tool call
  - [ ] Arguments parsed incrementally
  - [ ] Final tool call has complete arguments

- [x] 4. Add Partial JSON Parser Utility

  **What to do**:
  - Create function that parses potentially incomplete JSON
  - Returns valid object even if JSON is incomplete

  **File**: `limit-llm/src/client.rs` (new function)

  **Implementation**:
  ```rust
  /// Parse potentially incomplete JSON during streaming.
  /// Returns empty object if parsing fails.
  fn parse_partial_json(json: &str) -> serde_json::Value {
      if json.trim().is_empty() {
          return serde_json::json!({});
      }
      
      // Try standard parsing first
      if let Ok(value) = serde_json::from_str::<serde_json::Value>(json) {
          return value;
      }
      
      // If parsing fails, return empty object
      // (In future, could use partial-json crate for better handling)
      serde_json::json!({})
  }
  ```

  **Acceptance Criteria**:
  - [ ] Function handles empty strings
  - [ ] Function handles complete JSON
  - [ ] Function handles incomplete JSON gracefully

- [x] 5. Track Tool Calls by Index

  **What to do**:
  - When `content_block_start` with `tool_use` arrives, store mapping of index → (id, name)
  - When `content_block_delta` arrives with index, look up tool call

  **File**: `limit-llm/src/client.rs:217-235`

  **Implementation**: Add to state tracking in #3

  **Acceptance Criteria**:
  - [ ] Tool call index tracked from `content_block_start`
  - [ ] Deltas matched to correct tool call by index

- [x] 6. Remove Debug Statements

  **What to do**:
  - Remove `eprintln!("[DEBUG] Chunk: ...")` after testing

  **File**: `limit-llm/src/client.rs:192`

  **Acceptance Criteria**:
  - [ ] No debug output in production

- [x] 7. Test End-to-End

  **What to do**:
  - Run REPL and test with z.ai API
  - Verify: text response, tool call, tool result handling

  **Test Commands**:
  ```bash
  cargo build --release
  cargo run --release
  # In REPL: "Read the README.md file"
  ```

  **Acceptance Criteria**:
  - [ ] REPL displays LLM text responses
  - [ ] Tool calls work (file_read, etc.)
  - [ ] Tool results sent back to LLM
  - [ ] Final response displayed

---

## Commit Strategy

- **Commit 1**: `fix(llm): correct SSE parsing for z.ai format`
- **Commit 2**: `fix(llm): detect tool_use from content_block.type`
- **Commit 3**: `fix(llm): accumulate partial JSON for tool arguments`

---

## Success Criteria

### Verification Commands
```bash
cargo test -p limit-llm                    # All tests pass
cargo run --release                        # REPL starts
# In REPL: "Hello" → displays response
# In REPL: "Read README.md" → tool call works
```

### Final Checklist
- [ ] SSE parsing handles z.ai format
- [ ] Tool calls detected correctly
- [ ] Partial JSON accumulated
- [ ] REPL displays responses
- [ ] No debug statements
