# Changelog

All notable changes to this project will be documented in this file.

## [0.0.18] - 2026-03-09

### 🐛 Bug Fixes

- *(llm)* [**breaking**] Change FunctionCall.arguments to String for bincode compatibility
- *(release)* Sync versions to 0.0.17 and fix sed bug

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to v0.0.17
- Add git-cliff configuration for changelog generation

## [0.0.18] - 2026-03-09

All notable changes to this project will be documented in this file.

## [0.0.17] - 2026-03-09

### 🚀 Features

- Add real-time streaming text feedback in TUI status bar
- *(tui)* Add ActivityFeed component for tool execution status
- *(cli)* Add TokenUsage event for LLM token tracking
- *(cli)* Add token usage display in REPL mode

### 🐛 Bug Fixes

- *(tui)* Resolve deadlock issues and add session management commands
- Increase TUI poll timeout to 500ms for better readability

### 🚜 Refactor

- Simplify TUI status bar to show activity only
- *(cli)* Integrate ActivityFeed and remove dead TUI code

### 🧪 Testing

- Update TUI tests for ActivityFeed behavior

### ⚙️ Miscellaneous Tasks

- Update sisyphus boulder state for zai-provider plan
- Add scripts release

## [0.0.17] - 2026-03-09

All notable changes to this project will be documented in this file.

## [0.0.16] - 2026-03-09

### 🐛 Bug Fixes

- Add text wrapping to input fields
- Increase input area height to accommodate wrapped text
- Chat area uses remaining screen space instead of fixed size
- Chat view text wrapping to display long messages properly
- Resolve duplicate message display in chat view
- Remove duplicate response display in TUI

### 📚 Documentation

- Fix url to download
- Add comprehensive LLM provider configuration guides
- Add concise LLM provider configuration guides
- Add plan for text wrapping fix

## [0.0.15] - 2026-03-08

### 🐛 Bug Fixes

- Use warn level in release builds to match tracing feature

### 🚜 Refactor

- Rename binary from limit to lim

### ⚙️ Miscellaneous Tasks

- Bump version to 0.0.15

## [0.0.14] - 2026-03-08

### 🐛 Bug Fixes

- *(ci)* Keep binary named 'limit' inside tar for install script

### ⚙️ Miscellaneous Tasks

- Bump version to 0.0.14

## [0.0.13] - 2026-03-08

### 🚜 Refactor

- *(ci)* Split build and release jobs, add checksums and caching

### ⚙️ Miscellaneous Tasks

- Bump version to 0.0.13

## [0.0.12] - 2026-03-08

### 🐛 Bug Fixes

- *(ci)* Use absolute path for tar.gz location
- *(ci)* Update softprops/action-gh-release to v2 and use absolute paths

### ⚙️ Miscellaneous Tasks

- Bump version - lock

## [0.0.11] - 2026-03-08

### 🐛 Bug Fixes

- *(ci)* Correct binary path in prepare step
- *(ci)* Use bundled sqlite for cross-compilation
- *(ci)* Use limit-{os}-{arch} naming for install script compatibility

### ⚙️ Miscellaneous Tasks

- Bump version to 0.0.11

## [0.0.10] - 2026-03-08

### 🚀 Features

- *(limit-llm)* Add config schema, error types, events, and types
- *(limit-llm)* Add Anthropic streaming client with mocks
- *(limit-llm)* Add SQLite tracking, binary persistence, and model hand-off
- Complete Wave 2 - tool execution, Docker sandbox, state management
- *(limit-cli)* Add REPL interface with rustyline
- *(limit-cli)* Add file and bash tools
- *(limit-cli)* Add git tools and markdown rendering
- Complete limit code agent MVP with TUI integration and E2E tests
- *(llm)* Add base_url config for custom API endpoints
- Add tracing-based logging system
- *(limit-llm)* Add multi-provider trait system
- *(types)* Add ReasoningDelta to ProviderResponseChunk
- *(config)* Add ZAI provider support
- *(provider)* Implement ZaiProvider with LlmProvider trait
- *(lib)* Export ZaiProvider
- Add system prompt configuration module
- Make TUI the default interface with --no-tui flag
- Add file-based logging for TUI debugging
- Add chat scrolling, fix slash commands, slow cursor blink
- Add scroll indicators to chat view
- Add syntax highlighting support
- *(tui)* Add markdown rendering to chat view
- Add token tracking to session database
- Track and display tokens in REPL interface
- Track and display tokens in TUI interface
- Add TokenUsage event for streaming token information
- Add logo asset

### 🐛 Bug Fixes

- Resolve remaining clippy warnings
- Resolve all clippy warnings
- Resolve all clippy warnings
- *(llm)* Correct SSE parsing and tool calls for z.ai API
- Duplicate message and tool execution
- Resolve infinite tool call loop by filtering system messages
- Resolve clippy warnings and format code
- *(cli)* Handle ReasoningDelta in agent_bridge
- *(clippy)* Use assert! for boolean comparisons in tests
- Ensure response is returned when hitting max iterations
- Resolve TUI issues (fragmented responses, input blocking, double borders)
- Improve line-based scrolling in chat view
- Use alternate screen buffer for proper TUI scrolling
- Correct crossterm imports for mouse capture (event module)
- Trackpad scrolling with pinned_to_bottom state
- Timezone and scroll buffer issues
- Resolve compilation errors in syntax highlighting
- Improve agent iteration limit handling
- Update rustyline Editor initialization
- Resolver warnings do clippy
- *(ci)* Remove non-existent git-chglog-release-action
- *(ci)* Switch reqwest to rustls-tls for cross-compilation

### 🚜 Refactor

- *(types)* Make Message.content Optional and add tool_call_id field
- Integrate system prompt and improve agent behavior
- Use for loop instead of while-let-on-iterator

### 📚 Documentation

- Update README with comprehensive project documentation
- Plans to zai sse parsing
- *(readme)* Add ZAI provider configuration
- Add implementation plans and learnings for ZAI provider and multi-provider support
- Update README with new features and architecture
- Agents & changelogs

### 🎨 Styling

- Fix cargo clippy

### 🧪 Testing

- *(zai)* Add comprehensive unit tests
- *(zai)* Add integration tests for config, env vars, and provider switching

### ⚙️ Miscellaneous Tasks

- Init
- Initial workspace setup with 4 crates
- Plan executed: multi provider support
- Add debug logs for SSE parsing investigation
- Remove debug logging from mouse scroll handling
- Add GitHub Actions workflow for releases
- Remove unused imports and suppress dead code warnings
- Remove trash
- Removing example files

<!-- generated by git-cliff -->
