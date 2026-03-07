# Learnings - Multi-Provider Support

## 2026-03-07 Initial Analysis

### Current Architecture

**Config** (`limit-llm/src/config.rs`):
- Fields: `api_key: Option<String>`, `model: String`, `max_tokens: u32`, `timeout: u64`
- Defaults via serde: `default_model()`, `default_max_tokens()`, `default_timeout()`
- Missing: `base_url` field

**Client** (`limit-llm/src/client.rs`):
- `AnthropicClient` struct has `base_url: String` field (line 13)
- `new()` accepts only `api_key: String` (line 42)
- Hardcoded URL at line 52: `"https://api.anthropic.com/v1/messages"`
- Hardcoded timeout: 300s (line 44)
- `build_request_body` (lines 137-151) hardcodes `model` and `max_tokens`

**Agent Bridge** (`limit-cli/src/agent_bridge.rs`):
- Creates client at line 61: `AnthropicClient::new(api_key.clone())`
- Has access to full `config` but only passes `api_key`

### Key Changes Required

1. Config: Add `base_url: Option<String>` with serde default
2. Client `new()`: Accept base_url + timeout params
3. `build_request_body`: Accept model + max_tokens params
4. Agent bridge: Pass all config values to client

## 2026-03-07 Task: Add base_url to Config

### Pattern Observed
All Config fields with defaults follow this exact pattern:
```rust
#[serde(default = "default_<field>")]
pub <field>: <type>,
```

Helper functions placed after struct, before impl blocks:
```rust
fn default_<field>() -> <type> {
    <default_value>
}
```

### Implementation Details
- Added `base_url: Option<String>` field after `timeout` (logical grouping)
- Used `#[serde(default = "default_base_url")]` attribute
- Added helper: `fn default_base_url() -> Option<String> { None }`
- Updated `Default::default()` to include `base_url: None`
- Backward compatible: existing configs without base_url work (Option<T> handles missing values)

### Verification
- Config compiles without syntax errors
- Pattern matches existing field implementation exactly

## 2026-03-07 Task: Update AnthropicClient::new() for base_url and timeout

### Implementation Pattern
**Before:**
```rust
pub fn new(api_key: String) -> Self {
    let client = Client::builder()
        .timeout(Duration::from_secs(300))  // hardcoded
        .connect_timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    Self {
        api_key,
        client,
        base_url: "https://api.anthropic.com/v1/messages".to_string(),  // hardcoded
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
    }
}
```

**After:**
```rust
pub fn new(api_key: String, base_url: Option<&str>, timeout: u64) -> Self {
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout))  // parameterized
        .connect_timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    Self {
        api_key,
        client,
        base_url: base_url.unwrap_or("https://api.anthropic.com/v1/messages").to_string(),  // default provided
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 4096,
    }
}
```

### Key Changes
1. Added `base_url: Option<&str>` parameter - allows overriding default API URL
2. Added `timeout: u64` parameter - allows configurable timeout instead of hardcoded 300s
3. Used `unwrap_or()` to provide default URL when `base_url` is `None`
4. Updated all 4 test calls to use `None` for base_url and `300` for timeout (maintains backward compatibility)

### Test Pattern
Tests create client with new signature but manually construct struct with custom URL:
```rust
let client = AnthropicClient::new("test-key".to_string(), None, 300);
let base_url = format!("{}/v1/messages", server.url());
let client_with_url = AnthropicClient {
    api_key: "test-key".to_string(),
    client: client.client,
    base_url,  // custom URL from mock server
};
```

### Lessons Learned
- **LINE#ID hash mismatch**: When edit operations fail with hash mismatch, check if previous edits already succeeded (line content changes, hash ID updates)
- **Missing closing braces**: Use depth tracking to find unclosed delimiters (count `{` vs `}`)
- **Batch edits**: Re-read file after each edit to get updated LINE#ID tags for subsequent edits

## 2026-03-07 Task: Fix build_request_body to use config values

### Implementation Pattern: Adding Config Fields to Struct

When adding new fields derived from config to a struct:

1. **Add fields to struct** - Insert new fields in logical position:
   ```rust
   pub struct AnthropicClient {
       api_key: String,
       client: Client,
       base_url: String,
       model: String,      // NEW
       max_tokens: u32,    // NEW
   }
   ```

2. **Update Clone impl** - Must include ALL struct fields:
   ```rust
   impl Clone for AnthropicClient {
       fn clone(&self) -> Self {
           Self {
               api_key: self.api_key.clone(),
               client: self.client.clone(),
               base_url: self.base_url.clone(),
               model: self.model.clone(),      // NEW
               max_tokens: self.max_tokens,    // NEW
           }
       }
   }
   ```

3. **Initialize in constructor** - Use same values as hardcoded defaults for now:
   ```rust
   pub fn new(api_key: String, base_url: Option<&str>, timeout: u64) -> Self {
       // ... client setup ...
       Self {
           api_key,
           client,
           base_url: base_url.unwrap_or("...").to_string(),
           model: "claude-3-5-sonnet-20241022".to_string(),  // NEW
           max_tokens: 4096,                                   // NEW
       }
   }
   ```

4. **Clone values before async block** - In methods with async closures:
   ```rust
   pub async fn send(...) {
       let api_key = self.api_key.clone();
       let base_url = self.base_url.clone();
       let model = self.model.clone();        // NEW
       let max_tokens = self.max_tokens;      // NEW
       
       Box::pin(stream! {
           // Use cloned values in closure
           build_request_body(..., &model, max_tokens)
       })
   }
   ```

5. **Update function signatures** - Pass values explicitly:
   ```rust
   fn build_request_body(
       messages: &[Message],
       tools: &[Tool],
       model: &str,        // NEW parameter
       max_tokens: u32,    // NEW parameter
   ) -> Result<Value, LlmError>
   ```

6. **Use parameters in implementation** - Replace hardcoded values:
   ```rust
   let mut request = serde_json::json!({
       "model": model,        // Was: "claude-3-5-sonnet-20241022"
       "max_tokens": max_tokens,  // Was: 4096
       "messages": messages,
       "stream": true
   });
   ```

### Test Updates Required

When struct changes affect tests, update ALL struct initializations:

```rust
// Before
let client_with_url = AnthropicClient {
    api_key: "test-key".to_string(),
    client: client.client,
    base_url,
};

// After
let client_with_url = AnthropicClient {
    api_key: "test-key".to_string(),
    client: client.client,
    base_url,
    model: "claude-3-5-sonnet-20241022".to_string(),
    max_tokens: 4096,
};
```

### Key Learnings

- **Clone impl MUST include all fields** - Missing fields cause compilation errors
- **Values must be cloned before async closures** - Cannot borrow from `self` in async closures
- **Test struct initializations must match struct** - All tests creating struct manually need updates
- **Use same defaults as hardcoded values** - Ensures backward compatibility during transition
- **Compile without tests first** - Separate logic errors from test errors

## 2026-03-07 Task: Update agent_bridge to pass all config values

### Implementation Pattern: Full Config Passing

**Before (agent_bridge.rs line 61):**
```rust
let llm_client = AnthropicClient::new(api_key.clone());
```

**After:**
```rust
let llm_client = AnthropicClient::new(
    api_key.clone(),
    config.base_url.as_deref(),  // Option<String> -> Option<&str>
    config.timeout,
    &config.model,              // &String -> &str
    config.max_tokens,
);
```

### Key Pattern: Converting Config Types

When passing config values to functions with different type requirements:

1. **`Option<String>` to `Option<&str>`**: Use `.as_deref()`
   - `config.base_url.as_deref()` converts `Option<String>` to `Option<&str>`
   - Safe and idiomatic Rust pattern

2. **`String` to `&str`**: Use reference operator
   - `&config.model` converts `String` to `&str`
   - Works because we're passing a reference to owned data

3. **Direct passes for matching types**
   - `config.timeout` (u64) passes directly
   - `config.max_tokens` (u32) passes directly

### Test Updates Pattern

When function signatures change, update ALL test configs:

```rust
// Before
let config = LlmConfig {
    api_key: Some("test-key".to_string()),
    model: "claude-3-5-sonnet-20241022".to_string(),
    max_tokens: 4096,
    timeout: 60,
};

// After - must add base_url for all test configs
let config = LlmConfig {
    api_key: Some("test-key".to_string()),
    model: "claude-3-5-sonnet-20241022".to_string(),
    max_tokens: 4096,
    timeout: 60,
    base_url: None,  // NEW - required for all tests
};
```

### Files Modified

1. **limit-llm/src/client.rs**:
   - Updated `AnthropicClient::new()` signature to accept `model: &str` and `max_tokens: u32`
   - Updated struct initialization to use parameters instead of hardcoded values
   - Updated all 4 test calls to pass model and max_tokens parameters

2. **limit-cli/src/agent_bridge.rs**:
   - Updated client creation to pass all config values (base_url, timeout, model, max_tokens)
   - Updated 4 test configs to include `base_url: None`

3. **limit-cli/src/tui_bridge.rs**:
   - Updated 5 test configs to include `base_url: None`

4. **limit-cli/tests/e2e_test.rs**:
   - Updated 7 test configs to include `base_url: None`

5. **limit-cli/tests/tui_integration.rs**:
   - Updated 11 test configs to include `base_url: None`

### Lessons Learned

- **Batch test updates**: When struct fields change, ALL tests that create the struct must be updated
- **Type conversion patterns**: Use `.as_deref()` for `Option<String>` → `Option<&str>` and `&` for `String` → `&str`
- **Search for all usages**: Use grep to find all `let config = LlmConfig` patterns when adding new required fields
- **Fix duplicates systematically**: When sed/bash commands create duplicate lines, read and fix manually to avoid breaking code structure
- **Python scripts for complex fixes**: When file structure is broken by sed commands, use Python to systematically restructure
- **Verification order**: Fix compilation errors first, then test failures - separates concerns
