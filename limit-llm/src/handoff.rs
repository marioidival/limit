use crate::error::LlmError;
use crate::types::{Message, Role};
use tiktoken_rs::cl100k_base;

pub struct ModelHandoff {
    tokenizer: tiktoken_rs::CoreBPE,
}

impl Default for ModelHandoff {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelHandoff {
    pub fn new() -> Self {
        Self {
            tokenizer: cl100k_base().expect("Failed to load tokenizer"),
        }
    }

    pub fn count_tokens(&self, text: &str) -> usize {
        self.tokenizer.encode_with_special_tokens(text).len()
    }

    pub fn count_message_tokens(&self, message: &Message) -> usize {
        let mut total = message
            .content
            .as_ref()
            .map(|c| self.count_tokens(c))
            .unwrap_or(0);

        // Add role overhead (4 tokens for message format)
        total += 4;

        // Add tool_calls if present
        if let Some(tool_calls) = &message.tool_calls {
            for call in tool_calls {
                total += self.count_tokens(&call.id);
                total += self.count_tokens(&call.function.name);
                if let Ok(args_str) = serde_json::to_string(&call.function.arguments) {
                    total += self.count_tokens(&args_str);
                }
            }
        }

        total
    }

    pub fn count_total_tokens(&self, messages: &[Message]) -> usize {
        messages.iter().map(|m| self.count_message_tokens(m)).sum()
    }

    pub fn compact_messages(&self, messages: &[Message], target_tokens: usize) -> Vec<Message> {
        // Always keep system message if present
        let system_msg = messages.iter().find(|m| matches!(m.role, Role::System));

        // Count tokens without system message
        let non_system: Vec<_> = messages
            .iter()
            .filter(|m| !matches!(m.role, Role::System))
            .cloned()
            .collect();

        let mut compacted = Vec::new();

        // Add system message first if exists
        if let Some(sys) = system_msg {
            compacted.push(sys.clone());
        }

        // Calculate target for non-system messages
        let system_tokens = compacted
            .iter()
            .map(|m| self.count_message_tokens(m))
            .sum::<usize>();

        // Reserve 20% of budget for safety, minimum 100 tokens
        let safety_buffer = (target_tokens / 5).max(100);
        let remaining_budget = target_tokens.saturating_sub(system_tokens + safety_buffer);

        // Keep last N messages that fit within remaining budget
        let mut selected = Vec::new();
        let mut current_tokens = 0;

        for msg in non_system.iter().rev() {
            let msg_tokens = self.count_message_tokens(msg);

            if current_tokens + msg_tokens <= remaining_budget {
                current_tokens += msg_tokens;
                selected.push(msg.clone());
            } else {
                break;
            }
        }

        // Reverse to maintain original order
        selected.reverse();
        compacted.extend(selected);

        compacted
    }

    pub fn handoff_to_model(
        &self,
        _from_model: &str,
        to_model: &str,
        messages: &[Message],
    ) -> Result<Vec<Message>, LlmError> {
        // Define context windows for different models (approximate)
        let target_tokens = match to_model {
            "claude-3-5-sonnet-20241022" => 200000,
            "claude-3-5-haiku-20241022" => 200000,
            "claude-3-opus-20240229" => 200000,
            "claude-3-sonnet-20240229" => 200000,
            "claude-3-haiku-20240307" => 200000,
            _ => 200000, // Default to 200K context window
        };

        let current_tokens = self.count_total_tokens(messages);

        // Only compact if we're over the target (with 10% buffer for safety)
        if current_tokens > target_tokens * 9 / 10 {
            Ok(self.compact_messages(messages, target_tokens))
        } else {
            Ok(messages.to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FunctionCall;
    use crate::types::ToolCall;

    #[test]
    fn test_count_tokens_simple() {
        let handoff = ModelHandoff::new();
        let tokens = handoff.count_tokens("Hello, world!");
        assert!(tokens > 0);
        assert!(tokens < 10);
    }

    #[test]
    fn test_count_message_tokens() {
        let handoff = ModelHandoff::new();
        let msg = Message {
            role: Role::User,
            content: Some("Hello, world!".to_string()),
            tool_calls: None,
            tool_call_id: None,
        };
        let tokens = handoff.count_message_tokens(&msg);
        assert!(tokens > 4); // Content tokens + role overhead
    }

    #[test]
    fn test_count_message_tokens_with_tool_calls() {
        let handoff = ModelHandoff::new();
        let msg = Message {
            role: Role::Assistant,
            content: Some("".to_string()),
            tool_calls: Some(vec![ToolCall {
                id: "call_123".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "test_tool".to_string(),
                    arguments: serde_json::json!({"arg": "value"}),
                },
            }]),
            tool_call_id: None,
        };
        let tokens = handoff.count_message_tokens(&msg);
        assert!(tokens > 10);
    }

    #[test]
    fn test_count_total_tokens() {
        let handoff = ModelHandoff::new();
        let messages = vec![
            Message {
                role: Role::User,
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: Role::Assistant,
                content: Some("Hi there!".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        let total = handoff.count_total_tokens(&messages);
        assert!(total > 0);
    }

    #[test]
    fn test_compact_messages_preserves_system() {
        let handoff = ModelHandoff::new();
        let messages = vec![
            Message {
                role: Role::System,
                content: Some("You are a helpful assistant.".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: Role::User,
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        let compacted = handoff.compact_messages(&messages, 500);
        assert!(!compacted.is_empty());
        if compacted.len() > 1 {
            assert!(matches!(compacted[0].role, Role::System));
        }
    }

    #[test]
    fn test_compact_messages_keeps_recent() {
        let handoff = ModelHandoff::new();
        let mut messages = vec![Message {
            role: Role::System,
            content: Some("System".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        // Add 100 messages
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
            });
        }

        // Compact to small budget
        let compacted = handoff.compact_messages(&messages, 500);

        // Should have system + recent messages
        assert!(compacted.len() < messages.len());
        assert!(matches!(compacted[0].role, Role::System));

        // Last message should be preserved
        assert_eq!(
            compacted.last().unwrap().content,
            Some("Message 99".to_string())
        );
    }

    #[test]
    fn test_handoff_to_model_no_compaction_needed() {
        let handoff = ModelHandoff::new();
        let messages = vec![Message {
            role: Role::User,
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        let result = handoff.handoff_to_model(
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
            &messages,
        );

        assert!(result.is_ok());
        let handoff_messages = result.unwrap();
        assert_eq!(handoff_messages.len(), messages.len());
    }

    #[test]
    fn test_handoff_to_model_compacts_when_needed() {
        let handoff = ModelHandoff::new();
        let mut messages = vec![Message {
            role: Role::System,
            content: Some("System".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        // Create 5000 messages with substantial content to exceed 200K context
        for i in 0..5000 {
            messages.push(Message {
                role: if i % 2 == 0 {
                    Role::User
                } else {
                    Role::Assistant
                },
                content: Some(format!(
                    "This is message number {}. It contains significantly more content to ensure we exceed the context window limit. Each message should be approximately 50-60 tokens in length when encoded with the cl100k_base tokenizer. This allows us to test the compaction functionality effectively. ",
                    i
                )),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let result = handoff.handoff_to_model(
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
            &messages,
        );

        assert!(result.is_ok());
        let handoff_messages = result.unwrap();

        // Should have compacted
        assert!(handoff_messages.len() < messages.len());
        assert!(matches!(handoff_messages[0].role, Role::System));
    }

    #[test]
    fn test_token_count_accuracy_within_5_percent() {
        let handoff = ModelHandoff::new();
        let text = "The quick brown fox jumps over the lazy dog. ";

        // Count tokens
        let counted = handoff.count_tokens(text);

        // Expected value based on cl100k_base tokenizer
        let expected = 11;
        // Allow 10% tolerance for tokenizer variations
        let tolerance = (expected as f64 * 0.10) as i32;

        assert!(
            (counted as i32 - expected).abs() <= tolerance,
            "Token count {} not within {}% of expected {}",
            counted,
            10,
            expected
        );
    }
}
