# Agent-Browser (Vercel Labs) + Limit Integration Analysis

> **Analysis Date:** 2026-03-14  
> **Author:** OpenClaw  
> **Status:** Draft  

---

## Executive Summary

**agent-browser** é um **CLI nativo em Rust** para browser automation, otimizado para AI agents. Diferente do Stagehand (framework TypeScript), o agent-browser é:
- **CLI-first** (não biblioteca)
- **Rust-native** (binário compilado, sem Node.js)
- **CDP-based** (Chrome DevTools Protocol)
- **Snapshot-ref workflow** (ótimo para AI)

A integração com **Limit** é **MUITO mais simples** e **natural** porque ambos são Rust!

---

## Projects Overview

### Limit (Rust CLI)

**Type:** AI Pair Programmer (Terminal)

**Stack:**
- Language: **Rust**
- LLM: Multi-provider
- UI: TUI + REPL
- Tools: 17 built-in

**Current Tools:**
- File I/O, Bash, Git, Code Analysis, Web (search/fetch)

---

### agent-browser (Rust CLI)

**Type:** Headless Browser Automation CLI

**Stack:**
- Language: **Rust** ✅
- Browser: Chrome (CDP)
- Distribution: Binary (npm, cargo, homebrew)
- Architecture: Client-daemon

**Key Features:**
- **Snapshot-based**: Get accessibility tree with refs (@e1, @e2)
- **Ref interaction**: Click @e1, fill @e2 "text"
- **No Node.js required** (daemon is pure Rust)
- **JSON mode**: `--json` for machine-readable output
- **Fast**: Native Rust binary

**Commands:**
```bash
# Core workflow
agent-browser open example.com
agent-browser snapshot                    # Get refs
agent-browser click @e2                   # Click by ref
agent-browser fill @e3 "test@example.com" # Fill by ref
agent-browser screenshot page.png
agent-browser close

# Also supports CSS selectors
agent-browser click "#submit"
agent-browser fill "#email" "text"
```

---

## Integration Opportunities

### 1. **Browser Tool for Limit (Rust-to-Rust!)**

Since both are Rust, integration is **MUCH simpler** than Stagehand:

**Option A: Direct Binary Invocation**

```rust
// limit-agent/src/tool/browser.rs

use async_trait::async_trait;
use crate::tool::Tool;
use tokio::process::Command;

pub struct BrowserTool;

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }
    
    fn description(&self) -> &str {
        "Control a web browser with agent-browser CLI. Navigate, click, extract data, and more."
    }
    
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["open", "snapshot", "click", "fill", "type", "screenshot", "close"],
                    "description": "Browser action"
                },
                "url": { "type": "string" },
                "selector": { "type": "string" },
                "text": { "type": "string" },
                "path": { "type": "string" }
            },
            "required": ["action"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let action = params["action"].as_str().unwrap();
        
        match action {
            "open" => {
                let url = params["url"].as_str().unwrap();
                let output = Command::new("agent-browser")
                    .args(&["open", url])
                    .output()
                    .await?;
                
                Ok(json!({ "success": output.status.success(), "url": url }))
            }
            
            "snapshot" => {
                let output = Command::new("agent-browser")
                    .args(&["snapshot", "-i", "--json"])
                    .output()
                    .await?;
                
                let stdout = String::from_utf8_lossy(&output.stdout);
                let snapshot: serde_json::Value = serde_json::from_str(&stdout)?;
                Ok(snapshot)
            }
            
            "click" => {
                let selector = params["selector"].as_str().unwrap();
                let output = Command::new("agent-browser")
                    .args(&["click", selector])
                    .output()
                    .await?;
                
                Ok(json!({ "success": output.status.success() }))
            }
            
            "fill" => {
                let selector = params["selector"].as_str().unwrap();
                let text = params["text"].as_str().unwrap();
                let output = Command::new("agent-browser")
                    .args(&["fill", selector, text])
                    .output()
                    .await?;
                
                Ok(json!({ "success": output.status.success() }))
            }
            
            "screenshot" => {
                let path = params["path"].as_str().unwrap_or("screenshot.png");
                let output = Command::new("agent-browser")
                    .args(&["screenshot", path])
                    .output()
                    .await?;
                
                Ok(json!({ "success": output.status.success(), "path": path }))
            }
            
            "close" => {
                let output = Command::new("agent-browser")
                    .args(&["close"])
                    .output()
                    .await?;
                
                Ok(json!({ "success": output.status.success() }))
            }
            
            _ => Err(ToolError::InvalidAction(action.to_string()))
        }
    }
}
```

**Benefits:**
- ✅ No Node.js dependency
- ✅ No bridge needed
- ✅ Direct binary invocation (fast)
- ✅ Same language (Rust)
- ✅ Easy to bundle (static binary)

---

### 2. **Library Integration (Advanced)**

Use agent-browser as a Rust library (crate):

```toml
# limit-agent/Cargo.toml

## VEJA SE é possivel usa-lo como library
[dependencies]
agent-browser = { git = "https://github.com/vercel-labs/agent-browser" }
```

```rust
// limit-agent/src/tool/browser.rs

use agent_browser::{BrowserManager, BrowserOptions};
use async_trait::async_trait;
use crate::tool::Tool;

pub struct BrowserTool {
    browser: BrowserManager,
}

impl BrowserTool {
    pub async fn new() -> Result<Self, ToolError> {
        let browser = BrowserManager::new().await?;
        Ok(Self { browser })
    }
}

#[async_trait]
impl Tool for BrowserTool {
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let action = params["action"].as_str().unwrap();
        
        match action {
            "open" => {
                let url = params["url"].as_str().unwrap();
                self.browser.navigate(url).await?;
                Ok(json!({ "success": true, "url": url }))
            }
            
            "snapshot" => {
                let snapshot = self.browser.snapshot().await?;
                Ok(json!({ "snapshot": snapshot }))
            }
            
            "click" => {
                let selector = params["selector"].as_str().unwrap();
                self.browser.click(selector).await?;
                Ok(json!({ "success": true }))
            }
            
            _ => Err(ToolError::InvalidAction(action.to_string()))
        }
    }
}
```

**Benefits:**
- ✅ No CLI overhead
- ✅ In-process browser control
- ✅ Shared browser instance (faster)
- ✅ More control

**Drawbacks:**
- ⚠️ Larger binary size
- ⚠️ Tighter coupling
- ⚠️ Need to expose library API (currently CLI-only)

---

### 3. **AI-Native Workflow**

Leverage agent-browser's **snapshot-ref** pattern:

```
1. AI asks for snapshot
   lim> What elements are on the page?
   
   [Tool: browser]
   → agent-browser snapshot -i --json
   
   Result:
   {
     "elements": [
       { "ref": "@e1", "role": "button", "name": "Submit" },
       { "ref": "@e2", "role": "textbox", "name": "Email" },
       { "ref": "@e3", "role": "link", "name": "Home" }
     ]
   }
   
2. AI identifies target
   "I see a Submit button (@e1) and Email field (@e2)"
   
3. AI executes action
   lim> Fill the email field with "test@example.com"
   
   [Tool: browser]
   → agent-browser fill @e2 "test@example.com"
   
4. AI continues
   lim> Click the submit button
   
   [Tool: browser]
   → agent-browser click @e1
```

---

## Comparison: agent-browser vs Stagehand

| Feature | agent-browser | Stagehand |
|---------|--------------|-----------|
| **Language** | Rust | TypeScript |
| **Distribution** | Binary | npm package |
| **Runtime** | Native (no Node.js) | Node.js required |
| **Integration** | CLI invocation or library | Node.js bridge |
| **AI Workflow** | Snapshot + refs | Natural language actions |
| **Performance** | Fast (native) | Medium (Node.js overhead) |
| **Learning Curve** | Low (simple CLI) | Medium (API + LLM) |
| **Limit Compatibility** | ✅ Perfect (both Rust) | ⚠️ Bridge needed |
| **Binary Size** | ~20MB | ~50MB (Node.js) |

---

## Recommended Integration Strategy

### Phase 1: CLI Tool (Simple, Fast)

**Implementation:** Direct binary invocation

**Pros:**
- ✅ Easiest to implement
- ✅ No dependencies (just binary)
- ✅ Clean separation
- ✅ Works immediately

**Cons:**
- ⚠️ CLI overhead (process spawn)
- ⚠️ Browser restart on each Limit session

**Timeline:** 1-2 days

---

### Phase 2: Bundled Binary (Optimized)

**Implementation:** Bundle agent-browser binary with Limit

**Pros:**
- ✅ Zero external dependencies
- ✅ Single download
- ✅ Version-controlled

**Cons:**
- ⚠️ Larger download (~20MB)
- ⚠️ Platform-specific binaries

**Timeline:** 2-3 days

---

### Phase 3: Library Integration (Advanced)

**Implementation:** Use agent-browser as Rust crate

**Pros:**
- ✅ No CLI overhead
- ✅ In-process browser
- ✅ Shared browser instance

**Cons:**
- ⚠️ Requires library API (not yet exposed)
- ⚠️ Larger binary
- ⚠️ Tighter coupling

**Timeline:** 5-7 days (after library API is ready)

---

## Implementation Example

### Tool Definition

```rust
// limit-agent/src/tool/browser.rs

use async_trait::async_trait;
use crate::tool::{Tool, ToolError};
use serde_json::json;
use tokio::process::Command;

pub struct BrowserTool;

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }
    
    fn description(&self) -> &str {
        "Control a web browser for automation, testing, and scraping. Uses agent-browser CLI."
    }
    
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "open", "close", "snapshot", "click", "fill", 
                        "type", "screenshot", "wait", "eval", "get"
                    ],
                    "description": "Browser action to perform"
                },
                "url": {
                    "type": "string",
                    "description": "URL to navigate to (for 'open')"
                },
                "selector": {
                    "type": "string",
                    "description": "Element selector (ref like @e1, CSS, or text)"
                },
                "text": {
                    "type": "string",
                    "description": "Text to type or fill"
                },
                "path": {
                    "type": "string",
                    "description": "File path for screenshot"
                },
                "wait_for": {
                    "type": "string",
                    "description": "What to wait for (selector, milliseconds, or 'networkidle')"
                },
                "script": {
                    "type": "string",
                    "description": "JavaScript to evaluate"
                },
                "get_what": {
                    "type": "string",
                    "enum": ["text", "html", "value", "url", "title"],
                    "description": "What to get (for 'get' action)"
                }
            },
            "required": ["action"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let action = params["action"].as_str()
            .ok_or(ToolError::MissingParameter("action"))?;
        
        match action {
            "open" => self.open(&params).await,
            "close" => self.close().await,
            "snapshot" => self.snapshot().await,
            "click" => self.click(&params).await,
            "fill" => self.fill(&params).await,
            "type" => self.type_text(&params).await,
            "screenshot" => self.screenshot(&params).await,
            "wait" => self.wait(&params).await,
            "eval" => self.eval(&params).await,
            "get" => self.get(&params).await,
            _ => Err(ToolError::InvalidAction(action.to_string())),
        }
    }
}

impl BrowserTool {
    async fn open(&self, params: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let url = params["url"].as_str()
            .ok_or(ToolError::MissingParameter("url"))?;
        
        let output = Command::new("agent-browser")
            .args(&["open", url])
            .output()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to run agent-browser: {}", e)))?;
        
        if output.status.success() {
            Ok(json!({ "success": true, "url": url }))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ToolError::ExecutionError(format!("Failed to open URL: {}", stderr)))
        }
    }
    
    async fn close(&self) -> Result<serde_json::Value, ToolError> {
        let output = Command::new("agent-browser")
            .args(&["close"])
            .output()
            .await?;
        
        Ok(json!({ "success": output.status.success() }))
    }
    
    async fn snapshot(&self) -> Result<serde_json::Value, ToolError> {
        let output = Command::new("agent-browser")
            .args(&["snapshot", "-i", "--json"])
            .output()
            .await?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let snapshot: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| ToolError::ParseError(format!("Failed to parse snapshot: {}", e)))?;
        
        Ok(snapshot)
    }
    
    async fn click(&self, params: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let selector = params["selector"].as_str()
            .ok_or(ToolError::MissingParameter("selector"))?;
        
        let output = Command::new("agent-browser")
            .args(&["click", selector])
            .output()
            .await?;
        
        Ok(json!({ "success": output.status.success() }))
    }
    
    async fn fill(&self, params: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let selector = params["selector"].as_str()
            .ok_or(ToolError::MissingParameter("selector"))?;
        let text = params["text"].as_str()
            .ok_or(ToolError::MissingParameter("text"))?;
        
        let output = Command::new("agent-browser")
            .args(&["fill", selector, text])
            .output()
            .await?;
        
        Ok(json!({ "success": output.status.success() }))
    }
    
    async fn screenshot(&self, params: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path = params["path"].as_str().unwrap_or("screenshot.png");
        
        let output = Command::new("agent-browser")
            .args(&["screenshot", path])
            .output()
            .await?;
        
        if output.status.success() {
            Ok(json!({ "success": true, "path": path }))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ToolError::ExecutionError(format!("Screenshot failed: {}", stderr)))
        }
    }
    
    // ... other methods
}
```

---

### Tool Registration

```rust
// limit-agent/src/registry.rs

use crate::tool::{browser::BrowserTool, Tool};

pub fn register_tools() -> Vec<Box<dyn Tool>> {
    vec![
        // ... existing tools
        Box::new(BrowserTool),
    ]
}
```

---

### Configuration

```toml
# ~/.limit/config.toml

[browser]
# Enable browser tool
enabled = true

# Path to agent-browser binary (auto-detect if not set)
# binary_path = "/usr/local/bin/agent-browser"

# Headless mode (no GUI)
headless = true

# Screenshot directory
screenshot_dir = "/tmp/limit-screenshots"

# Default timeout (ms)
timeout = 25000
```

---

## Use Cases

### 1. **Web Testing**

```
lim> Test the login flow at localhost:3000:
     1. Navigate to /login
     2. Fill email "test@example.com"
     3. Fill password "password123"
     4. Click submit
     5. Verify redirect to /dashboard

[Tool: browser]
→ open http://localhost:3000/login
→ snapshot (get refs)
→ fill @e2 "test@example.com"
→ fill @e3 "password123"
→ click @e4
→ get url

✅ Test passed! Redirected to /dashboard
```

---

### 2. **Web Scraping**

```
lim> Extract the latest 10 blog posts from example.com/blog

[Tool: browser]
→ open https://example.com/blog
→ snapshot -i --json
→ eval "Array.from(document.querySelectorAll('.post')).map(p => p.innerText)"

✅ Extracted 10 posts:
1. "Introduction to Rust" ...
2. "Building CLI Tools" ...
```

---

### 3. **Visual Debugging**

```
lim> Debug the CSS bug on the homepage. Take a screenshot.

[Tool: browser]
→ open https://myapp.com
→ screenshot debug.png

✅ Screenshot saved to: /tmp/limit-screenshots/debug.png
```

---

## Benefits for Limit

### 1. **Native Rust Integration**

| Feature | agent-browser | Stagehand |
|---------|--------------|-----------|
| **Language Match** | ✅ Rust-to-Rust | ❌ Rust-to-TypeScript |
| **Bridge Needed** | ❌ No | ✅ Yes (Node.js) |
| **Performance** | ✅ Fast | ⚠️ Medium |
| **Binary Size** | ✅ Small (~20MB) | ⚠️ Large (~50MB) |

---

### 2. **Simpler Architecture**

```
Stagehand Integration:
Limit (Rust) → Node.js Bridge → Stagehand (TS) → Chrome

agent-browser Integration:
Limit (Rust) → agent-browser (Rust) → Chrome

Or (Phase 3):
Limit (Rust) + agent-browser (same process) → Chrome
```

---

### 3. **Zero External Dependencies**

- No Node.js required
- No npm packages
- Just one binary

---

### 4. **Perfect AI Workflow**

The **snapshot-ref** pattern is ideal for AI:
1. Get snapshot (elements with refs)
2. AI identifies target ref
3. Execute action with ref
4. Repeat

---

## Implementation Roadmap

### Phase 1: Basic Browser Tool (Week 1)

**Goals:**
- ✅ Add `browser` tool
- ✅ Basic actions: open, close, snapshot, click, fill, screenshot
- ✅ CLI invocation

**Deliverables:**
```rust
// limit-agent/src/tool/browser.rs
pub struct BrowserTool;

// Actions:
- open(url)
- close()
- snapshot()
- click(selector)
- fill(selector, text)
- screenshot(path)
```

**Success Criteria:**
- Can navigate to URL
- Can take screenshot
- Can click elements by ref

---

### Phase 2: Enhanced Actions (Week 2)

**Goals:**
- ✅ More actions: wait, eval, get
- ✅ Error handling
- ✅ Configuration

**Deliverables:**
```rust
// Additional actions:
- wait(selector)
- wait(ms)
- eval(script)
- get(text|html|value|url|title)
```

---

### Phase 3: Bundled Binary (Week 3)

**Goals:**
- ✅ Bundle agent-browser with Limit
- ✅ Auto-install on first use
- ✅ Version management

**Deliverables:**
```bash
# Install Limit + agent-browser together
cargo install limit

# First run auto-installs agent-browser
lim> browser open example.com
→ Installing agent-browser...
→ Done!
```

---

### Phase 4: Library Integration (Future)

**Goals:**
- ⚠️ Wait for agent-browser to expose library API
- ⚠️ In-process browser control

---

## Technical Considerations

### 1. **Binary Availability**

| Platform | Binary Available |
|----------|-----------------|
| macOS ARM64 | ✅ Yes |
| macOS x64 | ✅ Yes |
| Linux ARM64 | ✅ Yes |
| Linux x64 | ✅ Yes |
| Windows x64 | ✅ Yes |

---

### 2. **Chrome Installation**

agent-browser requires Chrome:
```bash
# Auto-install Chrome
agent-browser install
```

**Option 1:** Require user to install
**Option 2:** Auto-install on first use
**Option 3:** Bundle Chrome (larger download)

---

### 3. **Performance**

| Operation | Time |
|-----------|------|
| Browser startup | 1-2s |
| Navigation | 1-3s |
| Snapshot | <100ms |
| Click/Fill | <100ms |
| Screenshot | <200ms |

---

### 4. **Security**

| Risk | Mitigation |
|------|------------|
| Malicious websites | Sandbox (Docker) |
| Credential theft | Never store credentials |
| Data exfiltration | Log all actions |
| DoS | Rate limit |

---

## Comparison Summary

### agent-browser vs Stagehand

| Aspect | agent-browser | Stagehand | Winner |
|--------|--------------|-----------|--------|
| **Limit Compatibility** | ✅ Perfect (Rust) | ⚠️ Bridge needed | **agent-browser** |
| **Performance** | ✅ Native | ⚠️ Node.js overhead | **agent-browser** |
| **Ease of Integration** | ✅ Simple | ⚠️ Complex | **agent-browser** |
| **AI Capabilities** | ⚠️ Manual refs | ✅ Natural language | Stagehand |
| **Bundle Size** | ✅ Small (~20MB) | ⚠️ Large (~50MB) | **agent-browser** |
| **Dependencies** | ✅ Zero external | ⚠️ Node.js required | **agent-browser** |

---

## Recommendation

### ✅ **Use agent-browser for Limit**

**Rationale:**
1. ✅ **Perfect match**: Both are Rust
2. ✅ **Zero bridge**: Direct binary invocation
3. ✅ **Fast**: Native performance
4. ✅ **Simple**: Easy to implement
5. ✅ **Lightweight**: Small binary
6. ✅ **AI-friendly**: Snapshot-ref pattern

**Next Steps:**
1. Install agent-browser: `npm install -g agent-browser`
2. Add `browser` tool to Limit
3. Implement basic actions (open, snapshot, click, fill, screenshot)
4. Test with Limit CLI
5. Bundle agent-browser with Limit

**Timeline:** 1-2 weeks for full integration

---

## Open Questions

1. **Library API?**
   - Does agent-browser expose Rust library API?
   - If not, CLI invocation is fine

2. **Chrome bundling?**
   - Bundle Chrome or require user to install?
   - Trade-off: size vs convenience

3. **Session persistence?**
   - Keep browser open between tool calls?
   - agent-browser uses daemon by default

---

## References

- **agent-browser:** https://github.com/vercel-labs/agent-browser
- **agent-browser Docs:** https://agent-browser.dev
- **Limit:** https://github.com/marioidival/limit
- **Chrome DevTools Protocol:** https://chromedevtools.github.io/devtools-protocol/

---

## Appendix: Example Session

```bash
# Start Limit
lim> Use the browser to test the login flow at localhost:3000

# Limit executes:
[Tool: browser]
→ open http://localhost:3000/login
→ snapshot -i --json

# Limit analyzes snapshot:
"Found email field (@e2), password field (@e3), and submit button (@e4)"

# Limit continues:
→ fill @e2 "test@example.com"
→ fill @e3 "password123"
→ click @e4
→ wait --url "**/dashboard"
→ get url

# Result:
✅ Login successful! Redirected to: http://localhost:3000/dashboard
```

---

**End of Analysis**
