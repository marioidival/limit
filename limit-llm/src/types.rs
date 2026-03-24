//! Core types for LLM message passing and tool definitions.
//!
//! This module provides the fundamental types used throughout `limit-llm` for
//! constructing messages, defining tools, and handling responses from LLM providers.
//!
//! # Overview
//!
//! - [`Message`] — A single message in a conversation with role and content
//! - [`Role`] — The sender role (User, Assistant, System, or Tool)
//! - [`Tool`] / [`ToolCall`] — Function calling definitions for LLM tool use
//! - [`Response`] — Complete response with content, tool calls, and usage
//! - [`Usage`] — Token counting for prompt and completion

use serde::{Deserialize, Serialize};

/// A single message in a conversation.
///
/// Messages are the fundamental unit of communication with LLM providers.
/// Each message has a role (who sent it), content (the text), and optionally
/// tool calls (for function calling).
///
/// # Examples
///
/// ## User Message
///
/// ```
/// use limit_llm::{Message, Role};
///
/// let msg = Message {
///     role: Role::User,
///     content: Some("What is the capital of France?".to_string()),
///     tool_calls: None,
///     tool_call_id: None,
/// };
/// ```
///
/// ## Assistant Message with Tool Call
///
/// ```
/// use limit_llm::{Message, Role, ToolCall, FunctionCall};
///
/// let msg = Message {
///     role: Role::Assistant,
///     content: None,
///     tool_calls: Some(vec![ToolCall {
///         id: "call_123".to_string(),
///         tool_type: "function".to_string(),
///         function: FunctionCall {
///             name: "get_weather".to_string(),
///             arguments: r#"{"location": "Paris"}"#.to_string(),
///         },
///     }]),
///     tool_call_id: None,
/// };
/// ```
///
/// ## Tool Result Message
///
/// ```
/// use limit_llm::{Message, Role};
///
/// let msg = Message {
///     role: Role::Tool,
///     content: Some(r#"{"temp": 22, "condition": "sunny"}"#.to_string()),
///     tool_calls: None,
///     tool_call_id: Some("call_123".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message sender.
    pub role: Role,

    /// The text content of the message.
    ///
    /// Can be `None` for assistant messages that only contain tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Tool calls made by the assistant.
    ///
    /// Only present in assistant messages when the LLM decides to call tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// ID of the tool call this message is responding to.
    ///
    /// Only present in tool result messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Cache control for prompt caching (Anthropic/OpenAI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

/// The role of a message sender in a conversation.
///
/// # Serialization
///
/// Roles are serialized as lowercase strings:
/// - `User` → `"user"`
/// - `Assistant` → `"assistant"`
/// - `System` → `"system"`
/// - `Tool` → `"tool"`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A message from the user.
    User,

    /// A message from the assistant (LLM).
    Assistant,

    /// A system message providing instructions or context.
    System,

    /// A tool result message containing the output of a tool execution.
    Tool,
}

/// Cache control settings for prompt caching.
///
/// Used to enable API-level caching of messages to reduce input token costs.
/// Supported by Anthropic Claude and OpenAI models.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheControl {
    /// The type of cache control. Currently only "ephemeral" is supported.
    #[serde(rename = "type")]
    pub cache_type: String,

    /// Time-to-live for the cache entry (Anthropic only).
    /// Options: "5m" (default), "1h" when long retention is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
}

impl CacheControl {
    /// Create a new ephemeral cache control with default TTL.
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: None,
        }
    }

    /// Create an ephemeral cache control with long TTL (1 hour).
    pub fn ephemeral_long() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: Some("1h".to_string()),
        }
    }
}

/// A tool call made by the assistant.
///
/// When an LLM decides to use a tool, it returns a `ToolCall` containing
/// the tool ID, type, and the function to call with its arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call.
    pub id: String,

    /// The type of tool (always "function" for now).
    #[serde(rename = "type")]
    pub tool_type: String,

    /// The function call details.
    pub function: FunctionCall,
}

/// A function call with name and JSON arguments.
///
/// The `arguments` field contains a JSON string representing the function
/// parameters as defined in the tool schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function to call.
    pub name: String,

    /// JSON string representation of the function arguments.
    ///
    /// This is a string because LLMs return arguments as JSON strings
    /// during streaming. Parse with `serde_json::from_str` if needed.
    pub arguments: String,
}

/// A tool definition for LLM function calling.
///
/// Tools allow LLMs to perform actions by calling functions with structured
/// parameters. Define tools with JSON Schema for the parameters.
///
/// # Example
///
/// ```
/// use limit_llm::{Tool, ToolFunction};
/// use serde_json::json;
///
/// let tool = Tool {
///     tool_type: "function".to_string(),
///     function: ToolFunction {
///         name: "get_weather".to_string(),
///         description: "Get current weather for a location".to_string(),
///         parameters: json!({
///             "type": "object",
///             "properties": {
///                 "location": {
///                     "type": "string",
///                     "description": "City name"
///                 }
///             },
///             "required": ["location"]
///         }),
///     },
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The type of tool (always "function" for now).
    #[serde(rename = "type")]
    pub tool_type: String,

    /// The function definition.
    pub function: ToolFunction,
}

/// Function definition within a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    /// The function name. Must be unique within the tool set.
    pub name: String,

    /// Human-readable description of what the function does.
    /// This helps the LLM understand when to use the tool.
    pub description: String,

    /// JSON Schema defining the function parameters.
    ///
    /// Use `serde_json::json!` to construct the schema inline.
    pub parameters: serde_json::Value,
}

/// A complete response from an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// The text content of the response.
    pub content: String,

    /// Tool calls made by the assistant, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Token usage statistics.
    pub usage: Usage,
}

/// Token usage statistics for a request.
///
/// Tracks the number of tokens used in the prompt (input) and
/// completion (output). Use with [`TrackingDb`](crate::TrackingDb)
/// to monitor costs across sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt/input.
    pub input_tokens: u64,

    /// Number of tokens in the completion/output.
    pub output_tokens: u64,

    /// Number of tokens read from cache (~10% of input cost).
    #[serde(default, alias = "cache_read_input_tokens")]
    pub cache_read_tokens: u64,

    /// Number of tokens written to cache.
    #[serde(default, alias = "cache_creation_input_tokens")]
    pub cache_write_tokens: u64,
}

impl Usage {
    /// Calculate total tokens including cache operations.
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_read_tokens + self.cache_write_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            role: Role::User,
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.content, deserialized.content);
    }

    #[test]
    fn test_message_with_tool_calls() {
        let msg = Message {
            role: Role::Assistant,
            content: Some("".to_string()),
            tool_calls: Some(vec![ToolCall {
                id: "call_123".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "test_tool".to_string(),
                    arguments: serde_json::json!({"arg": "value"}).to_string(),
                },
            }]),
            tool_call_id: None,
            cache_control: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(deserialized.tool_calls.is_some());
    }

    #[test]
    fn test_tool_result_message() {
        let msg = Message {
            role: Role::Tool,
            content: Some("result output".to_string()),
            tool_calls: None,
            tool_call_id: Some("call_123".to_string()),
            cache_control: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        println!("Tool result message JSON: {}", json);
        assert!(json.contains("tool_call_id"));
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tool_call_id, Some("call_123".to_string()));
    }

    #[test]
    fn test_assistant_with_tool_calls_serialization() {
        let msg = Message {
            role: Role::Assistant,
            content: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_123".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "test_tool".to_string(),
                    arguments: serde_json::json!({}).to_string(),
                },
            }]),
            tool_call_id: None,
            cache_control: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        println!("Assistant with tool_calls JSON: {}", json);
        assert!(!json.contains("\"content\":null"));
        assert!(json.contains("tool_calls"));
    }

    #[test]
    fn test_role_serialization() {
        let role = Role::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "test_tool".to_string(),
                description: "A test tool".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
        };
        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: Tool = serde_json::from_str(&json).unwrap();
        assert_eq!(tool.function.name, deserialized.function.name);
    }

    #[test]
    fn test_response_serialization() {
        let response = Response {
            content: "Hello, world!".to_string(),
            tool_calls: None,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 5,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&json).unwrap();
        assert_eq!(response.content, deserialized.content);
        assert_eq!(response.usage.input_tokens, deserialized.usage.input_tokens);
    }

    #[test]
    fn test_usage_serialization() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
        };
        let json = serde_json::to_string(&usage).unwrap();
        let deserialized: Usage = serde_json::from_str(&json).unwrap();
        assert_eq!(usage.input_tokens, deserialized.input_tokens);
        assert_eq!(usage.output_tokens, deserialized.output_tokens);
    }

    #[test]
    fn test_cache_control_serialization() {
        let cache = CacheControl::ephemeral();
        let json = serde_json::to_string(&cache).unwrap();
        assert_eq!(json, r#"{"type":"ephemeral"}"#);

        let cache_long = CacheControl::ephemeral_long();
        let json_long = serde_json::to_string(&cache_long).unwrap();
        assert!(json_long.contains(r#""ttl":"1h""#));
    }

    #[test]
    fn test_message_with_cache_control() {
        let msg = Message {
            role: Role::User,
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
            cache_control: Some(CacheControl::ephemeral()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("cache_control"));
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(deserialized.cache_control.is_some());
    }

    #[test]
    fn test_usage_with_cache_fields() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: 80,
            cache_write_tokens: 20,
        };
        assert_eq!(usage.total_tokens(), 250);

        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("cache_read_tokens"));
    }

    #[test]
    fn test_usage_anthropic_aliases() {
        let json = r#"{
            "input_tokens": 100,
            "output_tokens": 50,
            "cache_read_input_tokens": 80,
            "cache_creation_input_tokens": 20
        }"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.cache_read_tokens, 80);
        assert_eq!(usage.cache_write_tokens, 20);
        assert_eq!(usage.total_tokens(), 250);
    }
}
