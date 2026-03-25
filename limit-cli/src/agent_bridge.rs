use crate::error::CliError;
use crate::system_prompt::get_system_prompt;
use crate::tools::{
    AstGrepTool, BashTool, BrowserTool, FileEditTool, FileReadTool, FileWriteTool, GitAddTool,
    GitCloneTool, GitCommitTool, GitDiffTool, GitLogTool, GitPullTool, GitPushTool, GitStatusTool,
    WebFetchTool, WebSearchTool,
};
use chrono::Datelike;
use futures::StreamExt;
use limit_agent::executor::{ToolCall, ToolExecutor};
use limit_agent::registry::ToolRegistry;
use limit_llm::apply_cache_control;
use limit_llm::providers::LlmProvider;
use limit_llm::types::{Message, Role, Tool as LlmTool, ToolCall as LlmToolCall};
use limit_llm::ModelHandoff;
use limit_llm::ProviderFactory;
use limit_llm::ProviderResponseChunk;
use limit_llm::Summarizer;
use limit_llm::TrackingDb;
use serde_json::json;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, instrument, trace};

/// Event types for streaming from agent to REPL
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AgentEvent {
    Thinking {
        operation_id: u64,
    },
    ToolStart {
        operation_id: u64,
        name: String,
        args: serde_json::Value,
    },
    ToolComplete {
        operation_id: u64,
        name: String,
        result: String,
    },
    ContentChunk {
        operation_id: u64,
        chunk: String,
    },
    Done {
        operation_id: u64,
    },
    Cancelled {
        operation_id: u64,
    },
    Error {
        operation_id: u64,
        message: String,
    },
    TokenUsage {
        operation_id: u64,
        input_tokens: u64,
        output_tokens: u64,
    },
}

/// Result from processing a message
#[derive(Debug, Clone, Default)]
pub struct ProcessResult {
    pub response: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Maximum number of recent tool calls to keep for deduplication
const MAX_RECENT_TOOL_CALLS: usize = 20;

/// Maximum characters in tool result before truncation
const MAX_TOOL_RESULT_CHARS: usize = 10000;

/// Bridge connecting limit-cli REPL to limit-agent executor and limit-llm client
pub struct AgentBridge {
    /// LLM client for communicating with LLM providers
    llm_client: Box<dyn LlmProvider>,
    /// Tool executor for running tool calls
    executor: ToolExecutor,
    /// List of registered tool names
    tool_names: Vec<&'static str>,
    /// Configuration loaded from ~/.limit/config.toml
    config: limit_llm::Config,
    /// Event sender for streaming events to REPL
    event_tx: Option<mpsc::UnboundedSender<AgentEvent>>,
    /// Token usage tracking database
    tracking_db: TrackingDb,
    cancellation_token: Option<CancellationToken>,
    operation_id: u64,
    recent_tool_calls: RefCell<Vec<(String, u64)>>,
    handoff: ModelHandoff,
    summarizer: Option<Summarizer>,
    last_context_percent: RefCell<usize>,
}

impl AgentBridge {
    /// Create a new AgentBridge with the given configuration
    /// Create a new AgentBridge with the given configuration
    /// Create a new AgentBridge with the given configuration
    ///
    /// # Arguments
    /// * `config` - LLM configuration (API key, model, etc.)
    ///
    /// # Returns
    /// A new AgentBridge instance or an error if initialization fails
    pub fn new(config: limit_llm::Config) -> Result<Self, CliError> {
        let tracking_db = TrackingDb::new().map_err(|e| CliError::ConfigError(e.to_string()))?;
        Self::with_tracking_db(config, tracking_db)
    }

    /// Create a new AgentBridge for testing with an in-memory tracking database
    #[cfg(test)]
    pub fn new_for_test(config: limit_llm::Config) -> Result<Self, CliError> {
        let tracking_db =
            TrackingDb::new_in_memory().map_err(|e| CliError::ConfigError(e.to_string()))?;
        Self::with_tracking_db(config, tracking_db)
    }

    /// Create a new AgentBridge with a custom tracking database
    pub fn with_tracking_db(
        config: limit_llm::Config,
        tracking_db: TrackingDb,
    ) -> Result<Self, CliError> {
        let llm_client = ProviderFactory::create_provider(&config)
            .map_err(|e| CliError::ConfigError(e.to_string()))?;

        let mut tool_registry = ToolRegistry::new();
        Self::register_tools(&mut tool_registry, &config);

        let executor = ToolExecutor::new(tool_registry);

        let tool_names = vec![
            "file_read",
            "file_write",
            "file_edit",
            "bash",
            "git_status",
            "git_diff",
            "git_log",
            "git_add",
            "git_commit",
            "git_push",
            "git_pull",
            "git_clone",
            // "grep",           // TEMP: disabled for ast_grep testing
            "ast_grep",
            // "lsp",            // TEMP: disabled for ast_grep testing
            "web_search",
            "web_fetch",
            "browser",
        ];

        Ok(Self {
            llm_client,
            executor,
            tool_names,
            config,
            event_tx: None,
            tracking_db,
            cancellation_token: None,
            operation_id: 0,
            recent_tool_calls: RefCell::new(Vec::new()),
            handoff: ModelHandoff::new(),
            summarizer: None,
            last_context_percent: RefCell::new(0),
        })
    }

    /// Set the event channel sender for streaming events
    pub fn set_event_tx(&mut self, tx: mpsc::UnboundedSender<AgentEvent>) {
        self.event_tx = Some(tx);
    }

    /// Set the cancellation token and operation ID for this operation
    pub fn set_cancellation_token(&mut self, token: CancellationToken, operation_id: u64) {
        debug!("set_cancellation_token: operation_id={}", operation_id);
        self.cancellation_token = Some(token);
        self.operation_id = operation_id;
    }

    /// Clear the cancellation token
    pub fn clear_cancellation_token(&mut self) {
        self.cancellation_token = None;
    }

    async fn maybe_compact(&self, messages: &mut Vec<Message>) {
        if !self.config.compaction.enabled {
            return;
        }

        let context_window: usize = 200_000;
        let target_tokens = (context_window * 6) / 10;
        let warn_tokens = context_window / 2;
        let current_tokens = self.handoff.count_total_tokens(messages);
        let current_pct = (current_tokens * 100) / context_window;

        if current_tokens > warn_tokens && current_tokens <= target_tokens {
            let last_pct = *self.last_context_percent.borrow();
            if current_pct != last_pct {
                tracing::warn!(
                    "Context at {}% ({} tokens). Compaction will trigger at 60%.",
                    current_pct,
                    current_tokens
                );
                *self.last_context_percent.borrow_mut() = current_pct;
            }
        } else if current_tokens <= warn_tokens {
            *self.last_context_percent.borrow_mut() = 0;
        }

        if current_tokens <= target_tokens {
            return;
        }

        let keep_recent = self.config.compaction.keep_recent_tokens as usize;

        if let Some(ref summarizer) = self.summarizer {
            if let Some(cut_idx) = self.handoff.find_cut_point(messages, keep_recent) {
                if cut_idx > 0 {
                    let to_summarize = &messages[..cut_idx];

                    match summarizer.summarize(to_summarize, None).await {
                        Ok(summary) => {
                            let summary_msg = Message {
                                role: Role::User,
                                content: Some(format!(
                                    "<context_summary>\n{}\n</context_summary>",
                                    summary
                                )),
                                tool_calls: None,
                                tool_call_id: None,
                                cache_control: None,
                            };

                            let mut new_messages = vec![summary_msg];
                            new_messages.extend(messages[cut_idx..].to_vec());
                            *messages = new_messages;

                            debug!(
                                "Compacted via summarization: {} messages -> {} messages",
                                cut_idx,
                                messages.len()
                            );
                            return;
                        }
                        Err(e) => {
                            debug!("Summarization failed, falling back to truncation: {}", e);
                        }
                    }
                }
            }
        }

        let compacted = self.handoff.compact_messages(messages, target_tokens);
        *messages = compacted;
    }

    fn hash_tool_call(tool_name: &str, args: &serde_json::Value) -> u64 {
        let mut hasher = DefaultHasher::new();
        tool_name.hash(&mut hasher);
        args.to_string().hash(&mut hasher);
        hasher.finish()
    }

    fn check_duplicate_tool_call(&self, tool_name: &str, args: &serde_json::Value) -> bool {
        let hash = Self::hash_tool_call(tool_name, args);
        self.recent_tool_calls
            .borrow()
            .iter()
            .any(|(name, h)| *name == tool_name && *h == hash)
    }

    fn record_tool_call(&self, tool_name: &str, args: &serde_json::Value) {
        let hash = Self::hash_tool_call(tool_name, args);
        self.recent_tool_calls
            .borrow_mut()
            .push((tool_name.to_string(), hash));
        if self.recent_tool_calls.borrow().len() > MAX_RECENT_TOOL_CALLS {
            self.recent_tool_calls.borrow_mut().remove(0);
        }
    }

    /// Register all CLI tools into the tool registry
    fn register_tools(registry: &mut ToolRegistry, config: &limit_llm::Config) {
        // File tools
        registry
            .register(FileReadTool::new())
            .expect("Failed to register file_read");
        registry
            .register(FileWriteTool::new())
            .expect("Failed to register file_write");
        registry
            .register(FileEditTool::new())
            .expect("Failed to register file_edit");

        // Bash tool
        registry
            .register(BashTool::new())
            .expect("Failed to register bash");

        // Git tools
        registry
            .register(GitStatusTool::new())
            .expect("Failed to register git_status");
        registry
            .register(GitDiffTool::new())
            .expect("Failed to register git_diff");
        registry
            .register(GitLogTool::new())
            .expect("Failed to register git_log");
        registry
            .register(GitAddTool::new())
            .expect("Failed to register git_add");
        registry
            .register(GitCommitTool::new())
            .expect("Failed to register git_commit");
        registry
            .register(GitPushTool::new())
            .expect("Failed to register git_push");
        registry
            .register(GitPullTool::new())
            .expect("Failed to register git_pull");
        registry
            .register(GitCloneTool::new())
            .expect("Failed to register git_clone");

        // Analysis tools
        // TEMP: disabled for ast_grep testing
        // registry
        //     .register(GrepTool::new())
        //     .expect("Failed to register grep");
        registry
            .register(AstGrepTool::new())
            .expect("Failed to register ast_grep");
        // registry
        //     .register(LspTool::new())
        //     .expect("Failed to register lsp");

        // Web tools
        registry
            .register(WebSearchTool::new())
            .expect("Failed to register web_search");
        registry
            .register(WebFetchTool::new())
            .expect("Failed to register web_fetch");

        // Browser tool
        let browser_config = crate::tools::browser::BrowserConfig::from(&config.browser);
        registry
            .register(BrowserTool::with_config(browser_config))
            .expect("Failed to register browser");
    }

    /// Process a user message through the LLM and execute any tool calls
    ///
    /// # Arguments
    /// * `user_input` - The user's message to process
    /// * `messages` - The conversation history (will be updated in place)
    ///
    /// # Returns
    /// The final response from the LLM or an error
    #[instrument(skip(self, _messages, user_input))]
    pub async fn process_message(
        &mut self,
        user_input: &str,
        _messages: &mut Vec<Message>,
    ) -> Result<ProcessResult, CliError> {
        // Add system message if this is the first message in the conversation
        // Note: Some providers (z.ai) don't support system role, but OpenAI-compatible APIs generally do
        if _messages.is_empty() {
            let system_message = Message {
                role: Role::System,
                content: Some(get_system_prompt()),
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            };
            _messages.push(system_message);
        }

        // Add user message to history
        let user_message = Message {
            role: Role::User,
            content: Some(user_input.to_string()),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        };
        _messages.push(user_message);

        // Get tool definitions
        let tool_definitions = self.get_tool_definitions();

        // Main processing loop
        let mut full_response = String::new();
        let mut tool_calls: Vec<LlmToolCall> = Vec::new();
        let max_iterations = self
            .config
            .providers
            .get(&self.config.provider)
            .map(|p| p.max_iterations)
            .unwrap_or(100); // Allow enough iterations for complex tasks
        let mut iteration = 0;
        let mut consecutive_no_exec = 0;
        let mut total_input_tokens: u64 = 0;
        let mut total_output_tokens: u64 = 0;

        while max_iterations == 0 || iteration < max_iterations {
            iteration += 1;
            debug!("Agent loop iteration {}", iteration);

            // Send thinking event
            debug!(
                "Sending Thinking event with operation_id={}",
                self.operation_id
            );
            self.send_event(AgentEvent::Thinking {
                operation_id: self.operation_id,
            });

            let request_start = std::time::Instant::now();

            self.maybe_compact(_messages).await;

            let cached_messages = apply_cache_control(_messages, &self.config.cache);
            let cache_count = cached_messages
                .iter()
                .filter(|m| m.cache_control.is_some())
                .count();
            debug!(
                "Cache control applied to {} of {} messages",
                cache_count,
                cached_messages.len()
            );

            let mut stream = self
                .llm_client
                .send(cached_messages, tool_definitions.clone())
                .await
                .map_err(|e| CliError::ConfigError(e.to_string()))?;

            tool_calls.clear();
            let mut current_content = String::new();
            // Track tool calls: (id) -> (name, args)
            let mut accumulated_calls: std::collections::HashMap<
                String,
                (String, serde_json::Value),
            > = std::collections::HashMap::new();

            // Process stream chunks with cancellation support
            loop {
                // Check for cancellation FIRST (before waiting for stream)
                if let Some(ref token) = self.cancellation_token {
                    if token.is_cancelled() {
                        debug!("Operation cancelled by user (pre-stream check)");
                        self.send_event(AgentEvent::Cancelled {
                            operation_id: self.operation_id,
                        });
                        return Err(CliError::ConfigError(
                            "Operation cancelled by user".to_string(),
                        ));
                    }
                }

                // Use tokio::select! to check cancellation while waiting for stream
                // Using cancellation_token.cancelled() for immediate cancellation detection
                let chunk_result = if let Some(ref token) = self.cancellation_token {
                    tokio::select! {
                        chunk = stream.next() => chunk,
                        _ = token.cancelled() => {
                            debug!("Operation cancelled via token while waiting for stream");
                            self.send_event(AgentEvent::Cancelled {
                                operation_id: self.operation_id,
                            });
                            return Err(CliError::ConfigError("Operation cancelled by user".to_string()));
                        }
                    }
                } else {
                    stream.next().await
                };

                let Some(chunk_result) = chunk_result else {
                    // Stream ended
                    break;
                };

                match chunk_result {
                    Ok(ProviderResponseChunk::ContentDelta(text)) => {
                        current_content.push_str(&text);
                        trace!(
                            "ContentDelta: {} chars (total: {})",
                            text.len(),
                            current_content.len()
                        );
                        self.send_event(AgentEvent::ContentChunk {
                            operation_id: self.operation_id,
                            chunk: text,
                        });
                    }
                    Ok(ProviderResponseChunk::ReasoningDelta(_)) => {
                        // Ignore reasoning chunks for now
                    }
                    Ok(ProviderResponseChunk::ToolCallDelta {
                        id,
                        name,
                        arguments,
                    }) => {
                        trace!(
                            "ToolCallDelta: id={}, name={}, args_len={}",
                            id,
                            name,
                            arguments.to_string().len()
                        );
                        // Store/merge tool call arguments
                        accumulated_calls.insert(id.clone(), (name.clone(), arguments.clone()));
                    }
                    Ok(ProviderResponseChunk::Done(usage)) => {
                        let duration_ms = request_start.elapsed().as_millis() as u64;
                        let cost =
                            calculate_cost(self.model(), usage.input_tokens, usage.output_tokens);

                        if usage.cache_read_tokens > 0 || usage.cache_write_tokens > 0 {
                            debug!(
                                "Cache tokens: read={}, write={}, input={}, output={}",
                                usage.cache_read_tokens,
                                usage.cache_write_tokens,
                                usage.input_tokens,
                                usage.output_tokens
                            );
                        } else {
                            debug!(
                                "No cache tokens in response: input={}, output={}",
                                usage.input_tokens, usage.output_tokens
                            );
                        }

                        let _ = self.tracking_db.track_request(
                            self.model(),
                            usage.input_tokens,
                            usage.output_tokens,
                            usage.cache_read_tokens,
                            usage.cache_write_tokens,
                            cost,
                            duration_ms,
                        );
                        total_input_tokens += usage.input_tokens;
                        total_output_tokens += usage.output_tokens;
                        // Emit token usage event for TUI display
                        self.send_event(AgentEvent::TokenUsage {
                            operation_id: self.operation_id,
                            input_tokens: usage.input_tokens,
                            output_tokens: usage.output_tokens,
                        });
                        break;
                    }
                    Err(e) => {
                        let error_msg = format!("LLM error: {}", e);
                        self.send_event(AgentEvent::Error {
                            operation_id: self.operation_id,
                            message: error_msg.clone(),
                        });
                        return Err(CliError::ConfigError(error_msg));
                    }
                }
            }

            // Convert accumulated calls to Vec<ToolCall> and filter invalid ones
            let raw_tool_calls: Vec<LlmToolCall> = accumulated_calls
                .into_iter()
                .map(|(id, (name, args))| LlmToolCall {
                    id,
                    tool_type: "function".to_string(),
                    function: limit_llm::types::FunctionCall {
                        name,
                        arguments: args.to_string(),
                    },
                })
                .collect();

            // Filter out invalid tool calls (empty names or unregistered tools)
            let raw_count = raw_tool_calls.len();
            tool_calls = raw_tool_calls
                .into_iter()
                .filter(|tc| {
                    let is_valid = !tc.function.name.is_empty()
                        && self.tool_names.contains(&tc.function.name.as_str());
                    if !is_valid {
                        debug!(
                            "Filtered invalid tool call: id={}, name='{}'",
                            tc.id, tc.function.name
                        );
                    }
                    is_valid
                })
                .collect();

            if tool_calls.len() != raw_count {
                debug!(
                    "Filtered {}/{} tool calls (empty names or unregistered tools)",
                    raw_count - tool_calls.len(),
                    raw_count
                );
            }

            // BUG FIX: Don't accumulate content across iterations
            // Only store content from the current iteration
            // If there are tool calls, we'll continue the loop and the LLM will see the tool results
            // If there are NO tool calls, this is the final response
            full_response = current_content.clone();

            trace!(
                "After iter {}: content.len()={}, tool_calls={}, response.len()={}",
                iteration,
                current_content.len(),
                tool_calls.len(),
                full_response.len()
            );

            // If no tool calls, we're done
            if tool_calls.is_empty() {
                debug!("No tool calls, breaking loop after iteration {}", iteration);
                break;
            }

            trace!(
                "Tool calls found (count={}), continuing to iteration {}",
                tool_calls.len(),
                iteration + 1
            );

            // Execute tool calls - add assistant message with tool_calls
            // Note: Per OpenAI API spec, when tool_calls are present, content should be null
            let assistant_message = Message {
                role: Role::Assistant,
                content: None, // Don't include content when tool_calls are present
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
                cache_control: None,
            };
            _messages.push(assistant_message);

            // Check for duplicate tool calls and filter them out
            let mut filtered_calls = Vec::new();
            let mut duplicate_calls = Vec::new();
            let mut calls_to_record = Vec::new();
            for tc in &tool_calls {
                let args: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                if self.check_duplicate_tool_call(&tc.function.name, &args) {
                    duplicate_calls.push((tc.id.clone(), tc.function.name.clone(), args));
                } else {
                    calls_to_record.push((tc.function.name.clone(), args.clone()));
                    filtered_calls.push(tc.clone());
                }
            }

            // Record new tool calls
            for (name, args) in calls_to_record {
                self.record_tool_call(&name, &args);
            }

            // Report duplicate calls to the model
            if !duplicate_calls.is_empty() {
                for (id, name, args) in &duplicate_calls {
                    debug!(
                        "Duplicate tool call blocked: {} with args: {}",
                        name,
                        serde_json::to_string(&args).unwrap_or_default()
                    );
                    self.send_event(AgentEvent::ToolStart {
                        operation_id: self.operation_id,
                        name: name.clone(),
                        args: args.clone(),
                    });
                    let duplicate_msg = json!({
                        "error": "DUPLICATE_CALL_BLOCKED",
                        "message": format!(
                            "You already called {} with these exact arguments in a recent turn. \
                            Check your conversation history for the previous result. \
                            Do not repeat the same query - use the existing data instead.",
                            name
                        ),
                        "tool": name,
                        "args": args
                    });
                    let result_str = serde_json::to_string(&duplicate_msg).unwrap_or_default();
                    self.send_event(AgentEvent::ToolComplete {
                        operation_id: self.operation_id,
                        name: name.clone(),
                        result: result_str.clone(),
                    });
                    let tool_result_message = Message {
                        role: Role::Tool,
                        content: Some(result_str),
                        tool_calls: None,
                        tool_call_id: Some(id.clone()),
                        cache_control: None,
                    };
                    _messages.push(tool_result_message);
                }
            }

            // Send ToolStart event for each non-duplicate tool BEFORE execution
            for tc in &filtered_calls {
                let args: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                debug!(
                    "ToolStart: {} with args: {}",
                    tc.function.name,
                    serde_json::to_string(&args).unwrap_or_default()
                );
                self.send_event(AgentEvent::ToolStart {
                    operation_id: self.operation_id,
                    name: tc.function.name.clone(),
                    args,
                });
            }
            // Execute tools (only non-duplicates)
            let filtered_executor_calls: Vec<ToolCall> = filtered_calls
                .iter()
                .map(|tc| {
                    let args: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                    ToolCall::new(&tc.id, &tc.function.name, args)
                })
                .collect();
            let results = self.executor.execute_tools(filtered_executor_calls).await;
            let results_count = results.len();

            // Add tool results to messages (OpenAI format: role=tool, tool_call_id, content)
            for result in results {
                let tool_call = filtered_calls.iter().find(|tc| tc.id == result.call_id);
                if let Some(tool_call) = tool_call {
                    let output_json = match &result.output {
                        Ok(value) => {
                            serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
                        }
                        Err(e) => json!({ "error": e.to_string() }).to_string(),
                    };

                    debug!(
                        "ToolComplete: {} result ({} chars): {}",
                        tool_call.function.name,
                        output_json.len(),
                        output_json
                    );

                    self.send_event(AgentEvent::ToolComplete {
                        operation_id: self.operation_id,
                        name: tool_call.function.name.clone(),
                        result: output_json.clone(),
                    });

                    // OpenAI tool result format
                    let tool_result_message = Message {
                        role: Role::Tool,
                        content: Some(truncate_tool_result(&output_json)),
                        tool_calls: None,
                        tool_call_id: Some(result.call_id),
                        cache_control: None,
                    };
                    _messages.push(tool_result_message);
                }
            }

            // Safety valve: break if too many consecutive iterations without tool execution
            if results_count == 0 && !tool_calls.is_empty() {
                consecutive_no_exec += 1;
                if consecutive_no_exec >= 3 {
                    debug!(
                        "Safety valve: {} consecutive iterations with tool calls but no executions",
                        consecutive_no_exec
                    );
                    break;
                }
            } else {
                consecutive_no_exec = 0;
            }
        }

        // If we hit max iterations, make one final request to get a response (no tools = forced text)
        // IMPORTANT: Only do this if max_iterations > 0 (0 means unlimited, so we never "hit" the limit)
        if max_iterations > 0 && iteration >= max_iterations && !_messages.is_empty() {
            debug!("Making final LLM call after hitting max iterations (forcing text response)");

            // Add constraint message to force text response
            let constraint_message = Message {
                role: Role::User,
                content: Some(
                    "We've reached the iteration limit. Please provide a summary of:\n\
                    1. What you've completed so far\n\
                    2. What remains to be done\n\
                    3. Recommended next steps for the user to continue"
                        .to_string(),
                ),
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            };
            _messages.push(constraint_message);

            // Send with NO tools to force text response
            let no_tools: Vec<LlmTool> = vec![];
            let mut stream = self
                .llm_client
                .send(_messages.clone(), no_tools)
                .await
                .map_err(|e| CliError::ConfigError(e.to_string()))?;

            // BUG FIX: Replace full_response instead of appending
            full_response.clear();
            loop {
                // Check for cancellation FIRST (before waiting for stream)
                if let Some(ref token) = self.cancellation_token {
                    if token.is_cancelled() {
                        debug!("Operation cancelled by user in final loop (pre-stream check)");
                        self.send_event(AgentEvent::Cancelled {
                            operation_id: self.operation_id,
                        });
                        return Err(CliError::ConfigError(
                            "Operation cancelled by user".to_string(),
                        ));
                    }
                }

                // Use tokio::select! to check cancellation while waiting for stream
                // Using cancellation_token.cancelled() for immediate cancellation detection
                let chunk_result = if let Some(ref token) = self.cancellation_token {
                    tokio::select! {
                        chunk = stream.next() => chunk,
                        _ = token.cancelled() => {
                            debug!("Operation cancelled via token while waiting for stream");
                            self.send_event(AgentEvent::Cancelled {
                                operation_id: self.operation_id,
                            });
                            return Err(CliError::ConfigError("Operation cancelled by user".to_string()));
                        }
                    }
                } else {
                    stream.next().await
                };

                let Some(chunk_result) = chunk_result else {
                    // Stream ended
                    break;
                };

                match chunk_result {
                    Ok(ProviderResponseChunk::ContentDelta(text)) => {
                        full_response.push_str(&text);
                        self.send_event(AgentEvent::ContentChunk {
                            operation_id: self.operation_id,
                            chunk: text,
                        });
                    }
                    Ok(ProviderResponseChunk::Done(_)) => {
                        break;
                    }
                    Err(e) => {
                        debug!("Error in final LLM call: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }

        // IMPORTANT: Add final assistant response to message history for session persistence
        // This is crucial for session export/share to work correctly
        // Only add if we have content AND we haven't already added this response
        if !full_response.is_empty() {
            // Find the last assistant message and check if it has content
            // If it has tool_calls but no content, UPDATE it instead of adding a new one
            // This prevents accumulation of empty assistant messages in the history
            let last_assistant_idx = _messages.iter().rposition(|m| m.role == Role::Assistant);

            if let Some(idx) = last_assistant_idx {
                let last_assistant = &mut _messages[idx];

                // If the last assistant message has no content (tool_calls only), update it
                if last_assistant.content.is_none()
                    || last_assistant
                        .content
                        .as_ref()
                        .map(|c| c.is_empty())
                        .unwrap_or(true)
                {
                    last_assistant.content = Some(full_response.clone());
                    debug!("Updated last assistant message with final response content");
                } else {
                    // Last assistant already has content, this shouldn't happen normally
                    // but we add a new message to be safe
                    debug!("Last assistant already has content, adding new message");
                    let final_assistant_message = Message {
                        role: Role::Assistant,
                        content: Some(full_response.clone()),
                        tool_calls: None,
                        tool_call_id: None,
                        cache_control: None,
                    };
                    _messages.push(final_assistant_message);
                }
            } else {
                // No assistant message found, add a new one
                debug!("No assistant message found, adding new message");
                let final_assistant_message = Message {
                    role: Role::Assistant,
                    content: Some(full_response.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                    cache_control: None,
                };
                _messages.push(final_assistant_message);
            }
        }

        self.send_event(AgentEvent::Done {
            operation_id: self.operation_id,
        });
        Ok(ProcessResult {
            response: full_response,
            input_tokens: total_input_tokens,
            output_tokens: total_output_tokens,
        })
    }

    /// Get tool definitions formatted for the LLM
    pub fn get_tool_definitions(&self) -> Vec<LlmTool> {
        self.tool_names
            .iter()
            .map(|name| {
                let (description, parameters) = Self::get_tool_schema(name);
                LlmTool {
                    tool_type: "function".to_string(),
                    function: limit_llm::types::ToolFunction {
                        name: name.to_string(),
                        description,
                        parameters,
                    },
                }
            })
            .collect()
    }

    /// Get the schema (description and parameters) for a tool
    fn get_tool_schema(name: &str) -> (String, serde_json::Value) {
        match name {
            "file_read" => (
                "Read the contents of a file".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        }
                    },
                    "required": ["path"]
                }),
            ),
            "file_write" => (
                "Write content to a file, creating parent directories if needed".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
                }),
            ),
            "file_edit" => (
                "Replace text in a file with new text".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to edit"
                        },
                        "old_text": {
                            "type": "string",
                            "description": "Text to find and replace"
                        },
                        "new_text": {
                            "type": "string",
                            "description": "New text to replace with"
                        }
                    },
                    "required": ["path", "old_text", "new_text"]
                }),
            ),
            "bash" => (
                "Execute a bash command in a shell".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Bash command to execute"
                        },
                        "workdir": {
                            "type": "string",
                            "description": "Working directory (default: current directory)"
                        },
                        "timeout": {
                            "type": "integer",
                            "description": "Timeout in seconds (default: 60)"
                        }
                    },
                    "required": ["command"]
                }),
            ),
            "git_status" => (
                "Get git repository status".to_string(),
                json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
            "git_diff" => (
                "Get git diff".to_string(),
                json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
            "git_log" => (
                "Get git commit log".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "count": {
                            "type": "integer",
                            "description": "Number of commits to show (default: 10)"
                        }
                    },
                    "required": []
                }),
            ),
            "git_add" => (
                "Add files to git staging area".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "files": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of file paths to add"
                        }
                    },
                    "required": ["files"]
                }),
            ),
            "git_commit" => (
                "Create a git commit".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Commit message"
                        }
                    },
                    "required": ["message"]
                }),
            ),
            "git_push" => (
                "Push commits to remote repository".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "remote": {
                            "type": "string",
                            "description": "Remote name (default: origin)"
                        },
                        "branch": {
                            "type": "string",
                            "description": "Branch name (default: current branch)"
                        }
                    },
                    "required": []
                }),
            ),
            "git_pull" => (
                "Pull changes from remote repository".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "remote": {
                            "type": "string",
                            "description": "Remote name (default: origin)"
                        },
                        "branch": {
                            "type": "string",
                            "description": "Branch name (default: current branch)"
                        }
                    },
                    "required": []
                }),
            ),
            "git_clone" => (
                "Clone a git repository".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "Repository URL to clone"
                        },
                        "directory": {
                            "type": "string",
                            "description": "Directory to clone into (optional)"
                        }
                    },
                    "required": ["url"]
                }),
            ),
            "grep" => (
                "Search for text patterns in files using regex".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Regex pattern to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "Path to search in (default: current directory)"
                        }
                    },
                    "required": ["pattern"]
                }),
            ),
            "ast_grep" => (
                "AST-aware code search and transformation. Supports search, replace, and scan commands across 25+ languages. Use meta-variables: $VAR (single node), $$$VAR (multiple nodes). Search finds patterns, replace transforms code, scan runs lint rules.".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "enum": ["search", "replace", "scan"],
                            "description": "Command to execute. Default: search"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "AST pattern to match (e.g., 'fn $NAME() {}'). Required for search and replace."
                        },
                        "language": {
                            "type": "string",
                            "description": "Programming language. Supported: bash, c, cpp, csharp, css, elixir, go, haskell, html, java, javascript, json, kotlin, lua, nix, php, python, ruby, rust, scala, solidity, swift, typescript, tsx, yaml. Required for search and replace."
                        },
                        "path": {
                            "type": "string",
                            "description": "Path to search in (default: current directory)"
                        },
                        "rewrite": {
                            "type": "string",
                            "description": "Replacement pattern for replace command (e.g., 'logger.info($MSG)'). Required for replace."
                        },
                        "dry_run": {
                            "type": "boolean",
                            "description": "Preview replacements without modifying files (default: false). Only for replace command."
                        },
                        "globs": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Include/exclude file patterns (e.g., ['*.rs', '!*.test.rs']). Prefix with ! to exclude."
                        },
                        "context_after": {
                            "type": "integer",
                            "description": "Show N lines after each match (default: 0). Only for search."
                        },
                        "context_before": {
                            "type": "integer",
                            "description": "Show N lines before each match (default: 0). Only for search."
                        },
                        "rule": {
                            "type": "string",
                            "description": "Path to YAML rule file for scan command."
                        },
                        "inline_rules": {
                            "type": "string",
                            "description": "Inline YAML rule text for scan command."
                        },
                        "filter": {
                            "type": "string",
                            "description": "Regex to filter rules by ID for scan command."
                        }
                    },
                    "required": ["pattern", "language"]
                }),
            ),
            "lsp" => (
                "Perform Language Server Protocol operations (goto_definition, find_references)"
                    .to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "LSP command: goto_definition or find_references"
                        },
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file"
                        },
                        "position": {
                            "type": "object",
                            "description": "Position in the file (line, character)",
                            "properties": {
                                "line": {"type": "integer"},
                                "character": {"type": "integer"}
                            },
                            "required": ["line", "character"]
                        }
                    },
                    "required": ["command", "file_path", "position"]
                }),
            ),
            "web_search" => (
                format!("Search the web using Exa AI. Returns results with titles, URLs, and content snippets. Use for current information beyond knowledge cutoff. The current year is {} - use this year when searching for recent information.", chrono::Local::now().year()),
                json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": format!("Search query. Be specific for better results (e.g., 'Rust async tutorial {}' rather than 'Rust')", chrono::Local::now().year())
                        },
                        "numResults": {
                            "type": "integer",
                            "description": "Number of results to return (default: 8, max: 20)",
                            "default": 8
                        }
                    },
                    "required": ["query"]
                }),
            ),
            "web_fetch" => (
                "Fetch content from a URL. Converts HTML to markdown format by default. Use when user provides a URL or after web_search to read full content of a specific result.".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to fetch (must start with http:// or https://)"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["markdown", "text", "html"],
                            "default": "markdown",
                            "description": "Output format (default: markdown)"
                        }
                    },
                    "required": ["url"]
                }),
            ),
            "browser" => (
                "Browser automation for testing, scraping, and web interaction. Use snapshot-ref workflow: open URL, take snapshot, use refs from snapshot for interactions. Supports Chrome and Lightpanda engines.".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": [
                                // Core
                                "open", "close", "snapshot",
                                // Interaction
                                "click", "dblclick", "fill", "type", "press", "hover", "select",
                                "focus", "check", "uncheck", "scrollintoview", "drag", "upload",
                                // Navigation
                                "back", "forward", "reload",
                                // Query
                                "screenshot", "pdf", "eval", "get", "get_attr", "get_count", "get_box", "get_styles",
                                "find", "is", "download",
                                // Waiting
                                "wait", "wait_for_text", "wait_for_url", "wait_for_load", "wait_for_download", "wait_for_fn", "wait_for_state",
                                // Tabs & Dialogs
                                "tab_list", "tab_new", "tab_close", "tab_select", "dialog_accept", "dialog_dismiss",
                                // Storage & Network
                                "cookies", "cookies_set", "storage_get", "storage_set", "network_requests",
                                // Settings
                                "set_viewport", "set_device", "set_geo",
                                // State
                                "scroll"
                            ],
                            "description": "Browser action to perform"
                        },
                        // Core
                        "url": {
                            "type": "string",
                            "description": "URL to open (required for 'open' action)"
                        },
                        // Interaction
                        "selector": {
                            "type": "string",
                            "description": "Element selector or ref (for click, fill, type, hover, select, focus, check, uncheck, scrollintoview, get_attr, get_count, get_box, get_styles, is, download, upload)"
                        },
                        "text": {
                            "type": "string",
                            "description": "Text to input (for fill, type actions)"
                        },
                        "key": {
                            "type": "string",
                            "description": "Key to press (required for 'press' action)"
                        },
                        "value": {
                            "type": "string",
                            "description": "Value (for select, cookies_set, storage_set)"
                        },
                        "target": {
                            "type": "string",
                            "description": "Target selector (for drag action)"
                        },
                        "files": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "File paths to upload (for upload action)"
                        },
                        // Query
                        "path": {
                            "type": "string",
                            "description": "File path (for screenshot, pdf, download actions)"
                        },
                        "script": {
                            "type": "string",
                            "description": "JavaScript to evaluate (required for 'eval' and 'wait_for_fn' actions)"
                        },
                        "get_what": {
                            "type": "string",
                            "enum": ["text", "html", "value", "url", "title"],
                            "description": "What to get (required for 'get' action)"
                        },
                        "attr": {
                            "type": "string",
                            "description": "Attribute name (for get_attr action)"
                        },
                        // Find
                        "locator_type": {
                            "type": "string",
                            "enum": ["role", "text", "label", "placeholder", "alt", "title", "testid", "css", "xpath"],
                            "description": "Locator strategy (for find action)"
                        },
                        "locator_value": {
                            "type": "string",
                            "description": "Locator value (for find action)"
                        },
                        "find_action": {
                            "type": "string",
                            "enum": ["click", "fill", "text", "count", "first", "last", "nth", "hover", "focus", "check", "uncheck"],
                            "description": "Action to perform on found element (for find action)"
                        },
                        "action_value": {
                            "type": "string",
                            "description": "Value for find action (optional)"
                        },
                        // Waiting
                        "wait_for": {
                            "type": "string",
                            "description": "Wait condition (for wait action)"
                        },
                        "state": {
                            "type": "string",
                            "enum": ["visible", "hidden", "attached", "detached", "enabled", "disabled", "networkidle", "domcontentloaded", "load"],
                            "description": "State to wait for (for wait_for_state, wait_for_load actions)"
                        },
                        // State check
                        "what": {
                            "type": "string",
                            "enum": ["visible", "hidden", "enabled", "disabled", "editable"],
                            "description": "State to check (required for 'is' action)"
                        },
                        // Scroll
                        "direction": {
                            "type": "string",
                            "enum": ["up", "down", "left", "right"],
                            "description": "Scroll direction (for scroll action)"
                        },
                        "pixels": {
                            "type": "integer",
                            "description": "Pixels to scroll (optional for scroll action)"
                        },
                        // Tabs
                        "index": {
                            "type": "integer",
                            "description": "Tab index (for tab_close, tab_select actions)"
                        },
                        // Dialogs
                        "dialog_text": {
                            "type": "string",
                            "description": "Text for prompt dialog (for dialog_accept action)"
                        },
                        // Storage
                        "storage_type": {
                            "type": "string",
                            "enum": ["local", "session"],
                            "description": "Storage type (for storage_get, storage_set actions)"
                        },
                        "key_name": {
                            "type": "string",
                            "description": "Storage key name (for storage_get, storage_set actions)"
                        },
                        // Network
                        "filter": {
                            "type": "string",
                            "description": "Network request filter (optional for network_requests action)"
                        },
                        // Settings
                        "width": {
                            "type": "integer",
                            "description": "Viewport width (for set_viewport action)"
                        },
                        "height": {
                            "type": "integer",
                            "description": "Viewport height (for set_viewport action)"
                        },
                        "scale": {
                            "type": "number",
                            "description": "Device scale factor (optional for set_viewport action)"
                        },
                        "device_name": {
                            "type": "string",
                            "description": "Device name to emulate (for set_device action)"
                        },
                        "latitude": {
                            "type": "number",
                            "description": "Latitude (for set_geo action)"
                        },
                        "longitude": {
                            "type": "number",
                            "description": "Longitude (for set_geo action)"
                        },
                        // Cookie
                        "name": {
                            "type": "string",
                            "description": "Cookie name (for cookies_set action)"
                        },
                        // Engine
                        "engine": {
                            "type": "string",
                            "enum": ["chrome", "lightpanda"],
                            "default": "chrome",
                            "description": "Browser engine to use"
                        }
                    },
                    "required": ["action"]
                }),
            ),
            _ => (
                format!("Tool: {}", name),
                json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
        }
    }

    /// Send an event through the event channel
    fn send_event(&self, event: AgentEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event);
        }
    }

    /// Check if the bridge is ready to process messages
    #[allow(dead_code)]
    pub fn is_ready(&self) -> bool {
        self.config
            .providers
            .get(&self.config.provider)
            .map(|p| p.api_key_or_env(&self.config.provider).is_some())
            .unwrap_or(false)
    }

    /// Get the current model name
    pub fn model(&self) -> &str {
        self.config
            .providers
            .get(&self.config.provider)
            .map(|p| p.model.as_str())
            .unwrap_or("")
    }

    /// Get the max tokens setting
    pub fn max_tokens(&self) -> u32 {
        self.config
            .providers
            .get(&self.config.provider)
            .map(|p| p.max_tokens)
            .unwrap_or(4096)
    }

    /// Get the timeout setting
    pub fn timeout(&self) -> u64 {
        self.config
            .providers
            .get(&self.config.provider)
            .map(|p| p.timeout)
            .unwrap_or(60)
    }
}
/// Calculate cost based on model pricing (per 1M tokens)
fn calculate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
    let (input_price, output_price) = match model {
        // Claude 3.5 Sonnet: $3/1M input, $15/1M output
        "claude-3-5-sonnet-20241022" | "claude-3-5-sonnet" => (3.0, 15.0),
        // GPT-4: $30/1M input, $60/1M output
        "gpt-4" => (30.0, 60.0),
        // GPT-4 Turbo: $10/1M input, $30/1M output
        "gpt-4-turbo" | "gpt-4-turbo-preview" => (10.0, 30.0),
        // Default: no cost tracking
        _ => (0.0, 0.0),
    };
    (input_tokens as f64 * input_price / 1_000_000.0)
        + (output_tokens as f64 * output_price / 1_000_000.0)
}

fn truncate_tool_result(result: &str) -> String {
    if result.len() > MAX_TOOL_RESULT_CHARS {
        let truncated = &result[..MAX_TOOL_RESULT_CHARS];
        format!(
            "{}\n\n... [TRUNCATED: {} chars total, showing first {}]",
            truncated,
            result.len(),
            MAX_TOOL_RESULT_CHARS
        )
    } else {
        result.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use limit_llm::{BrowserConfigSection, Config as LlmConfig, ProviderConfig};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_agent_bridge_new() {
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                api_key: Some("test-key".to_string()),
                model: "claude-3-5-sonnet-20241022".to_string(),
                base_url: None,
                max_tokens: 4096,
                timeout: 60,
                max_iterations: 100,
                thinking_enabled: false,
                clear_thinking: true,
            },
        );
        let config = LlmConfig {
            provider: "anthropic".to_string(),
            providers,
            browser: BrowserConfigSection::default(),
            compaction: limit_llm::CompactionSettings::default(),
            cache: limit_llm::CacheSettings::default(),
        };

        let bridge = AgentBridge::new(config).unwrap();
        assert!(bridge.is_ready());
    }

    #[tokio::test]
    async fn test_agent_bridge_new_no_api_key() {
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                api_key: None,
                model: "claude-3-5-sonnet-20241022".to_string(),
                base_url: None,
                max_tokens: 4096,
                timeout: 60,
                max_iterations: 100,
                thinking_enabled: false,
                clear_thinking: true,
            },
        );
        let config = LlmConfig {
            provider: "anthropic".to_string(),
            providers,
            browser: BrowserConfigSection::default(),
            compaction: limit_llm::CompactionSettings::default(),
            cache: limit_llm::CacheSettings::default(),
        };

        let result = AgentBridge::new(config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_tool_definitions() {
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                api_key: Some("test-key".to_string()),
                model: "claude-3-5-sonnet-20241022".to_string(),
                base_url: None,
                max_tokens: 4096,
                timeout: 60,
                max_iterations: 100,
                thinking_enabled: false,
                clear_thinking: true,
            },
        );
        let config = LlmConfig {
            provider: "anthropic".to_string(),
            providers,
            browser: BrowserConfigSection::default(),
            compaction: limit_llm::CompactionSettings::default(),
            cache: limit_llm::CacheSettings::default(),
        };

        let bridge = AgentBridge::new(config).unwrap();
        let definitions = bridge.get_tool_definitions();

        assert_eq!(definitions.len(), 16); // grep, lsp disabled

        // Check file_read tool definition
        let file_read = definitions
            .iter()
            .find(|d| d.function.name == "file_read")
            .unwrap();
        assert_eq!(file_read.tool_type, "function");
        assert_eq!(file_read.function.name, "file_read");
        assert!(file_read.function.description.contains("Read"));

        // Check bash tool definition
        let bash = definitions
            .iter()
            .find(|d| d.function.name == "bash")
            .unwrap();
        assert_eq!(bash.function.name, "bash");
        assert!(bash.function.parameters["required"]
            .as_array()
            .unwrap()
            .contains(&"command".into()));
    }

    #[test]
    fn test_get_tool_schema() {
        let (desc, params) = AgentBridge::get_tool_schema("file_read");
        assert!(desc.contains("Read"));
        assert_eq!(params["properties"]["path"]["type"], "string");
        assert!(params["required"]
            .as_array()
            .unwrap()
            .contains(&"path".into()));

        let (desc, params) = AgentBridge::get_tool_schema("bash");
        assert!(desc.contains("bash"));
        assert_eq!(params["properties"]["command"]["type"], "string");

        let (desc, _params) = AgentBridge::get_tool_schema("unknown_tool");
        assert!(desc.contains("unknown_tool"));
    }

    #[test]
    fn test_is_ready() {
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                api_key: Some("test-key".to_string()),
                model: "claude-3-5-sonnet-20241022".to_string(),
                base_url: None,
                max_tokens: 4096,
                timeout: 60,
                max_iterations: 100,
                thinking_enabled: false,
                clear_thinking: true,
            },
        );
        let config_with_key = LlmConfig {
            provider: "anthropic".to_string(),
            providers,
            browser: BrowserConfigSection::default(),
            compaction: limit_llm::CompactionSettings::default(),
            cache: limit_llm::CacheSettings::default(),
        };

        let bridge = AgentBridge::new(config_with_key).unwrap();
        assert!(bridge.is_ready());
    }

    #[test]
    fn test_handoff_compaction_preserves_system() {
        let handoff = ModelHandoff::new();

        let mut messages = vec![Message {
            role: Role::System,
            content: Some("System prompt".to_string()),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        for i in 0..50 {
            messages.push(Message {
                role: if i % 2 == 0 {
                    Role::User
                } else {
                    Role::Assistant
                },
                content: Some(format!(
                    "Message {} with enough content to consume tokens",
                    i
                )),
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            });
        }

        let target = 500;
        let compacted = handoff.compact_messages(&messages, target);

        assert_eq!(compacted[0].role, Role::System);
        assert!(compacted.len() < messages.len());
    }

    #[test]
    fn test_handoff_compaction_keeps_recent() {
        let handoff = ModelHandoff::new();

        let mut messages = vec![Message {
            role: Role::System,
            content: Some("System".to_string()),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        for i in 0..100 {
            messages.push(Message {
                role: if i % 2 == 0 {
                    Role::User
                } else {
                    Role::Assistant
                },
                content: Some(format!("Message {}", i)),
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            });
        }

        let target = 200;
        let compacted = handoff.compact_messages(&messages, target);

        assert!(compacted.len() < messages.len());
        let last_content = compacted.last().unwrap().content.clone();
        assert_eq!(last_content, Some("Message 99".to_string()));
    }

    #[test]
    fn test_compaction_config_respects_settings() {
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                api_key: Some("test-key".to_string()),
                model: "claude-3-5-sonnet-20241022".to_string(),
                base_url: None,
                max_tokens: 4096,
                timeout: 60,
                max_iterations: 100,
                thinking_enabled: false,
                clear_thinking: true,
            },
        );

        let config = LlmConfig {
            provider: "anthropic".to_string(),
            providers,
            browser: BrowserConfigSection::default(),
            compaction: limit_llm::CompactionSettings {
                enabled: true,
                reserve_tokens: 8192,
                keep_recent_tokens: 10000,
                use_summarization: true,
            },
            cache: limit_llm::CacheSettings::default(),
        };

        let bridge = AgentBridge::new(config).unwrap();
        assert!(bridge.config.compaction.enabled);
        assert_eq!(bridge.config.compaction.reserve_tokens, 8192);
        assert_eq!(bridge.config.compaction.keep_recent_tokens, 10000);
    }
}
