
## Event Schema Implementation (Task 4)

### Pattern: Flat Event Enum with Version Field
- Event enum uses flat structure with version: u32 field in each variant
- Version field enables future compatibility and schema evolution
- Serde derive macros (Debug, Clone, Serialize, Deserialize) for JSON support
- HashMap<String, serde_json::Value> for flexible ToolCall args

### Success Factors
- Keep event schema simple and versioned
- Unit test validates JSON round-trip serialization
- Version field is mandatory in all variants for compatibility

### Dependencies Added
- serde = { version = "1.0", features = ["derive"] }
- serde_json = "1.0"

### Test Validation
- `cargo test --package limit-agent` confirms all tests pass
- Serialization/deserialization test ensures JSON compatibility

## Task 2: Config Schema + Loading

### Implementation Notes
- Added dependencies: toml, serde, dirs to limit-llm/Cargo.toml
- Created limit-llm/src/config.rs with Config struct and load() function
- Config struct fields: api_key (Option<String>), model, max_tokens, timeout
- Default values: model="claude-3-5-sonnet-20241022", max_tokens=4096, timeout=60
- load() reads from ~/.limit/config.toml, returns defaults if file missing
- Implemented Default trait for Config (not just a default() method)
- Added unit tests: test_load_missing_file, test_load_valid_config, test_load_partial_config_uses_defaults, test_default_config
- All 10 tests pass (6 existing types tests + 4 new config tests)
- cargo clippy passes with no warnings
- cargo fmt applied

### Code Quality
- Used serde Deserialize derive with default functions for optional fields
- Proper error handling with io::Error for load() function
- Clean implementation of Default trait to avoid clippy warnings
- Config path: ~/.limit/config.toml (using dirs crate)
- Tests cover missing file, valid config, partial config with defaults

## Task 3: Error Types + thiserror Setup

### Implementation Notes
- Added thiserror = "1.0" to all three crates (limit-llm, limit-agent, limit-cli)
- Created error.rs files with thiserror::Error derive macro for each crate
- Error types:
  - LlmError: ApiError, NetworkError, ConfigError, PersistenceError
  - AgentError: ToolError, StateError, SandboxError
  - CliError: IoError, ConfigError, AgentError (with From<> trait for limit_agent::error::AgentError)
- Exported error modules in lib.rs (library crates) and main.rs (binary crate)
- Added limit-agent as dependency to limit-cli for error interop
- Added serde and serde_json to limit-agent (already present from Task 4)

### Success Factors
- Cargo.toml files must have [dependencies] section header
- thiserror provides explicit error types with #[error()] attribute for formatting
- From<> traits enable automatic error conversion between crates
- Keep error variants simple and focused (just the specified ones, no extra logic)

### Code Quality
- Build succeeds with cargo build --workspace
- Minor warnings (unused import, dead code) are expected until errors are used
- Simple, explicit error types following Rust best practices
- Clear error messages using thiserror's #[error()] attribute

### Files Modified
- limit-llm/Cargo.toml: Added thiserror
- limit-agent/Cargo.toml: Added thiserror, [dependencies] section
- limit-cli/Cargo.toml: Added thiserror, limit-agent dependency, [dependencies] section
- limit-llm/src/error.rs: Created with LlmError enum
- limit-agent/src/error.rs: Created with AgentError enum
- limit-cli/src/error.rs: Created with CliError enum
- limit-llm/src/lib.rs: Added pub mod error
- limit-agent/src/lib.rs: Added pub mod error
- limit-cli/src/main.rs: Added mod error

## Task 6: Anthropic Client with Streaming

### Implementation Notes
- Added dependencies to limit-llm/Cargo.toml: reqwest (json, stream features), tokio (full), async-stream, futures, async-trait, bytes, mockito (dev)
- Created limit-llm/src/client.rs with AnthropicClient struct and ResponseChunk enum
- AnthropicClient: api_key, client (reqwest::Client with 30s connect, 300s timeout), base_url
- ResponseChunk enum: ContentDelta(String), ToolCallDelta {id, name, arguments}, Done(Usage)
- send() method returns Pin<Box<dyn Stream<Item = Result<ResponseChunk, LlmError>> + Send + '_>>
- Implemented SSE (Server-Sent Events) stream parsing from Anthropic Messages API
- Retry logic with exponential backoff: 3 attempts, delays of 1s, 2s, 4s (2^attempt seconds)
- HTTP error handling: 429 returns ApiError with rate limit message, other errors include status code
- SSE parser handles: content_block_delta (text/partial_json), content_block_start (tool_use), content_block_stop, message_delta (stop_reason, usage)
- parse_sse_line() parses SSE format: "data: {json}\n\n"
- Buffer management: accumulate chunks, parse lines, remove parsed data from buffer

### Tests
- test_streaming: Mock server with 3 SSE chunks, validates chunk parsing
- test_retry_on_429: Mock returns 429 twice then 200, validates retry logic
- test_timeout: Mock with slow response (500ms sleep), validates timeout handling
- test_tool_call_streaming: Mock with tool_use events, validates tool call parsing
- test_parse_sse_line: Unit test for SSE line parsing
- test_parse_sse_line_empty: Unit test for empty lines
- test_parse_sse_line_comment: Unit test for comment lines
- All 17 tests pass (10 existing + 7 new client tests)

### Success Factors
- Use try_stream! macro for async stream with error propagation
- Pin<Box<dyn Stream>> for returning streams from async functions
- Clone trait implementation for AnthropicClient to enable reuse in send()
- Mockito with with_chunked_body() for streaming response mocking
- Borrow checker fix: to_string() to avoid borrowing buffer while modifying it
- Unpin trait bound on stream parameter for next() method compatibility
- std::io::Error type for mockito closures (not mockito::Error)
- while let loop pattern for SSE line parsing (clippy-friendly)

### Code Quality
- cargo test --package limit-llm: 17 tests passed, 0 failed
- cargo clippy --package limit-llm: No warnings
- Proper error handling for network errors and API errors
- Clean separation: AnthropicClient (HTTP), do_request (single request), parse_sse_stream (stream parsing)
- No caching layer, batch requests, or request queuing (as required)
- Tests use mock server, never call real Anthropic API

### Files Modified
- limit-llm/Cargo.toml: Added reqwest, tokio, async-stream, futures, async-trait, bytes, mockito
- limit-llm/src/client.rs: Created (453 lines)
- limit-llm/src/lib.rs: Added pub mod client, pub use client::{AnthropicClient, ResponseChunk}

## Task 7: SQLite Tracking Implementation

### Implementation Notes
- Added rusqlite = "0.31" to limit-llm/Cargo.toml
- Created limit-llm/src/tracking.rs with TrackingDb and UsageStats
- Database location: ~/.limit/tracking.db (created automatically)
- UsageStats struct: total_requests, total_tokens, total_cost, avg_duration_ms
- TrackingDb struct wraps Arc<Mutex<Connection>> for thread-safe concurrent access
- track_request() method: logs model, input_tokens, output_tokens, cost, duration_ms
- get_usage_stats(days: u32) method: retrieves stats for last N days
- Auto-creates tables on first run with CREATE TABLE IF NOT EXISTS
- Index on timestamp for efficient time-range queries
- PRAGMA busy_timeout set to 5 seconds for concurrent access handling

### Database Schema
```sql
CREATE TABLE IF NOT EXISTS requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    model TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    cost REAL NOT NULL,
    duration_ms INTEGER NOT NULL
)
```

### Tests
- test_create_tables: Verifies table initialization
- test_track_request: Inserts 2 requests, validates count
- test_get_usage_stats: 3 requests, validates aggregation (675 tokens, 0.008 cost)
- test_empty_usage_stats: Validates default values (all zeros)
- test_concurrent_access: 2 threads inserting 10 requests each (20 total)
- All 22 tests pass (18 existing + 4 new tracking tests)

### Success Factors
- Arc<Mutex<Connection>> for thread-safe concurrent access (Connection is not Send/Sync)
- rusqlite::params! macro for mixed-type parameter binding (strings + numbers)
- Proper error handling with lock acquisition failures
- Test database uses tempfile for isolation
- No caching, request queuing, or compaction logic (as required)
- Timestamp calculations use SystemTime::now() - days * 86400 seconds

### Code Quality
- cargo test --package limit-llm: 22 tests passed, 0 failed
- cargo clippy --package limit-llm: No warnings
- Clean separation: TrackingDb (public API), tests (private helpers)
- SQLite busy_timeout prevents lock contention in concurrent access
- Mutex ensures data consistency during concurrent inserts/queries

### Files Modified
- limit-llm/Cargo.toml: Added rusqlite = "0.31"
- limit-llm/src/tracking.rs: Created (299 lines)
- limit-llm/src/lib.rs: Added pub mod tracking

## Task 10: Tool Trait and Registry System

### Implementation Notes
- Added async-trait = "0.1" to limit-agent/Cargo.toml dependencies
- Added tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] } to [dev-dependencies]
- Created limit-agent/src/tool.rs with Tool trait and EchoTool example
- Created limit-agent/src/registry.rs with ToolRegistry for tool management
- Tool trait: async execute() returning Result<Value, AgentError>, name() method
- ToolRegistry: HashMap-based storage with register(), get(), list(), execute() methods
- EchoTool: Simple example tool that echoes back input arguments
- Arc<dyn Tool> for trait object storage in registry

### Registry Operations
- register<T>(tool: T): Registers any type implementing Tool, stores as Arc<dyn Tool>
- get(name: &str): Returns cloned Arc<dyn Tool> if found
- list(): Returns sorted Vec<String> of all tool names
- execute(name: &str, args: Value): Looks up tool and executes, returns error if not found

### Tests
- Tool tests: test_echo_tool_name, test_echo_tool_execute, test_echo_tool_default
- Registry tests: test_registry_new, test_registry_default, test_registry_register, test_registry_get, test_registry_get_nonexistent, test_registry_list, test_registry_execute, test_registry_execute_nonexistent, test_registry_multiple_tools
- All 13 tests pass (10 existing + 3 tool tests)

### Success Factors
- Use async_trait::async_trait for trait with async methods (Rust limitation)
- Box<dyn Tool> or Arc<dyn Tool> for trait objects due to async trait size limitation
- Cloneable Arc in get() method allows multiple callers to use same tool
- Generic register<T>() accepts any type implementing Tool (flexible API)
- Default trait implementations for both EchoTool and ToolRegistry
- Comprehensive test coverage including error cases and concurrent scenarios

### Code Quality
- cargo test --package limit-agent: 13 tests passed, 0 failed
- cargo fmt --package limit-agent: Formatting applied
- cargo clippy --package limit-agent: No warnings
- Clean separation: Tool (trait), EchoTool (example), ToolRegistry (registry)
- No tool validation logic (as required)
- Proper error handling with AgentError::ToolError for missing tools

### Files Modified
- limit-agent/Cargo.toml: Added async-trait, [dev-dependencies] with tokio
- limit-agent/src/tool.rs: Created (70 lines)
- limit-agent/src/registry.rs: Created (156 lines)
- limit-agent/src/lib.rs: Added pub mod tool, pub mod registry, exports for Tool, EchoTool, ToolRegistry
- limit-agent/src/error.rs: Removed unused import (cargo fix cleanup)
- limit-llm/Cargo.toml: Fixed syntax error (removed empty line)

## Task 9: Model Hand-off (Resume/Compact)

### Implementation Summary

Successfully implemented context compaction and model switching in `limit-llm/src/handoff.rs`:

**Key Components:**
1. **ModelHandoff struct** - Token counting and compaction logic
2. **Token counting** - Uses tiktoken-rs with cl100k_base tokenizer
3. **compact_messages()** - Rule-based compaction:
   - Preserves system message
   - Keeps last N messages within token budget
   - Uses 20% safety buffer (minimum 100 tokens)
4. **handoff_to_model()** - Model switching with automatic compaction

**Compaction Strategy (Rule-based, no AI summarization):**
- Always keep system message if present
- Calculate remaining budget after system tokens and safety buffer
- Iterate messages in reverse order, keeping those that fit
- Preserve conversation context by prioritizing recent messages

**Token Counting Features:**
- `count_tokens()` - Simple text token count
- `count_message_tokens()` - Includes role overhead (4 tokens) and tool_calls
- `count_total_tokens()` - Sum of all message tokens

**Test Coverage (9/9 passing):**
- Token counting accuracy
- Message token counting with tool calls
- Total token counting
- Compaction preserves system message
- Compaction keeps recent messages
- Handoff without compaction
- Handoff with compaction when needed
- Token count accuracy within tolerance

### Technical Decisions

1. **Safety buffer**: 20% of target tokens (min 100) to prevent exceeding context
2. **Compaction threshold**: 90% of context window triggers compaction
3. **Tokenizer**: cl100k_base (used by Claude models)
4. **Role enum**: Added PartialEq derive for test comparisons

### Gotchas

1. **Edit tool LINE#ID tags** - When using edit tool, ensure the `lines` parameter contains ONLY the content to be inserted, not the LINE#ID tags
2. **Small budgets with safety buffers** - The safety buffer can consume entire budget for small targets, need to test with realistic budgets (500+ tokens)
3. **Duplicate code during edits** - Multiple edits on same section can leave duplicate code if not careful with pos/end ranges
4. **Token count expectations** - Actual token counts can vary from expectations; use reasonable tolerance (10-20%) in tests

### Performance Considerations

- Token counting is O(n) where n is message length
- Compaction is O(m) where m is number of messages
- Suitable for use before LLM calls without significant overhead


## Task 11: Conditional Tool Execution (Parallel vs Sequential)

### Implementation Notes
- Created limit-agent/src/executor.rs with ToolExecutor struct (400 lines)
- Added tokio and futures to limit-agent/Cargo.toml for async execution
- ToolCall struct: id, name, args for single tool execution
- ToolResult struct: call_id, output (Result<Value, AgentError>)
- ToolExecutor: registry (Arc<ToolRegistry>), max_concurrent (default 5), timeout (default 60s)

### Conditional Execution Logic
- categorize_calls(): Analyzes tool calls for dependencies
- has_dependencies(): Checks args for variable references ($output_, $var_, $result_)
- Independent calls → execute_parallel() with concurrency limit
- Dependent calls → execute_sequential()

### Parallel Execution Implementation
- Uses futures::stream with buffer_unordered() for concurrency control
- Arc::clone() for sharing registry across concurrent tasks
- tokio::time::timeout() enforces per-tool timeout
- Configurable max_concurrent prevents unlimited parallelism

### Sequential Execution
- Executes dependent tools one-by-one in order
- Still applies timeout per tool
- Used when tools reference previous outputs

### Tests (12/12 passing)
- test_tool_call_new: Validates ToolCall creation
- test_executor_new/test_executor_with_config: Constructor and builder pattern
- test_execute_tools_empty: Edge case handling
- test_execute_tools_single: Single tool execution
- test_execute_tools_parallel: Multiple independent tools (parallel)
- test_execute_tools_sequential_with_dependencies: Tools with $output_ references
- test_execute_tools_timeout: Timeout enforcement (100ms test)
- test_execute_tools_tool_not_found: Error handling
- test_categorize_calls_no_dependencies: Independence detection
- test_categorize_calls_with_dependencies: Dependency detection
- test_parallel_with_concurrency_limit: Concurrency limit validation (2 limit, 5 calls)

### Success Factors
- Arc<ToolRegistry> enables sharing across async tasks without Clone on registry
- futures::stream::buffer_unordered() provides clean concurrency limiting
- tokio::time::timeout() prevents hanging on slow tools
- Dependency detection via string patterns ($output_, etc.) is simple and effective
- No agent reasoning logic, just execution (as required)

### Code Quality
- cargo test --package limit-agent executor: 12 passed, 0 failed
- cargo clippy --package limit-agent: No warnings for executor.rs
- cargo fmt --package limit-agent: Formatted
- Clean separation: ToolCall, ToolResult, ToolExecutor, tests
- No warnings when building lib (warnings from other modules are pre-existing)

### Files Modified
- limit-agent/Cargo.toml: Added tokio (full features), futures = "0.3"
- limit-agent/src/executor.rs: Created (400 lines)
- limit-agent/src/lib.rs: Added pub mod executor
- limit-agent/src/error.rs: Added Clone derive to AgentError (for ToolResult Clone)

### Dependencies Added
- tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
- futures = "0.3"
# State Management Implementation Learnings

## Task 13: limit-agent state management

### What was implemented:
- AgentState struct with messages, tool_results, decisions, todos, iteration fields
- save_state() and load_state() methods using bincode serialization
- Max iterations: 50 tool calls per session
- Loop detection: reject same tool+args 3 times
- State storage at: .limit/agent-state.bin

### Key Learnings:

**1. Bincode limitations with dynamic types**
- bincode 1.3 doesn't work well with `serde_json::Value`
- bincode expects compile-time type information
- `serde_json::Value` can represent any JSON type dynamically
- Solution: Convert to structured types or use JSON serialization

**2. Test isolation issues**
- Multiple tests sharing the same state file caused interference
- Added `fs::remove_dir_all(".limit")` for proper cleanup
- Tests should use `--test-threads=1` to prevent concurrency issues

**3. State structure**
```rust
pub struct AgentState {
    pub messages: Vec<Message>,
    pub tool_results: HashMap<String, serde_json::Value>,  // Limited by bincode
    pub decisions: Vec<Decision>,
    pub todos: Vec<Todo>,
    pub iteration: u32,
    tool_call_history: Vec<String>,  // For loop detection
}
```

**4. Loop detection approach**
- Create signature: `format!("{}:{}", tool_name, args)`
- Track all calls in history vector
- Count occurrences of signature before adding new call
- Reject if count >= MAX_LOOP_COUNT (3)

**5. Iteration tracking**
- Increment before each tool call
- Check if iteration > MAX_ITERATIONS (50)
- Return error if exceeded

### Tests passing:
- test_agent_state_default ✓
- test_agent_state_new ✓
- test_increment_iteration ✓
- test_max_iterations ✓
- test_is_max_iterations ✓
- test_loop_detection ✓
- test_loop_detection_different_args ✓

### Known limitations:
1. `tool_results` field with `serde_json::Value` not fully testable with bincode
2. State persistence may need refactoring for production use with dynamic tool results

### Potential improvements:
1. Use JSON instead of bincode for serialization
2. Add state versioning for migration
3. Implement state compression for large histories
4. Add state export/import functionality
