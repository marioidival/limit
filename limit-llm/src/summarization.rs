use crate::error::LlmError;
use crate::providers::LlmProvider;
#[allow(unused_imports)]
use crate::types::ToolCall;
use crate::types::{FunctionCall, Message, Role};
use crate::ProviderResponseChunk;
use futures::StreamExt;
use std::collections::HashSet;

/// Result of summarization
pub struct SummaryOutput {
    pub summary: String,
    pub first_kept_index: usize,
    pub tokens_before: usize,
}

/// File operations extracted from tool calls
#[derive(Debug, Default)]
pub struct FileOperations {
    pub read: HashSet<String>,
    pub written: HashSet<String>,
    pub edited: HashSet<String>,
}

/// Extract file operations from tool calls in messages
pub fn extract_file_operations(messages: &[Message]) -> FileOperations {
    let mut ops = FileOperations::default();

    for msg in messages {
        if let Some(tool_calls) = &msg.tool_calls {
            for call in tool_calls {
                extract_from_tool_call(&call.function, &mut ops);
            }
        }
    }

    ops
}

fn extract_from_tool_call(func: &FunctionCall, ops: &mut FileOperations) {
    let path = extract_path_from_args(&func.arguments);

    match func.name.as_str() {
        "file_read" => {
            if let Some(p) = path {
                ops.read.insert(p);
            }
        }
        "file_write" => {
            if let Some(p) = path {
                ops.written.insert(p);
            }
        }
        "file_edit" => {
            if let Some(p) = path {
                ops.edited.insert(p);
            }
        }
        _ => {}
    }
}

fn extract_path_from_args(args: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(args)
        .ok()
        .and_then(|v| {
            v.get("path")
                .or_else(|| v.get("filePath"))
                .and_then(|p| p.as_str().map(|s| s.to_string()))
        })
}

/// Summarizer for context compaction
pub struct Summarizer {
    provider: Box<dyn LlmProvider>,
}

impl Summarizer {
    pub fn new(provider: Box<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    /// Generate summary of messages for context compaction
    pub async fn summarize(
        &self,
        messages: &[Message],
        previous_summary: Option<&str>,
    ) -> Result<String, LlmError> {
        let file_ops = extract_file_operations(messages);
        let prompt = build_summary_prompt(messages, previous_summary, &file_ops);

        let summary_request = vec![Message {
            role: Role::User,
            content: Some(prompt.into()),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        let mut stream = self.provider.send(summary_request, vec![]).await?;
        let mut result = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(ProviderResponseChunk::ContentDelta(text)) => result.push_str(&text),
                Err(e) => return Err(e),
                _ => {}
            }
        }

        Ok(result)
    }
}

fn build_summary_prompt(
    messages: &[Message],
    previous_summary: Option<&str>,
    file_ops: &FileOperations,
) -> String {
    let mut prompt = String::from("Summarize this conversation for context compaction.\n\n");

    if let Some(prev) = previous_summary {
        prompt.push_str("**Previous summary (update and condense):**\n");
        prompt.push_str(prev);
        prompt.push_str("\n\n");
    }

    prompt.push_str("**Messages to summarize:**\n");
    for msg in messages {
        match msg.role {
            Role::User => {
                prompt.push_str(&format!(
                    "User: {}\n",
                    msg.content
                        .as_ref()
                        .map(|c| c.to_text())
                        .unwrap_or_default()
                ));
            }
            Role::Assistant => {
                if let Some(content) = &msg.content {
                    prompt.push_str(&format!("Assistant: {}\n", content.to_text()));
                }
                if let Some(calls) = &msg.tool_calls {
                    for call in calls {
                        prompt.push_str(&format!(
                            "  Tool: {}({})\n",
                            call.function.name, call.function.arguments
                        ));
                    }
                }
            }
            Role::Tool => {
                prompt.push_str(&format!(
                    "Tool result: {}\n",
                    msg.content
                        .as_ref()
                        .map(|c| c.to_text())
                        .unwrap_or_default()
                ));
            }
            Role::System => {}
        }
    }

    if !file_ops.read.is_empty() || !file_ops.written.is_empty() || !file_ops.edited.is_empty() {
        prompt.push_str("\n**Files touched:**\n");
        for p in &file_ops.read {
            prompt.push_str(&format!("- Read: {}\n", p));
        }
        for p in &file_ops.edited {
            prompt.push_str(&format!("- Edited: {}\n", p));
        }
        for p in &file_ops.written {
            prompt.push_str(&format!("- Written: {}\n", p));
        }
    }

    prompt.push_str("\n**Output format:**\n");
    prompt.push_str("## Summary\n[2-3 sentences]\n\n");
    prompt.push_str("## Key Decisions\n- [decisions]\n\n");
    prompt.push_str("## Pending\n- [next steps]\n");

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool_call(name: &str, args: &str) -> ToolCall {
        ToolCall {
            id: "call_1".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: name.to_string(),
                arguments: args.to_string(),
            },
        }
    }

    #[test]
    fn test_extract_file_operations_read() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: None,
            tool_calls: Some(vec![make_tool_call(
                "file_read",
                r#"{"path": "/src/main.rs"}"#,
            )]),
            tool_call_id: None,
            cache_control: None,
        }];

        let ops = extract_file_operations(&messages);
        assert!(ops.read.contains("/src/main.rs"));
        assert!(ops.written.is_empty());
        assert!(ops.edited.is_empty());
    }

    #[test]
    fn test_extract_file_operations_edit() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: None,
            tool_calls: Some(vec![make_tool_call(
                "file_edit",
                r#"{"filePath": "/src/lib.rs", "oldString": "fn old", "newString": "fn new"}"#,
            )]),
            tool_call_id: None,
            cache_control: None,
        }];

        let ops = extract_file_operations(&messages);
        assert!(ops.edited.contains("/src/lib.rs"));
        assert!(ops.read.is_empty());
    }

    #[test]
    fn test_extract_file_operations_write() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: None,
            tool_calls: Some(vec![make_tool_call(
                "file_write",
                r#"{"path": "/src/new.rs", "content": "..."}"#,
            )]),
            tool_call_id: None,
            cache_control: None,
        }];

        let ops = extract_file_operations(&messages);
        assert!(ops.written.contains("/src/new.rs"));
    }

    #[test]
    fn test_extract_file_operations_multiple() {
        let messages = vec![
            Message {
                role: Role::Assistant,
                content: None,
                tool_calls: Some(vec![
                    make_tool_call("file_read", r#"{"path": "/src/a.rs"}"#),
                    make_tool_call("file_edit", r#"{"filePath": "/src/b.rs"}"#),
                ]),
                tool_call_id: None,
                cache_control: None,
            },
            Message {
                role: Role::Assistant,
                content: None,
                tool_calls: Some(vec![make_tool_call(
                    "file_write",
                    r#"{"path": "/src/c.rs"}"#,
                )]),
                tool_call_id: None,
                cache_control: None,
            },
        ];

        let ops = extract_file_operations(&messages);
        assert_eq!(ops.read.len(), 1);
        assert_eq!(ops.edited.len(), 1);
        assert_eq!(ops.written.len(), 1);
    }

    #[test]
    fn test_extract_file_operations_ignores_other_tools() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: None,
            tool_calls: Some(vec![make_tool_call("bash", r#"{"command": "ls"}"#)]),
            tool_call_id: None,
            cache_control: None,
        }];

        let ops = extract_file_operations(&messages);
        assert!(ops.read.is_empty());
        assert!(ops.written.is_empty());
        assert!(ops.edited.is_empty());
    }

    #[test]
    fn test_build_summary_prompt_includes_files() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: None,
            tool_calls: Some(vec![make_tool_call(
                "file_read",
                r#"{"path": "/src/main.rs"}"#,
            )]),
            tool_call_id: None,
            cache_control: None,
        }];

        let ops = extract_file_operations(&messages);
        let prompt = build_summary_prompt(&messages, None, &ops);

        assert!(prompt.contains("**Files touched:**"));
        assert!(prompt.contains("- Read: /src/main.rs"));
    }

    #[test]
    fn test_build_summary_prompt_with_previous() {
        let messages = vec![Message {
            role: Role::User,
            content: Some(crate::MessageContent::text("New message")),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        let ops = FileOperations::default();
        let prompt = build_summary_prompt(&messages, Some("Old summary"), &ops);

        assert!(prompt.contains("**Previous summary (update and condense):**"));
        assert!(prompt.contains("Old summary"));
    }
}
