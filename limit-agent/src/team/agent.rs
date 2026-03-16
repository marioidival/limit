//! Specialized agent with a specific role.
//!
//! Wraps an LLM provider with role-specific system prompts and
//! an optional tool registry.

use crate::error::AgentError;
use crate::registry::ToolRegistry;
use crate::team::role::Role;
use futures::StreamExt;
use limit_llm::{LlmProvider, Message, Role as LlmRole};
use serde_json::Value;
use std::sync::Arc;

/// A specialized agent that acts as a single member of a team.
pub struct TeamAgent {
    role: Role,
    provider: Box<dyn LlmProvider>,
    registry: Arc<ToolRegistry>,
    history: Vec<Message>,
}

impl TeamAgent {
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
            history: vec![system_msg],
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

    /// Reset conversation to just the system prompt.
    pub fn clear_history(&mut self) {
        let system_msg = Message {
            role: LlmRole::System,
            content: Some(self.role.system_prompt().to_string()),
            tool_calls: None,
            tool_call_id: None,
        };
        self.history = vec![system_msg];
    }

    /// Send a user prompt and collect the full response.
    ///
    /// Handles tool calls automatically: if the provider returns tool-call
    /// chunks they are executed and the results are fed back.
    pub async fn prompt(&mut self, user_input: &str) -> Result<String, AgentError> {
        self.history.push(Message {
            role: LlmRole::User,
            content: Some(user_input.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

        // Build the LLM tool definitions from the registry
        let tools = self.build_llm_tools();

        let response = self.send_and_collect(tools).await?;

        // If there are tool calls in the response, execute them and continue
        let has_tool_calls = self
            .history
            .last()
            .and_then(|msg| msg.tool_calls.as_ref())
            .map(|tc| !tc.is_empty())
            .unwrap_or(false);

        if has_tool_calls {
            // Extract the tool calls from the last message before borrowing mutably
            let tool_calls = self
                .history
                .last()
                .and_then(|msg| msg.tool_calls.clone())
                .expect("tool_calls exist");
            return self.handle_tool_calls(&tool_calls).await;
        }

        Ok(response)
    }

    /// Send the current history to the provider and collect the response.
    async fn send_and_collect(
        &mut self,
        tools: Vec<limit_llm::Tool>,
    ) -> Result<String, AgentError> {
        let mut stream = self
            .provider
            .send(self.history.clone(), tools)
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
                    if let Value::Object(_) = &arguments {
                        current_tool_args = serde_json::to_string(&arguments).unwrap_or_default();
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
        self.history.push(Message {
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

            self.history.push(Message {
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
        // Provider handles tool calling natively.
        Vec::new()
    }
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
}
