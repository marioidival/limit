# Browser Tool Documentation

Limit provides browser automation capabilities through the `browser` tool, enabling AI agents to interact with web pages for testing, scraping, and automation tasks.

## Requirements

The browser tool requires [agent-browser](https://github.com/marioidival/agent-browser) CLI to be installed:

```bash
# Install agent-browser
go install github.com/marioidival/agent-browser@latest
```

## Configuration

Add browser configuration to `~/.limit/config.toml`:

```toml
[browser]
enabled = true
engine = "chrome"        # or "lightpanda"
headless = true          # Run without visible window
timeout_ms = 30000       # Operation timeout
# binary_path = "/custom/path/agent-browser"  # Optional custom path
```

## Available Actions (46+)

### Core Actions

| Action | Description | Required Params |
|--------|-------------|-----------------|
| `open` | Navigate to URL | `url` |
| `close` | Close browser | - |
| `snapshot` | Get accessibility tree | - |

### Interaction Actions

| Action | Description | Required Params |
|--------|-------------|-----------------|
| `click` | Click element | `selector` |
| `dblclick` | Double-click element | `selector` |
| `fill` | Fill form field | `selector`, `text` |
| `type` | Type character by character | `selector`, `text` |
| `press` | Press keyboard key | `key` |
| `hover` | Hover over element | `selector` |
| `select` | Select dropdown option | `selector`, `value` |
| `focus` | Focus element | `selector` |
| `check` | Check checkbox | `selector` |
| `uncheck` | Uncheck checkbox | `selector` |
| `scrollintoview` | Scroll element into view | `selector` |
| `drag` | Drag and drop | `selector`, `target` |
| `upload` | Upload files | `selector`, `files` |

### Navigation Actions

| Action | Description | Required Params |
|--------|-------------|-----------------|
| `back` | Navigate back | - |
| `forward` | Navigate forward | - |
| `reload` | Reload page | - |

### Query Actions

| Action | Description | Required Params |
|--------|-------------|-----------------|
| `screenshot` | Take screenshot | `path` |
| `pdf` | Save as PDF | `path` |
| `eval` | Execute JavaScript | `script` |
| `get` | Get page content | `get_what` |
| `get_attr` | Get element attribute | `selector`, `attr` |
| `get_count` | Count matching elements | `selector` |
| `get_box` | Get element bounding box | `selector` |
| `get_styles` | Get computed styles | `selector` |
| `find` | Find element by locator | `locator_type`, `locator_value`, `find_action` |
| `is` | Check element state | `selector`, `what` |
| `download` | Download file | `selector`, `path` |

### Waiting Actions

| Action | Description | Required Params |
|--------|-------------|-----------------|
| `wait` | Generic wait | `wait_for` |
| `wait_for_text` | Wait for text to appear | `text` |
| `wait_for_url` | Wait for URL pattern | `value` |
| `wait_for_load` | Wait for load state | `state` |
| `wait_for_download` | Wait for download | `path` (optional) |
| `wait_for_fn` | Wait for JS function | `script` |
| `wait_for_state` | Wait for element state | `selector`, `state` |

### Tab & Dialog Actions

| Action | Description | Required Params |
|--------|-------------|-----------------|
| `tab_list` | List all tabs | - |
| `tab_new` | Open new tab | `url` (optional) |
| `tab_close` | Close tab | `index` (optional) |
| `tab_select` | Switch to tab | `index` |
| `dialog_accept` | Accept dialog | `dialog_text` (optional) |
| `dialog_dismiss` | Dismiss dialog | - |

### Storage & Network Actions

| Action | Description | Required Params |
|--------|-------------|-----------------|
| `cookies` | Get all cookies | - |
| `cookies_set` | Set cookie | `name`, `value` |
| `storage_get` | Get storage value | `storage_type`, `key_name` (optional) |
| `storage_set` | Set storage value | `storage_type`, `key_name`, `value` |
| `network_requests` | Get network requests | `filter` (optional) |

### Settings Actions

| Action | Description | Required Params |
|--------|-------------|-----------------|
| `set_viewport` | Set viewport size | `width`, `height`, `scale` (optional) |
| `set_device` | Emulate device | `device_name` |
| `set_geo` | Set geolocation | `latitude`, `longitude` |

### State Actions

| Action | Description | Required Params |
|--------|-------------|-----------------|
| `scroll` | Scroll page | `direction`, `pixels` (optional) |

## Usage Examples

### Basic Workflow

```
User: Open https://example.com and take a snapshot

[Agent uses browser tool:]
1. action: "open", url: "https://example.com"
2. action: "snapshot"
   → Returns accessibility tree with element refs
```

### Form Interaction

```
User: Fill the login form on the page

[Agent uses browser tool:]
1. action: "snapshot" → Get refs for form fields
2. action: "fill", selector: "ref=email", text: "user@example.com"
3. action: "fill", selector: "ref=password", text: "secret"
4. action: "click", selector: "ref=submit-button"
```

### Screenshot Capture

```
User: Take a screenshot of the page

[Agent uses browser tool:]
action: "screenshot", path: "/tmp/screenshot.png"
```

### Waiting for Content

```
User: Wait for the results to load

[Agent uses browser tool:]
action: "wait_for_text", text: "Results"
```

### Tab Management

```
User: Open a new tab and navigate to GitHub

[Agent uses browser tool:]
1. action: "tab_new"
2. action: "open", url: "https://github.com"
3. action: "tab_list" → Returns all tabs
```

## Snapshot-Ref Workflow

The recommended workflow for browser automation:

1. **Take a snapshot** - Get the accessibility tree with element references
2. **Use refs for interactions** - References like `ref=e1` are more reliable than CSS selectors
3. **Re-snapshot after navigation** - References change when page content changes

Example snapshot output:
```
- button "Submit" [ref=e1]
- textbox "Email" [ref=e2]
- textbox "Password" [ref=e3]
```

## Architecture

The browser tool is organized into extension traits for maintainability:

```
limit-cli/src/tools/browser/
├── client.rs           # Core client (open, close, executor access)
├── client_ext/
│   ├── navigation.rs   # back, forward, reload
│   ├── waiting.rs      # wait_for_*
│   ├── interaction.rs  # click, fill, type, press, etc.
│   ├── query.rs        # snapshot, screenshot, eval, get_*
│   ├── tabs.rs         # tab_*, dialog_*
│   └── storage.rs      # cookies, storage_*, network_*, set_*
├── config.rs           # BrowserConfig, BrowserEngine
├── executor.rs         # BrowserExecutor trait, CliExecutor
├── tool.rs             # LLM agent tool implementation
└── types.rs            # SnapshotResult, TabInfo, BoundingBox, etc.
```

## TUI Commands

In the TUI, you can use `/browser` commands:

```
/browser open https://example.com
/browser snapshot
/browser click "button.submit"
/browser fill "input.email" "user@example.com"
/browser screenshot /tmp/screenshot.png
/browser close
```

## Engine Support

| Engine | Description |
|--------|-------------|
| `chrome` | Google Chrome (default) |
| `lightpanda` | Lightweight browser for headless environments |

## Error Handling

The browser tool returns structured errors:

```json
{
  "error": "BrowserError::InvalidArguments: Selector cannot be empty"
}
```

Common error types:
- `InvalidArguments` - Missing or invalid parameters
- `ExecutionFailed` - Browser command failed
- `ParseError` - Failed to parse response
- `Timeout` - Operation timed out

## Best Practices

1. **Always snapshot first** - Get element references before interacting
2. **Use refs over selectors** - References are more stable
3. **Wait for navigation** - Use `wait_for_load` after page transitions
4. **Handle dialogs** - Accept or dismiss alerts/prompts explicitly
5. **Check element state** - Use `is` action before interacting with elements
