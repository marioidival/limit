//! Specialized agent with a specific role.
//!
//! Wraps an LLM provider with role-specific system prompts and
//! an optional tool registry.

use crate::error::AgentError;
use crate::registry::ToolRegistry;
use crate::team::role::Role;
use futures::StreamExt;
use limit_llm::{LlmProvider, Message, ProviderResponseChunk, Role as LlmRole, Usage};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Maximum number of automatic retries for transient LLM failures.
const MAX_RETRIES: usize = 3;
/// Base delay between retries (doubles on each attempt: 1s, 2s, 4s).
const RETRY_BASE_DELAY: Duration = Duration::from_secs(1);
/// Maximum tool-call rounds per `prompt()` call before forcing a text response.
/// Prevents infinite loops where the LLM keeps requesting more tool calls.
const MAX_TOOL_ROUNDS: usize = 10;

/// Result from [`TeamAgent::prompt`] including token usage and tool-limit status.
#[derive(Debug, Clone)]
pub struct PromptResult {
    /// The text content of the LLM response.
    pub text: String,
    /// Token usage accumulated across all rounds.
    pub usage: Usage,
    /// Whether the tool-call limit ([`MAX_TOOL_ROUNDS`]) was hit.
    pub hit_tool_limit: bool,
    /// File paths modified by file_write or file_edit tool calls.
    pub files_modified: Vec<String>,
}

fn zero_usage() -> Usage {
    Usage {
        input_tokens: 0,
        output_tokens: 0,
    }
}

/// Accumulates tool-call deltas from streaming responses.
///
/// Tool calls arrive as deltas with id, name, and arguments fragments.
/// This accumulator tracks the current tool being accumulated and
/// returns completed tool calls when a new tool starts or on flush.
#[derive(Default)]
struct ToolCallAccumulator {
    current_tool_id: String,
    current_tool_name: String,
    current_tool_args: String,
}

impl ToolCallAccumulator {
    /// Process a tool-call delta and return a completed ToolCall if a new tool started.
    ///
    /// Returns `Some(ToolCall)` when a new tool id is received (meaning the previous
    /// tool is complete), or `None` if still accumulating the current tool.
    fn process_delta(
        &mut self,
        id: String,
        name: String,
        arguments: Value,
    ) -> Option<limit_llm::ToolCall> {
        // Flush previous tool call if starting a new one
        let completed = if id != self.current_tool_id && !self.current_tool_id.is_empty() {
            Some(limit_llm::ToolCall {
                id: std::mem::take(&mut self.current_tool_id),
                tool_type: "function".to_string(),
                function: limit_llm::FunctionCall {
                    name: std::mem::take(&mut self.current_tool_name),
                    arguments: std::mem::take(&mut self.current_tool_args),
                },
            })
        } else {
            None
        };

        self.current_tool_id = id;
        self.current_tool_name = name;

        // Accumulate arguments from all deltas (handles both Object and String fragments)
        match &arguments {
            Value::Object(_) | Value::Array(_) | Value::Bool(_) | Value::Number(_) => {
                // Structured value - serialize it
                self.current_tool_args = serde_json::to_string(&arguments).unwrap_or_default();
            }
            Value::String(s) => {
                // String fragment - append it
                self.current_tool_args.push_str(s);
            }
            Value::Null => {
                // Null - ignore
            }
        }

        completed
    }

    /// Flush the current tool call if any remains.
    ///
    /// Call this after the stream ends to get the final tool call.
    fn flush(&mut self) -> Option<limit_llm::ToolCall> {
        if self.current_tool_id.is_empty() {
            None
        } else {
            Some(limit_llm::ToolCall {
                id: std::mem::take(&mut self.current_tool_id),
                tool_type: "function".to_string(),
                function: limit_llm::FunctionCall {
                    name: std::mem::take(&mut self.current_tool_name),
                    arguments: std::mem::take(&mut self.current_tool_args),
                },
            })
        }
    }
}

/// A specialized agent that acts as a single member of a team.
pub struct TeamAgent {
    role: Role,
    provider: Box<dyn LlmProvider>,
    registry: Arc<ToolRegistry>,
    history: Arc<Vec<Message>>,
}

impl TeamAgent {
    /// Create a placeholder agent for mem::replace purposes.
    /// This agent has a noop provider and should never actually be used.
    pub(crate) fn placeholder() -> Self {
        use std::pin::Pin;

        struct NoopProvider;

        #[async_trait::async_trait]
        impl LlmProvider for NoopProvider {
            async fn send(
                &self,
                _messages: Vec<Message>,
                _tools: Vec<limit_llm::Tool>,
            ) -> Result<
                Pin<
                    Box<
                        dyn futures::Stream<
                                Item = Result<ProviderResponseChunk, limit_llm::LlmError>,
                            > + Send
                            + '_,
                    >,
                >,
                limit_llm::LlmError,
            > {
                Ok(Box::pin(futures::stream::empty()))
            }

            fn provider_name(&self) -> &str {
                "noop"
            }

            fn model_name(&self) -> &str {
                "noop"
            }

            fn clone_box(&self) -> Box<dyn LlmProvider> {
                Box::new(NoopProvider)
            }
        }

        let system_msg = Message {
            role: LlmRole::System,
            content: Some(Role::Jr.system_prompt().to_string()),
            tool_calls: None,
            tool_call_id: None,
        };
        Self {
            role: Role::Jr,
            provider: Box::new(NoopProvider),
            registry: Arc::new(ToolRegistry::new()),
            history: Arc::new(vec![system_msg]),
        }
    }

    /// Create a new team agent for the given role.
    pub fn new(role: Role, provider: Box<dyn LlmProvider>, registry: Arc<ToolRegistry>) -> Self {
        Self::with_allowed_tools(role, provider, registry, None)
    }

    /// Create a new team agent, restricting tools to the given whitelist.
    ///
    /// `allowed_tools`:
    /// - `None` → all tools in the registry are available
    /// - `Some(tools)` → only tools whose name appears in `tools` are available
    pub fn with_allowed_tools(
        role: Role,
        provider: Box<dyn LlmProvider>,
        registry: Arc<ToolRegistry>,
        allowed_tools: Option<Vec<String>>,
    ) -> Self {
        // Build a filtered registry if a whitelist is provided
        let registry = if let Some(ref whitelist) = allowed_tools {
            let mut filtered = ToolRegistry::new();
            for name in whitelist {
                if let Some(tool) = registry.get(name) {
                    filtered.register_arc(tool);
                    // Also copy the schema so the agent can build proper
                    // LLM tool definitions (description + parameters).
                    if let Some((desc, params)) = registry.get_schema(name) {
                        filtered.set_schema(name, desc.clone(), params.clone());
                    }
                }
            }
            Arc::new(filtered)
        } else {
            registry
        };

        let system_msg = Message {
            role: LlmRole::System,
            content: Some(role.system_prompt().to_string()),
            tool_calls: None,
            tool_call_id: None,
        };
        Self {
            role,
            provider,
            registry,
            history: Arc::new(vec![system_msg]),
        }
    }

    /// The agent's role.
    pub fn role(&self) -> &Role {
        &self.role
    }

    /// Read-only access to the conversation history.
    pub fn history(&self) -> &[Message] {
        &self.history
    }

    /// Push a message to history using copy-on-write.
    fn push_message(&mut self, msg: Message) {
        let mut hist = (*self.history).clone();
        hist.push(msg);
        self.history = Arc::new(hist);
    }

    /// Pop the last message from history using copy-on-write.
    fn pop_message(&mut self) -> Option<Message> {
        let mut hist = (*self.history).clone();
        let msg = hist.pop();
        self.history = Arc::new(hist);
        msg
    }

    /// Helper to push to Arc<Vec<Message>> using copy-on-write.
    fn push_to_history(history: &mut Arc<Vec<Message>>, msg: Message) {
        let mut hist = (**history).clone();
        hist.push(msg);
        *history = Arc::new(hist);
    }

    /// Reset conversation to just the system prompt.
    pub fn clear_history(&mut self) {
        let system_msg = Message {
            role: LlmRole::System,
            content: Some(self.role.system_prompt().to_string()),
            tool_calls: None,
            tool_call_id: None,
        };
        self.history = Arc::new(vec![system_msg]);
    }

    /// Trim conversation history to prevent unbounded growth.
    ///
    /// Keeps the system message and the most recent `max_messages` messages.
    /// This should be called periodically (e.g., after task completion)
    /// to prevent history from growing too large for the LLM context window.
    pub fn trim_history(&mut self, max_messages: usize) {
        if self.history.len() <= max_messages + 1 {
            return; // +1 for system message
        }

        // Always keep the system message
        let system_msg = self.history.first().cloned();
        if let Some(system) = system_msg {
            // Keep the most recent messages
            let start = self.history.len().saturating_sub(max_messages);
            let mut new_history = vec![system];
            new_history.extend(self.history[start..].to_vec());
            self.history = Arc::new(new_history);
        }
    }

    /// Send a user prompt and collect the full response.
    ///
    /// Handles tool calls automatically: if the provider returns tool-call
    /// chunks they are executed and the results are fed back.
    ///
    /// Transient LLM errors are retried up to [`MAX_RETRIES`] times with
    /// exponential backoff.
    pub async fn prompt(&mut self, user_input: &str) -> Result<PromptResult, AgentError> {
        self.push_message(Message {
            role: LlmRole::User,
            content: Some(user_input.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

        let tools = self.build_llm_tools();

        // Retry loop for transient failures.
        let mut attempt = 0;
        loop {
            match self.send_and_collect(tools.clone()).await {
                Ok((response, usage)) => {
                    // If there are tool calls in the response, execute them and continue
                    let has_tool_calls = self
                        .history
                        .last()
                        .and_then(|msg| msg.tool_calls.as_ref())
                        .map(|tc| !tc.is_empty())
                        .unwrap_or(false);

                    if has_tool_calls {
                        if let Some(tool_calls) =
                            self.history.last().and_then(|msg| msg.tool_calls.clone())
                        {
                            let result = self.handle_tool_calls(&tool_calls, usage).await?;
                            return Ok(result);
                        }
                        tracing::warn!(
                            "[team] {:?} has_tool_calls was true but no tool_calls found in last message",
                            self.role
                        );
                        // Continue with normal response flow
                    }

                    return Ok(PromptResult {
                        text: response,
                        usage,
                        hit_tool_limit: false,
                        files_modified: vec![],
                    });
                }
                Err(ref e) if is_retryable(e) && attempt < MAX_RETRIES => {
                    attempt += 1;
                    let delay = RETRY_BASE_DELAY * 2u32.pow(attempt as u32 - 1);
                    tracing::warn!(
                        "[team] {:?} prompt failed (attempt {}/{}): {}. Retrying in {:?}",
                        self.role,
                        attempt,
                        MAX_RETRIES,
                        e,
                        delay,
                    );
                    // Remove the failed assistant message so we can retry
                    if self
                        .history
                        .last()
                        .is_some_and(|m| m.role == LlmRole::Assistant)
                    {
                        self.pop_message();
                    }
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Send the current history to the provider and collect the response.
    async fn send_and_collect(
        &mut self,
        tools: Vec<limit_llm::Tool>,
    ) -> Result<(String, Usage), AgentError> {
        let mut stream = self
            .provider
            .send((*self.history).clone(), tools)
            .await
            .map_err(|e| AgentError::ToolError(format!("LLM error: {}", e)))?;

        let mut content = String::new();
        let mut tool_calls: Vec<limit_llm::ToolCall> = Vec::new();
        let mut accumulator = ToolCallAccumulator::default();
        let mut usage = zero_usage();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(limit_llm::ProviderResponseChunk::ContentDelta(text)) => {
                    content.push_str(&text);
                }
                Ok(limit_llm::ProviderResponseChunk::ToolCallDelta {
                    id,
                    name,
                    arguments,
                }) => {
                    if let Some(completed) = accumulator.process_delta(id, name, arguments) {
                        tool_calls.push(completed);
                    }
                }
                Ok(limit_llm::ProviderResponseChunk::Done(u)) => {
                    usage.input_tokens += u.input_tokens;
                    usage.output_tokens += u.output_tokens;
                    break;
                }
                Err(e) => {
                    return Err(AgentError::ToolError(format!("Stream error: {}", e)));
                }
                _ => {}
            }
        }
        // Drop stream explicitly before any mutable borrow
        drop(stream);

        // Flush last tool call if any
        if let Some(completed) = accumulator.flush() {
            tool_calls.push(completed);
        }

        // Store assistant message
        self.push_message(Message {
            role: LlmRole::Assistant,
            content: if content.is_empty() && !tool_calls.is_empty() {
                None
            } else {
                Some(content.clone())
            },
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            tool_call_id: None,
        });

        Ok((content.trim().to_string(), usage))
    }

    /// Execute tool calls, feed results back, and collect the final response.
    async fn handle_tool_calls(
        &mut self,
        tool_calls: &[limit_llm::ToolCall],
        mut usage: Usage,
    ) -> Result<PromptResult, AgentError> {
        let mut files_modified: Vec<String> = Vec::new();

        for tc in tool_calls {
            let args: Value = serde_json::from_str(&tc.function.arguments).unwrap_or(Value::Null);
            Self::track_file(&mut files_modified, &tc.function.name, &args);
            tracing::debug!(
                "[team] {:?} calling tool '{}' with args: {}",
                self.role,
                tc.function.name,
                tc.function.arguments
            );
            let result = self.registry.execute(&tc.function.name, args).await;

            let result_content = match result {
                Ok(val) => {
                    let s = serde_json::to_string(&val).unwrap_or_else(|_| "ok".to_string());
                    tracing::debug!(
                        "[team] {:?} tool '{}' succeeded: {}",
                        self.role,
                        tc.function.name,
                        &s[..s.len().min(200)]
                    );
                    s
                }
                Err(e) => {
                    tracing::warn!(
                        "[team] {:?} tool '{}' failed: {}",
                        self.role,
                        tc.function.name,
                        e
                    );
                    format!("Error: {}", e)
                }
            };

            self.push_message(Message {
                role: LlmRole::Tool,
                content: Some(result_content),
                tool_calls: None,
                tool_call_id: Some(tc.id.clone()),
            });
        }

        // Continue conversation with tool results, looping if the LLM
        // requests more tool calls (e.g., read file then edit it).
        let tools = self.build_llm_tools();
        let (mut response, round_usage) = self.send_and_collect(tools.clone()).await?;
        usage.input_tokens += round_usage.input_tokens;
        usage.output_tokens += round_usage.output_tokens;
        let mut tool_rounds = 1;

        // Loop: if the response contains more tool calls, execute them.
        // Bounded by MAX_TOOL_ROUNDS to prevent infinite loops.
        loop {
            if tool_rounds >= MAX_TOOL_ROUNDS {
                tracing::warn!(
                    "[team] {:?} hit tool-call limit ({}) — stopping",
                    self.role,
                    MAX_TOOL_ROUNDS
                );
                return Ok(PromptResult {
                    text: response,
                    usage,
                    hit_tool_limit: true,
                    files_modified,
                });
            }

            let has_more = self
                .history
                .last()
                .and_then(|msg| msg.tool_calls.as_ref())
                .map(|tc| !tc.is_empty())
                .unwrap_or(false);

            if !has_more {
                break;
            }

            let more_calls = self
                .history
                .last()
                .and_then(|msg| msg.tool_calls.clone())
                .unwrap_or_default();

            tool_rounds += 1;
            tracing::debug!(
                "[team] {:?} tool round {}/{} ({} calls)",
                self.role,
                tool_rounds,
                MAX_TOOL_ROUNDS,
                more_calls.len()
            );

            for tc in &more_calls {
                let args: Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or(Value::Null);
                Self::track_file(&mut files_modified, &tc.function.name, &args);
                tracing::debug!("[team] {:?} calling tool '{}'", self.role, tc.function.name);
                let result = self.registry.execute(&tc.function.name, args).await;
                let result_content = match result {
                    Ok(val) => serde_json::to_string(&val).unwrap_or_else(|_| "ok".to_string()),
                    Err(e) => format!("Error: {}", e),
                };
                self.push_message(Message {
                    role: LlmRole::Tool,
                    content: Some(result_content),
                    tool_calls: None,
                    tool_call_id: Some(tc.id.clone()),
                });
            }

            let (round_response, round_usage) = self.send_and_collect(tools.clone()).await?;
            response = round_response;
            usage.input_tokens += round_usage.input_tokens;
            usage.output_tokens += round_usage.output_tokens;
        }

        Ok(PromptResult {
            text: response,
            usage,
            hit_tool_limit: false,
            files_modified,
        })
    }

    /// Track file paths from file_write/file_edit tool calls.
    fn track_file(files: &mut Vec<String>, tool_name: &str, args: &Value) {
        if tool_name == "file_write" || tool_name == "file_edit" {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                if !files.contains(&path.to_string()) {
                    files.push(path.to_string());
                }
            }
        }
    }

    /// Build LLM tool definitions from the registry's tool list.
    fn build_llm_tools(&self) -> Vec<limit_llm::Tool> {
        // Build tool definitions from the registry for providers that need them.
        // Uses stored schemas (description + JSON Schema) when available,
        // falling back to generic definitions.
        self.registry
            .list()
            .into_iter()
            .filter_map(|name| {
                self.registry.get(&name).map(|_tool| {
                    let (description, parameters) =
                        self.registry.get_schema(&name).cloned().unwrap_or_else(|| {
                            (
                                format!("Tool: {}", name),
                                serde_json::json!({"type": "object"}),
                            )
                        });
                    limit_llm::Tool {
                        tool_type: "function".to_string(),
                        function: limit_llm::ToolFunction {
                            name,
                            description,
                            parameters,
                        },
                    }
                })
            })
            .collect()
    }

    /// Send a user prompt and return a streaming response.
    ///
    /// Returns chunks of text as they arrive from the LLM provider.
    /// Tool calls are handled automatically: if the provider returns
    /// tool-call chunks they are executed and the results are fed back
    /// before yielding more content.
    pub fn prompt_stream<'a>(
        &'a mut self,
        user_input: &'a str,
    ) -> impl futures::Stream<Item = Result<String, AgentError>> + 'a {
        async_stream::stream! {
            Self::push_to_history(&mut self.history, Message {
                role: LlmRole::User,
                content: Some(user_input.to_string()),
                tool_calls: None,
                tool_call_id: None,
            });

            let tools = self.build_llm_tools(); // Build tool definitions for provider

            let mut attempt = 0;
            let mut tool_rounds = 0;
            loop {
                let stream_result = self.provider.send((*self.history).clone(), tools.clone()).await;

                let mut stream = match stream_result {
                    Ok(s) => s,
                    Err(e) => {
                        if is_retryable_e(&e.to_string()) && attempt < MAX_RETRIES {
                            attempt += 1;
                            let delay = RETRY_BASE_DELAY * 2u32.pow(attempt as u32 - 1);
                            tracing::warn!(
                                "[team] {:?} stream failed (attempt {}/{}): {}. Retrying in {:?}",
                                self.role, attempt, MAX_RETRIES, e, delay,
                            );
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                        yield Err(AgentError::LlmError(e.to_string()));
                        return;
                    }
                };

                let mut content = String::new();
                let mut tool_calls: Vec<limit_llm::ToolCall> = Vec::new();
                let mut accumulator = ToolCallAccumulator::default();

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(ProviderResponseChunk::ContentDelta(text)) => {
                            content.push_str(&text);
                            yield Ok(text);
                        }
                        Ok(ProviderResponseChunk::ToolCallDelta {
                            id,
                            name,
                            arguments,
                        }) => {
                            if let Some(completed) = accumulator.process_delta(id, name, arguments) {
                                tool_calls.push(completed);
                            }
                        }
                        Ok(ProviderResponseChunk::Done(_)) => break,
                        Err(e) => {
                            yield Err(AgentError::LlmError(e.to_string()));
                            return;
                        }
                        _ => {}
                    }
                }
                drop(stream);

                // Flush last tool call if any
                if let Some(completed) = accumulator.flush() {
                    tool_calls.push(completed);
                }

                // Store assistant message
                let tool_calls_clone = tool_calls.clone();
                Self::push_to_history(&mut self.history, Message {
                    role: LlmRole::Assistant,
                    content: if content.is_empty() && !tool_calls_clone.is_empty() {
                        None
                    } else {
                        Some(content.clone())
                    },
                    tool_calls: if tool_calls_clone.is_empty() {
                        None
                    } else {
                        Some(tool_calls_clone)
                    },
                    tool_call_id: None,
                });

                // If there are tool calls, execute them and continue
                if !tool_calls.is_empty() {
                    tool_rounds += 1;
                    if tool_rounds >= MAX_TOOL_ROUNDS {
                        tracing::warn!(
                            "[team] {:?} stream hit tool-call limit ({}) — stopping",
                            self.role,
                            MAX_TOOL_ROUNDS
                        );
                        return;
                    }

                    for tc in &tool_calls {
                        let args: Value =
                            serde_json::from_str(&tc.function.arguments).unwrap_or(Value::Null);
                        let result = self.registry.execute(&tc.function.name, args).await;

                        let result_content = match result {
                            Ok(val) => {
                                serde_json::to_string(&val).unwrap_or_else(|_| "ok".to_string())
                            }
                            Err(e) => format!("Error: {}", e),
                        };

                        Self::push_to_history(&mut self.history, Message {
                            role: LlmRole::Tool,
                            content: Some(result_content),
                            tool_calls: None,
                            tool_call_id: Some(tc.id.clone()),
                        });
                    }
                    // Loop to get the response after tool results
                    continue;
                }

                return;
            }
        }
    }
}

/// Determine whether an error is transient and worth retrying.
/// Uses word-boundary matching to avoid false positives.
fn is_retryable(err: &AgentError) -> bool {
    is_retryable_e(&err.to_string())
}

fn is_retryable_e(msg: &str) -> bool {
    let lower = msg.to_lowercase();

    // Check for specific HTTP status codes with word boundaries
    if lower.contains("429") || lower.contains("502") || lower.contains("503") {
        return true;
    }

    // Check for rate limiting
    if lower.contains("rate limit") || lower.contains("rate_limit") {
        return true;
    }

    // Check for server errors
    if lower.contains("server error")
        || lower.contains("internal error")
        || lower.contains("service unavailable")
    {
        return true;
    }

    // Check for timeout and connection issues
    if lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("connection reset")
        || lower.contains("connection refused")
    {
        return true;
    }

    // Check for overload conditions
    if lower.contains("overloaded") || lower.contains("too many requests") {
        return true;
    }

    // Check for temporary failures
    if lower.contains("temporary") || lower.contains("retry") {
        return true;
    }

    // Check for specific 500 error (avoid false positives like "500 files")
    if lower.contains("http 500") || lower.contains("status 500") || lower.contains("error 500") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_agent_role() {
        assert_eq!(Role::PM.label(), "PM");
        assert_eq!(Role::TL.label(), "TL");
        assert_eq!(Role::Jr.label(), "Jr");
    }

    #[test]
    fn test_team_agent_registry_arc() {
        let registry = ToolRegistry::new();
        let _arc: Arc<ToolRegistry> = Arc::new(registry);
    }

    #[test]
    fn test_is_retryable_rate_limit() {
        let err = AgentError::LlmError("Rate limit exceeded (429)".into());
        assert!(is_retryable(&err));
    }

    #[test]
    fn test_is_retryable_server_error() {
        let err = AgentError::LlmError("Server error 503".into());
        assert!(is_retryable(&err));
    }

    #[test]
    fn test_is_retryable_timeout() {
        let err = AgentError::LlmError("Connection timeout".into());
        assert!(is_retryable(&err));
    }

    #[test]
    fn test_is_retryable_overloaded() {
        let err = AgentError::LlmError("Model overloaded".into());
        assert!(is_retryable(&err));
    }

    #[test]
    fn test_is_not_retryable_auth() {
        let err = AgentError::LlmError("Invalid API key".into());
        assert!(!is_retryable(&err));
    }

    #[test]
    fn test_is_not_retryable_tool_error() {
        let err = AgentError::ToolError("Tool not found".into());
        assert!(!is_retryable(&err));
    }

    #[test]
    fn test_retry_constants() {
        const _: () = assert!(MAX_RETRIES >= 2);
        const _: () = assert!(!RETRY_BASE_DELAY.is_zero());
    }

    #[test]
    fn test_tool_call_accumulator_single_tool() {
        let mut acc = ToolCallAccumulator::default();

        // No completed tool yet
        assert!(acc
            .process_delta(
                "id1".into(),
                "test_tool".into(),
                Value::String("arg".into())
            )
            .is_none());

        // Flush returns the completed tool
        let tool = acc.flush();
        assert!(tool.is_some());
        let tool = tool.unwrap();
        assert_eq!(tool.id, "id1");
        assert_eq!(tool.function.name, "test_tool");
        assert_eq!(tool.function.arguments, "arg");
    }

    #[test]
    fn test_tool_call_accumulator_multiple_tools() {
        let mut acc = ToolCallAccumulator::default();

        // First tool delta
        assert!(acc
            .process_delta("id1".into(), "tool1".into(), Value::String("arg1".into()))
            .is_none());

        // Second tool delta completes first tool
        let completed =
            acc.process_delta("id2".into(), "tool2".into(), Value::String("arg2".into()));
        assert!(completed.is_some());
        let tool1 = completed.unwrap();
        assert_eq!(tool1.id, "id1");
        assert_eq!(tool1.function.name, "tool1");
        assert_eq!(tool1.function.arguments, "arg1");

        // Flush returns second tool
        let tool2 = acc.flush();
        assert!(tool2.is_some());
        let tool2 = tool2.unwrap();
        assert_eq!(tool2.id, "id2");
        assert_eq!(tool2.function.name, "tool2");
        assert_eq!(tool2.function.arguments, "arg2");
    }

    #[test]
    fn test_tool_call_accumulator_structured_args() {
        let mut acc = ToolCallAccumulator::default();

        // Structured argument (object)
        acc.process_delta(
            "id1".into(),
            "tool1".into(),
            serde_json::json!({"key": "value"}),
        );

        let tool = acc.flush().unwrap();
        assert_eq!(tool.function.arguments, r#"{"key":"value"}"#);
    }

    #[test]
    fn test_tool_call_accumulator_string_fragments() {
        let mut acc = ToolCallAccumulator::default();

        // Multiple string fragments accumulate
        acc.process_delta("id1".into(), "tool1".into(), Value::String("part1".into()));
        acc.process_delta("id1".into(), "tool1".into(), Value::String("part2".into()));

        let tool = acc.flush().unwrap();
        assert_eq!(tool.function.arguments, "part1part2");
    }

    #[test]
    fn test_tool_call_accumulator_flush_empty() {
        let mut acc = ToolCallAccumulator::default();
        assert!(acc.flush().is_none());
    }
}
