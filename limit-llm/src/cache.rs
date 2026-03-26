//! Helper functions for applying cache control to messages.

use crate::config::CacheSettings;
use crate::types::{CacheControl, Message, Role};

/// Apply cache control to strategic message positions.
///
/// Strategy (Anthropic/OpenAI best practices):
/// 1. System message (if present) - first message if role is System
/// 2. Last user message - enables caching of conversation context
///
/// Returns a new Vec with cache_control applied where appropriate.
pub fn apply_cache_control(messages: &[Message], settings: &CacheSettings) -> Vec<Message> {
    if !settings.is_enabled() {
        return messages.to_vec();
    }

    let cache_control = if settings.is_long_retention() {
        CacheControl::ephemeral_long()
    } else {
        CacheControl::ephemeral()
    };

    let mut result = messages.to_vec();
    let len = result.len();

    // Apply to system message (first message if role is System)
    if len > 0 && result[0].role == Role::System {
        result[0].cache_control = Some(cache_control.clone());
    }

    // Apply to last user message
    for i in (0..len).rev() {
        if result[i].role == Role::User {
            result[i].cache_control = Some(cache_control.clone());
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_caching_when_disabled() {
        let settings = CacheSettings {
            retention: "none".to_string(),
        };
        let messages = vec![Message {
            role: Role::User,
            content: Some(crate::MessageContent::text("Hello")),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        let result = apply_cache_control(&messages, &settings);
        assert!(result[0].cache_control.is_none());
    }

    #[test]
    fn test_cache_applied_to_user_message() {
        let settings = CacheSettings {
            retention: "short".to_string(),
        };
        let messages = vec![Message {
            role: Role::User,
            content: Some(crate::MessageContent::text("Hello")),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        let result = apply_cache_control(&messages, &settings);
        assert!(result[0].cache_control.is_some());
    }

    #[test]
    fn test_cache_applied_to_system_and_last_user() {
        let settings = CacheSettings {
            retention: "short".to_string(),
        };
        let messages = vec![
            Message {
                role: Role::System,
                content: Some(crate::MessageContent::text("System prompt")),
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            },
            Message {
                role: Role::User,
                content: Some(crate::MessageContent::text("Hello")),
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            },
            Message {
                role: Role::Assistant,
                content: Some(crate::MessageContent::text("Hi!")),
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            },
            Message {
                role: Role::User,
                content: Some(crate::MessageContent::text("How are you?")),
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            },
        ];

        let result = apply_cache_control(&messages, &settings);

        // System message should have cache
        assert!(result[0].cache_control.is_some());
        // First user message should NOT have cache (only last)
        assert!(result[1].cache_control.is_none());
        // Assistant should NOT have cache
        assert!(result[2].cache_control.is_none());
        // Last user message should have cache
        assert!(result[3].cache_control.is_some());
    }

    #[test]
    fn test_long_retention_ttl() {
        let settings = CacheSettings {
            retention: "long".to_string(),
        };
        let messages = vec![Message {
            role: Role::User,
            content: Some(crate::MessageContent::text("Hello")),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        let result = apply_cache_control(&messages, &settings);
        let cc = result[0].cache_control.as_ref().unwrap();
        assert_eq!(cc.ttl, Some("1h".to_string()));
    }
}
