# Changelog

All notable changes to this project will be documented in this file.

## [0.0.30] - 2026-03-21

### 🐛 Bug Fixes

- Publish

## [0.0.30] - 2026-03-20

All notable changes to this project will be documented in this file.

## [0.0.29] - 2026-03-20

### 🚀 Features

- Add limit-tldr crate for code analysis

- Expose TLDR tool in agent registry

- Make CallGraphCache Clone for thread sharing

- Wire TreeSitterParser into AST layer

- Implement call graph extraction with tree-sitter

- Add debug logging for tldr_analyze tool invocation

- Add explicit triggers to tldr_analyze tool description

- Add source analysis type to get function code

- Add missing public API methods

- Implement real CFG with McCabe cyclomatic complexity

- Add CFG basic block splitting with edges

- Implement real DFG with statement-scoped data flows

- Implement backward program slicing via PDG

- Implement embedding-based semantic search with text fallback

- Add disambiguation and class lookup to ASTLayer

- Add find_all_functions and project_path to public API

- Include structs/classes in semantic search index

- Rewrite warm() with parallel parsing, incremental cache, and semantic persistence

- Add pre-warm TLDR on startup with smart freshness detection

- Enable semantic feature by default


### 🐛 Bug Fixes

- Resolve all clippy warnings in limit-tldr

- Resolve clippy warnings in TLDR implementation

- Add #[cfg(unix)] guard to daemon module for Windows cross-compilation

- Return cached TLDR instance instead of creating new one

- Filter functions with empty file paths on cache load

- Use blake3 hash for cache path to prevent collisions

- Allow type_complexity for TldrTool cache

- Discourage combining tldr_analyze with file_read

- Limit Architecture output to counts + samples

- Add stronger system prompt instruction to prefer tldr_analyze

- Use OnceCell to prevent race condition in warm()

- Fix PHP language constant and mut model in semantic search

- Tighten system prompt to reduce token cost

- Truncate large tool results to prevent context bloat

- Add efficiency strategy to tool description

- Disambiguate Source by file param, return relative paths

- Add signatures and relative paths to Search results

- Update tool description with disambiguation hints

- Log errors before propagating from analyze()

- Extract only signature line, not full function body

- Resolve multiple source lookup failures causing excessive tokens

- Skip test_tui_app_new in headless environments

- Remove env var mutation from zai validation test

- Fix CI test failure and relocate fastembed cache


### 🚜 Refactor

- Convert limit-tldr to pure library

- Remove duplicate TldrTool, re-export from limit-agent

- Remove direct limit-tldr dependency

- Implement file path matching and remove daemon

- Extract find_function_node to shared parser utility

- Move warm_guard to limit-cli

- Move tldr tool to limit-cli

- Wire tldr and warm_guard modules in limit-cli

- Remove tools module from limit-agent

- Remove tool result truncation

- Use .gitignore for file discovery


### 📚 Documentation

- Add TLDR analysis workflow to system prompt

- Add warm() rewrite design

- Add pre-warm startup design


### 🎨 Styling

- Apply cargo fmt to limit-tldr


### 🧪 Testing

- Add comprehensive test suite to limit-tldr

- Add failing test for AST line numbers (RED phase)

- Add failing test for file path matching


### ⚙️ Miscellaneous Tasks

- Update Cargo.lock for limit-tldr

- Add GitHub Actions workflow for cargo test

- Remove dead layer caches and unused imports

- Add .worktrees/ to .gitignore

- Add fastembed v5 dependency for semantic search

- Apply cargo fmt and update Cargo.lock

- Run workflow only on pull requests

- Remove plan docs from branch

- Bump limit-tldr version to 0.0.1

- Update Cargo.lock for limit-tldr 0.0.1

- Add limit-tldr and walkdir deps to limit-cli

- Update Cargo.lock

- Remove simulate_session example

- Add limit-tldr to release script

## [0.0.29] - 2026-03-20

All notable changes to this project will be documented in this file.

## [0.0.28] - 2026-03-15

### 🚀 Features

- Add detailed browser activity messages in TUI


### 🐛 Bug Fixes

- Share command doesnt copy prompt


### 🚜 Refactor

- Remove duplicate tool list from system prompt


### 📚 Documentation

- Add comprehensive documentation and module-level docs

- Add comprehensive documentation for tool system

- Add comprehensive documentation for TUI components

- Add comprehensive documentation for CLI application


### 🎨 Styling

- Format event and tool definitions

## [0.0.28] - 2026-03-15

All notable changes to this project will be documented in this file.

## [0.0.27] - 2026-03-15

### 🚀 Features

- Add input history with persistence

- Integrate history into input editor

- Export InputHistory from input module

- Increase size of state events

- Improve pasted content display with placeholder

- Add browser automation tool for agent and TUI (#9)


### 🐛 Bug Fixes

- Use arrow keys for history navigation instead of scroll

- Correct message count assertions in tui_integration

- Resolve test isolation and syntax detection issues

- Complete tui module doc example with config setup


### 🚜 Refactor

- Add debug logging to input handler

- Add factory methods for dependency injection

- Add ArgsExt trait and Response builder

- Add From<BrowserError> for AgentError

- Add BrowserAction enum for type-safe dispatch

- Split handlers into categorical modules


### 🎨 Styling

- Apply cargo fmt to autocomplete

- Apply cargo fmt to bridge_impl

- Apply cargo fmt to renderer


### ⚙️ Miscellaneous Tasks

- Remove plans

## [0.0.27] - 2026-03-15

All notable changes to this project will be documented in this file.

## [0.0.26] - 2026-03-13

### 🚀 Features

- Complete command system implementation and code formatting


### 🐛 Bug Fixes

- Resolve clippy warning for unused parameter

- Prevent text deletion before @ in file autocomplete

- Add detailed debug logs for autocomplete investigation

- Use selected_match() instead of accept_completion() for file autocomplete


### 🚜 Refactor

- Phase 1 - Extract state types to separate module

- Phase 2 - Extract input handling to separate module

- Phase 3 - Extract command system to separate module

- Phase 4 - Extract UI rendering to separate module

- Integrate InputHandler into TuiApp

- Remove unused imports from command modules

- Use InputHandler for cursor blink and ESC timing

- Phase 5 - Split TuiBridge and TuiApp into separate modules

- Extract activity message formatting to separate module

- Extract input text editor to separate module

- Improve TuiState and remove deprecated code

- Extract file autocomplete manager to separate module

- Update module structure and remove deprecated exports

- Remove deprecated debug_log function

- Consolidate imports in app_impl.rs

- Clean up command registry

- Remove unnecessary clones in OpenAI provider


### 📚 Documentation

- Crates


### ⚡ Performance

- Optimize input/handler.rs for hot paths

- Optimize renderer with stack allocation and inlining

- Add mutex poison recovery in bridge getters


### 🧪 Testing

- Fix integration tests to use dynamic operation_id

- Add comprehensive tests for InputEditor and FileAutocompleteManager

## [0.0.26] - 2026-03-13

All notable changes to this project will be documented in this file.

## [0.0.25] - 2026-03-13

### 🚀 Features

- Add operation cancellation support with double ESC


### 🧪 Testing

- Update tests for operation_id in AgentEvent

## [0.0.25] - 2026-03-13

All notable changes to this project will be documented in this file.

## [0.0.24] - 2026-03-12

### 🚀 Features

- Add session share/export with /share command


### 🐛 Bug Fixes

- Correct content accumulation and message persistence in agent loop

- Prevent race conditions and clear chat on new session


### 📚 Documentation

- Add /share command documentation

## [0.0.24] - 2026-03-12

All notable changes to this project will be documented in this file.

## [0.0.23] - 2026-03-12

### 🚀 Features

- File autocomplete with fuzzy matching (#8)


### 🐛 Bug Fixes

- Git cliff config

## [0.0.23] - 2026-03-12

All notable changes to this project will be documented in this file.

## [0.0.22] - 2026-03-12

### 🚀 Features

- *(cli)* Add --version flag support

### 🐛 Bug Fixes

- *(tui)* Correct scroll behavior when pinned to bottom
- *(tui)* Resolve chat view scroll asymmetry issue
- *(tui)* Improve clipboard and mouse selection handling
- Script release

### 📚 Documentation

- Update LLM models to latest 2026 versions

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to v0.0.21
- Bump version to 0.0.21
- Add folder to gitignore

## [0.0.22] - 2026-03-11

All notable changes to this project will be documented in this file.

## [0.0.21] - 2026-03-11

### 🚀 Features

- Add local LLM provider support
- Add web search and fetch tools

### 🐛 Bug Fixes

- *(test)* Resolve failing test_get_tool_definitions by adding API key
- *(test)* Update zai provider test expectation

### 📚 Documentation

- Add local LLM providers documentation

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to v0.0.20

## [0.0.21] - 2026-03-11

All notable changes to this project will be documented in this file.

## [0.0.20] - 2026-03-11

### 🚀 Features

- Add clipboard and text selection to TUI (#7)

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to v0.0.19

## [0.0.20] - 2026-03-11

All notable changes to this project will be documented in this file.

## [0.0.19] - 2026-03-10

### 🚀 Features

- Add TUI performance optimizations
- *(llm)* Add thinking_enabled config for Z.AI

### 🐛 Bug Fixes

- *(llm)* [**breaking**] Change FunctionCall.arguments to String for bincode compatibility
- *(release)* Sync versions to 0.0.17 and fix sed bug
- Correct scroll when using sliding window
- *(release)* Update workspace dependencies in release script

### 📚 Documentation

- Add base_url configuration for custom OpenAI-compatible servers

### ⚙️ Miscellaneous Tasks

- Add git-cliff configuration for changelog generation
- *(release)* Bump version to v0.0.18
- Prepare crates for crates.io publishing v0.0.18 (#6)
- Trash

## [0.0.19] - 2026-03-10

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
