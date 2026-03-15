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
            content: None, // Empty content
            tool_calls: Some(vec![ToolCall {
                id: "call_123".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "test_tool".to_string(),
                    arguments: serde_json::json!({}).to_string(),
                },
            }]),
            tool_call_id: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        println!("Assistant with tool_calls JSON: {}", json);
        // Content should be omitted when None
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
        };
        let json = serde_json::to_string(&usage).unwrap();
        let deserialized: Usage = serde_json::from_str(&json).unwrap();
        assert_eq!(usage.input_tokens, deserialized.input_tokens);
        assert_eq!(usage.output_tokens, deserialized.output_tokens);
    }
}
