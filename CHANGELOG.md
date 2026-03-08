# Changelog

All notable changes to this project will be documented in this file.

## [0.0.12] - 2026-03-08

### 🚀 Features

- *(tui)* Fix session behavior to always create new sessions
- *(tui)* Auto-save sessions after each LLM response
- *(tui)* Display session ID in status bar (last 8 chars)
- *(tui)* Add welcome message for new TUI sessions
- *(tui)* Enhanced logging for session save operations

### 🐛 Bug Fixes

- *(tui)* Fix sessions being reused instead of creating new ones
- *(tests)* Fix test files to unwrap TuiBridge::new() Result
- *(tests)* Remove unused test helper accessing private SessionManager fields
- *(clippy)* Remove unused methods from AgentBridge
- *(clippy)* Resolve all clippy warnings and errors

### 📚 Documentation

- Add TUI_SESSION_NOTES.md with implementation details
- Add FIX_SUMMARY_TUI_SESSION.md with before/after comparison
- Add CLIPPY_FIXES_SUMMARY.md with all fixes documented
- Add SESSION_COMPLETE_SUMMARY.md with complete overview

### ⚙️ Miscellaneous Tasks

- Bump version to 0.0.12

## [0.0.11] - 2026-03-08

### 🚀 Features

- *(tui)* Add session persistence to TUI interface
- *(tui)* Load previous session messages on startup
- *(tui)* Display conversation history in TUI chat view
- *(tui)* Auto-save session after each LLM response
- *(tui)* Display token counts in TUI title bar
- *(tui)* Filter system/tool messages from display
- Add test script for TUI session verification

### 🐛 Bug Fixes

- Resolve compilation errors in TuiBridge session integration
- Fix session ID propagation in TUI event loop

### 📚 Documentation

- Add TUI_SESSION_NOTES.md with implementation details

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
