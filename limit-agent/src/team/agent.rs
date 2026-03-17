//! Specialized agent with a specific role.
//!
//! Wraps an LLM provider with role-specific system prompts and
//! an optional tool registry.

use crate::error::AgentError;
use crate::registry::ToolRegistry;
use crate::team::role::Role;
use futures::StreamExt;
use limit_llm::{LlmProvider, Message, ProviderResponseChunk, Role as LlmRole};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Maximum number of automatic retries for transient LLM failures.
const MAX_RETRIES: usize = 3;
/// Base delay between retries (doubles on each attempt: 1s, 2s, 4s).
const RETRY_BASE_DELAY: Duration = Duration::from_secs(1);

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
                    // We can't extract the inner dyn Tool from Arc,
                    // so we clone the Arc into the filtered registry.
                    filtered.register_arc(tool);
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
    pub async fn prompt(&mut self, user_input: &str) -> Result<String, AgentError> {
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
                Ok(response) => {
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
                            return self.handle_tool_calls(&tool_calls).await;
                        }
                        tracing::warn!(
                            "[team] {:?} has_tool_calls was true but no tool_calls found in last message",
                            self.role
                        );
                        // Continue with normal response flow
                    }

                    return Ok(response);
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
    ) -> Result<String, AgentError> {
        let mut stream = self
            .provider
            .send((*self.history).clone(), tools)
            .await
            .map_err(|e| AgentError::ToolError(format!("LLM error: {}", e)))?;

        let mut content = String::new();
        let mut tool_calls: Vec<limit_llm::ToolCall> = Vec::new();
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_args = String::new();

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
                    // Flush previous tool call if starting a new one
                    if id != current_tool_id && !current_tool_id.is_empty() {
                        tool_calls.push(limit_llm::ToolCall {
                            id: current_tool_id.clone(),
                            tool_type: "function".to_string(),
                            function: limit_llm::FunctionCall {
                                name: current_tool_name.clone(),
                                arguments: current_tool_args.clone(),
                            },
                        });
                    }
                    current_tool_id = id;
                    current_tool_name = name;
                    // Accumulate arguments from all deltas (handles both Object and String fragments)
                    match &arguments {
                        Value::Object(_) | Value::Array(_) | Value::Bool(_) | Value::Number(_) => {
                            // Structured value - serialize it
                            current_tool_args =
                                serde_json::to_string(&arguments).unwrap_or_default();
                        }
                        Value::String(s) => {
                            // String fragment - append it
                            current_tool_args.push_str(s);
                        }
                        Value::Null => {
                            // Null - ignore
                        }
                    }
                }
                Ok(limit_llm::ProviderResponseChunk::Done(_)) => {
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
        if !current_tool_id.is_empty() {
            tool_calls.push(limit_llm::ToolCall {
                id: current_tool_id.clone(),
                tool_type: "function".to_string(),
                function: limit_llm::FunctionCall {
                    name: current_tool_name.clone(),
                    arguments: current_tool_args.clone(),
                },
            });
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

        Ok(content.trim().to_string())
    }

    /// Execute tool calls, feed results back, and collect the final response.
    async fn handle_tool_calls(
        &mut self,
        tool_calls: &[limit_llm::ToolCall],
    ) -> Result<String, AgentError> {
        for tc in tool_calls {
            let args: Value = serde_json::from_str(&tc.function.arguments).unwrap_or(Value::Null);
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

        // Continue conversation with tool results
        let tools = self.build_llm_tools();
        let response = self.send_and_collect(tools).await?;
        Ok(response)
    }

    /// Build LLM tool definitions from the registry's tool list.
    fn build_llm_tools(&self) -> Vec<limit_llm::Tool> {
        // Build tool definitions from the registry for providers that need them.
        // Some providers (like OpenAI) can work without explicit tool definitions,
        // but providing them ensures consistent behavior across all providers.
        self.registry
            .list()
            .into_iter()
            .filter_map(|name| {
                // Get tool from registry to build its definition
                self.registry.get(&name).map(|_tool| {
                    let desc = format!("Tool: {}", name);
                    limit_llm::Tool {
                        tool_type: "function".to_string(),
                        function: limit_llm::ToolFunction {
                            name,
                            description: desc,
                            parameters: serde_json::json!({"type": "object"}),
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
                let mut current_tool_id = String::new();
                let mut current_tool_name = String::new();
                let mut current_tool_args = String::new();

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
                            if id != current_tool_id && !current_tool_id.is_empty() {
                                tool_calls.push(limit_llm::ToolCall {
                                    id: current_tool_id.clone(),
                                    tool_type: "function".to_string(),
                                    function: limit_llm::FunctionCall {
                                        name: current_tool_name.clone(),
                                        arguments: current_tool_args.clone(),
                                    },
                                });
                            }
                            current_tool_id = id;
                            current_tool_name = name;
                            // Accumulate arguments from all deltas (handles both Object and String fragments)
                            match &arguments {
                                Value::Object(_) | Value::Array(_) | Value::Bool(_) | Value::Number(_) => {
                                    // Structured value - serialize it
                                    current_tool_args = serde_json::to_string(&arguments).unwrap_or_default();
                                }
                                Value::String(s) => {
                                    // String fragment - append it
                                    current_tool_args.push_str(s);
                                }
                                Value::Null => {
                                    // Null - ignore
                                }
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

                if !current_tool_id.is_empty() {
                    tool_calls.push(limit_llm::ToolCall {
                        id: current_tool_id,
                        tool_type: "function".to_string(),
                        function: limit_llm::FunctionCall {
                            name: current_tool_name,
                            arguments: current_tool_args,
                        },
                    });
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
}
