
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
