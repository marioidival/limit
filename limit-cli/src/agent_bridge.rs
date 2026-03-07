use crate::error::CliError;
use crate::tools::{
    AstGrepTool, BashTool, FileEditTool, FileReadTool, FileWriteTool, GitAddTool, GitCloneTool,
    GitCommitTool, GitDiffTool, GitLogTool, GitPullTool, GitPushTool, GitStatusTool, GrepTool,
    LspTool,
};
use futures::StreamExt;
use limit_agent::executor::{ToolCall, ToolExecutor};
use limit_agent::registry::ToolRegistry;
use limit_llm::client::{AnthropicClient, ResponseChunk};
use limit_llm::types::{Message, Role, Tool as LlmTool, ToolCall as LlmToolCall};
use serde_json::json;
use tokio::sync::mpsc;
use tracing::instrument;

/// Event types for streaming from agent to REPL
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AgentEvent {
    Thinking,
    ToolStart {
        name: String,
        args: serde_json::Value,
    },
    ToolComplete {
        name: String,
        result: String,
    },
    ContentChunk(String),
    Done,
    Error(String),
}

/// Bridge connecting limit-cli REPL to limit-agent executor and limit-llm client
pub struct AgentBridge {
    /// LLM client for communicating with Anthropic API
    llm_client: AnthropicClient,
    /// Tool executor for running tool calls
    executor: ToolExecutor,
    /// List of registered tool names
    tool_names: Vec<&'static str>,
    /// Configuration loaded from ~/.limit/config.toml
    config: limit_llm::Config,
    /// Event sender for streaming events to REPL
    event_tx: Option<mpsc::UnboundedSender<AgentEvent>>,
}

impl AgentBridge {
    /// Create a new AgentBridge with the given configuration
    ///
    /// # Arguments
    /// * `config` - LLM configuration (API key, model, etc.)
    ///
    /// # Returns
    /// A new AgentBridge instance or an error if initialization fails
    pub fn new(config: limit_llm::Config) -> Result<Self, CliError> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| CliError::ConfigError("API key not found in config".to_string()))?;

        let llm_client = AnthropicClient::new(
            api_key.clone(),
            config.base_url.as_deref(),
            config.timeout,
            &config.model,
            config.max_tokens,
        );

        let mut tool_registry = ToolRegistry::new();
        Self::register_tools(&mut tool_registry);

        // Create executor (which takes ownership of registry as Arc)
        let executor = ToolExecutor::new(tool_registry);

        // Generate tool definitions before giving ownership to executor
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
            "grep",
            "ast_grep",
            "lsp",
        ];

        Ok(Self {
            llm_client,
            executor,
            tool_names,
            config,
            event_tx: None,
        })
    }

    /// Set the event channel sender for streaming events
    pub fn set_event_tx(&mut self, tx: mpsc::UnboundedSender<AgentEvent>) {
        self.event_tx = Some(tx);
    }

    /// Register all CLI tools into the tool registry
    fn register_tools(registry: &mut ToolRegistry) {
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
        registry
            .register(GrepTool::new())
            .expect("Failed to register grep");
        registry
            .register(AstGrepTool::new())
            .expect("Failed to register ast_grep");
        registry
            .register(LspTool::new())
            .expect("Failed to register lsp");
    }

    /// Process a user message through the LLM and execute any tool calls
    ///
    /// # Arguments
    /// * `user_input` - The user's message to process
    /// * `messages` - The conversation history (will be updated in place)
    ///
    /// # Returns
    /// The final response from the LLM or an error
    #[instrument(skip(self, messages))]
    /// The final response from the LLM or an error
    pub async fn process_message(
        &mut self,
        user_input: &str,
        messages: &mut Vec<Message>,
    ) -> Result<String, CliError> {
        // Add user message to history
        let user_message = Message {
            role: Role::User,
            content: user_input.to_string(),
            tool_calls: None,
        };
        messages.push(user_message.clone());

        // Get tool definitions
        let tool_definitions = self.get_tool_definitions();

        // Main processing loop
        let mut full_response = String::new();
        let mut tool_calls: Vec<LlmToolCall> = Vec::new();
        let max_iterations = 10;
        let mut iteration = 0;

        while iteration < max_iterations {
            iteration += 1;

            // Send thinking event
            self.send_event(AgentEvent::Thinking);

            // Call LLM
            let mut stream = self
                .llm_client
                .send(messages.clone(), tool_definitions.clone())
                .await;

            tool_calls.clear();
            let mut current_content = String::new();
            // Track tool calls: (id) -> (name, args)
            let mut accumulated_calls: std::collections::HashMap<
                String,
                (String, serde_json::Value),
            > = std::collections::HashMap::new();

            // Process stream chunks
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(ResponseChunk::ContentDelta(text)) => {
                        current_content.push_str(&text);
                        self.send_event(AgentEvent::ContentChunk(text));
                    }
                    Ok(ResponseChunk::ToolCallDelta {
                        id,
                        name,
                        arguments,
                    }) => {
                        // Store/merge tool call arguments
                        accumulated_calls.insert(id.clone(), (name.clone(), arguments.clone()));
                    }
                    Ok(ResponseChunk::Done(_)) => {
                        break;
                    }
                    Err(e) => {
                        let error_msg = format!("LLM error: {}", e);
                        self.send_event(AgentEvent::Error(error_msg.clone()));
                        return Err(CliError::ConfigError(error_msg));
                    }
                }
            }

            // Convert accumulated calls to Vec<ToolCall>
            tool_calls = accumulated_calls
                .into_iter()
                .map(|(id, (name, args))| LlmToolCall {
                    id,
                    tool_type: "function".to_string(),
                    function: limit_llm::types::FunctionCall {
                        name,
                        arguments: args,
                    },
                })
                .collect();
            full_response.push_str(&current_content);

            // If no tool calls, we're done
            if tool_calls.is_empty() {
                break;
            }

            // Execute tool calls
            let assistant_message = if current_content.is_empty() {
                Message {
                    role: Role::Assistant,
                    content: String::new(),
                    tool_calls: Some(tool_calls.clone()),
                }
            } else {
                Message {
                    role: Role::Assistant,
                    content: current_content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                }
            };
            messages.push(assistant_message);

            // Convert LLM tool calls to executor tool calls
            let executor_calls: Vec<ToolCall> = tool_calls
                .iter()
                .map(|tc| ToolCall::new(&tc.id, &tc.function.name, tc.function.arguments.clone()))
                .collect();

            // Execute tools
            let results = self.executor.execute_tools(executor_calls).await;

            // Add tool results to messages
            for result in results {
                let tool_call = tool_calls.iter().find(|tc| tc.id == result.call_id);
                if let Some(tool_call) = tool_call {
                    let output_json = match &result.output {
                        Ok(value) => serde_json::to_string_pretty(value).unwrap_or_else(|_| {
                            serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
                        }),
                        Err(e) => json!({ "error": e.to_string() }).to_string(),
                    };

                    self.send_event(AgentEvent::ToolComplete {
                        name: tool_call.function.name.clone(),
                        result: output_json.clone(),
                    });

                    let tool_result_message = Message {
                        role: Role::User,
                        content: serde_json::json!({
                            "tool_use_id": result.call_id,
                            "output": output_json
                        })
                        .to_string(),
                        tool_calls: None,
                    };
                    messages.push(tool_result_message);
                }
            }
        }

        self.send_event(AgentEvent::Done);
        Ok(full_response)
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
                "Search code using AST patterns (structural code matching)".to_string(),
                json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "AST pattern to match"
                        },
                        "language": {
                            "type": "string",
                            "description": "Programming language (rust, typescript, python)"
                        },
                        "path": {
                            "type": "string",
                            "description": "Path to search in (default: current directory)"
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
        self.config.api_key.is_some()
    }

    /// Get the current model name
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Get the max tokens setting
    pub fn max_tokens(&self) -> u32 {
        self.config.max_tokens
    }

    /// Get the timeout setting
    pub fn timeout(&self) -> u64 {
        self.config.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use limit_llm::Config as LlmConfig;

    #[tokio::test]
    async fn test_agent_bridge_new() {
        let config = LlmConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            timeout: 60,
            base_url: None,
        };

        let bridge = AgentBridge::new(config).unwrap();
        assert!(bridge.is_ready());
    }

    #[tokio::test]
    async fn test_agent_bridge_new_no_api_key() {
        let config = LlmConfig {
            api_key: None,
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            timeout: 60,
            base_url: None,
        };

        let result = AgentBridge::new(config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_tool_definitions() {
        let config = LlmConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            timeout: 60,
            base_url: None,
        };

        let bridge = AgentBridge::new(config).unwrap();
        let definitions = bridge.get_tool_definitions();

        assert_eq!(definitions.len(), 15);

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
        let config_with_key = LlmConfig {
            api_key: Some("test-key".to_string()),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            timeout: 60,
            base_url: None,
        };

        let bridge = AgentBridge::new(config_with_key.clone()).unwrap();
        assert!(bridge.is_ready());
    }
}
