# Browser Tool

Automate browser interactions directly from Limit using the agent-browser CLI.

## Quick Start

```bash
# 1. Install agent-browser
npm install -g agent-browser

# 2. Verify installation
agent-browser --version

# 3. Use in Limit TUI
lim> /browser open https://example.com
lim> /browser snapshot
```

That's it! The browser runs headless by default (no visible window).

## Prerequisites

### Required

- **agent-browser** - Browser automation CLI
- **Chrome** - Browser engine (default)

### Install agent-browser

Install agent-browser globally:

```bash
npm install -g agent-browser
```

Verify installation:

```bash
agent-browser --version
```

## Usage

### TUI Commands

Use the `/browser` command in the Limit TUI:

```
/browser open <url>        Open a URL in the browser
/browser close             Close the browser
/browser snapshot          Take an accessibility snapshot
/browser click <selector>  Click an element
/browser fill <sel> <text> Fill a form field
/browser screenshot <path> Save a screenshot
/browser get <what>        Get page content (text, html, url, title)
/browser help              Show help
```

**Alias**: `/b` (short for `/browser`)

#### Examples

```
# Open a website
/browser open https://example.com

# Take a snapshot to see the page structure
/browser snapshot

# Click a button
/browser click "button.submit"

# Fill a form field
/browser fill "input[name=email]" "test@example.com"

# Take a screenshot
/browser screenshot /tmp/page.png

# Get page title
/browser get title

# Close the browser
/browser close
```

### Agent Tool

The LLM can use the browser tool automatically. Just ask:

```
"Open example.com and take a screenshot"
"Go to github.com and find the trending repositories"
"Fill out the contact form on this website"
```

#### Available Actions

| Action | Description | Required Parameters |
|--------|-------------|---------------------|
| `open` | Open a URL | `url` |
| `close` | Close the browser | none |
| `snapshot` | Get accessibility tree | none |
| `click` | Click an element | `selector` |
| `fill` | Fill a form field | `selector`, `text` |
| `screenshot` | Save screenshot | `path` |
| `wait` | Wait for condition | `wait_for` |
| `eval` | Execute JavaScript | `script` |
| `get` | Get page content | `get_what` (text/html/value/url/title) |

### Workflow: Snapshot-Ref Pattern

The recommended workflow uses snapshots to discover element references:

1. **Open** the page
2. **Snapshot** to see the accessibility tree with element refs
3. **Interact** using refs from the snapshot (e.g., `ref="e3"`)
4. **Repeat** snapshot as needed to see changes

Example session:

```
> /browser open https://example.com
Opened: https://example.com

> /browser snapshot
Title: Example Domain
URL: https://example.com

--- Snapshot ---
- main [ref=e1]
  - heading "Example Domain" [ref=e2]
  - paragraph "This domain is for use in illustrative examples..." [ref=e3]
  - link "More information" [ref=e4]

> /browser click e4
Clicked: e4
```

## Configuration

### Default Behavior

The browser tool works out of the box with default settings:

| Option | Default | Description |
|--------|---------|-------------|
| `engine` | chrome | Browser engine (chrome or lightpanda) |
| `headless` | true | Run without visible window |
| `timeout_ms` | 30000 | Operation timeout in milliseconds |
| `binary_path` | `agent-browser` | Binary name (looks in PATH) |

**Default behavior:**
- Runs in headless mode (no visible browser window)
- Uses Chrome browser (must be installed)
- Looks for `agent-browser` in your PATH

### Custom Binary Location

If `agent-browser` is installed in a custom location, create a symlink or add it to your PATH:

```bash
# Option 1: Add to PATH
export PATH="$PATH:/path/to/agent-browser-directory"

# Option 2: Create symlink
ln -s /path/to/agent-browser /usr/local/bin/agent-browser
```

### Advanced Configuration (Programmatic)

For custom configuration, you can modify the source code or use the Rust API:

```rust
use limit_cli::tools::browser::{BrowserConfig, BrowserEngine, BrowserClient};
use std::sync::Arc;

// Create custom config
let config = BrowserConfig::new()
    .with_engine(BrowserEngine::Lightpanda)  // Use lightpanda instead of Chrome
    .with_headless(false)                     // Show browser window
    .with_timeout_ms(60_000)                  // 60 second timeout
    .with_binary_path(std::path::PathBuf::from("/custom/path/agent-browser"));

// Use with client
let executor = Arc::new(limit_cli::tools::browser::executor::CliExecutor::new(config));
let client = BrowserClient::new(executor);
```

### Browser Engines

| Engine | Description | Requirements |
|--------|-------------|--------------|
| `chrome` | Google Chrome (default) | Chrome must be installed |
| `lightpanda` | Lightweight browser | No external dependencies |

### Config File Support (Planned)

Future versions will support `~/.limit/config.toml`:

```toml
# Planned syntax (not yet implemented)
[browser]
engine = "chrome"
headless = true
timeout_ms = 30000
# binary_path = "/custom/path/agent-browser"  # optional
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Limit                            │
├─────────────────────────────────────────────────────┤
│  TUI Command          Agent Tool       Internal     │
│  `/browser`    ←→    `browser`    ←→   API          │
│       │                   │                │        │
│       └───────────────────┼────────────────┘        │
│                           ▼                         │
│              ┌─────────────────────┐                │
│              │   BrowserExecutor   │                │
│              │   (trait)           │                │
│              └─────────────────────┘                │
│                           │                         │
│                           ▼                         │
│              ┌─────────────────────┐                │
│              │    CliExecutor      │                │
│              │  (agent-browser)    │                │
│              └─────────────────────┘                │
└─────────────────────────────────────────────────────┘
```

## Programmatic Usage

```rust
use limit_cli::tools::browser::{BrowserClient, BrowserConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let client = BrowserClient::with_default_config();

    // Open a page
    client.open("https://example.com").await.unwrap();

    // Take a snapshot
    let snapshot = client.snapshot().await.unwrap();
    println!("Title: {:?}", snapshot.title);
    println!("Content:\n{}", snapshot.content);

    // Take a screenshot
    client.screenshot("/tmp/screenshot.png").await.unwrap();

    // Get page text
    let text = client.get("text").await.unwrap();
    println!("Page text: {}", text);

    // Close browser
    client.close().await.unwrap();
}
```

## Troubleshooting

### "Browser binary not found"

Install agent-browser:

```bash
npm install -g agent-browser
```

### "Failed to open URL"

Ensure Chrome is installed (for Chrome engine) or use Lightpanda:

```bash
# Check Chrome
google-chrome --version  # Linux
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --version  # macOS
```

### Timeout errors

Increase the timeout:

```rust
let config = BrowserConfig::new()
    .with_timeout_ms(60_000);  // 60 seconds
```

### Browser not closing

The browser automatically closes when:
- You run `/browser close`
- The Limit session ends (via Drop trait)

If stuck, manually close:

```bash
pkill -f agent-browser
```

## Related

- [agent-browser](https://github.com/vercel-labs/agent-browser) - Vercel Labs browser automation CLI
- [Configuration](CONFIGURATION.md) - General Limit configuration
