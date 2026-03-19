//! Token counting utilities

/// Estimate token count for text
/// Uses a simple heuristic: ~4 characters per token
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Count tokens using cl100k_base encoding (Claude's tokenizer)
/// This is a simplified implementation. In production, use tiktoken.
pub fn count_tokens(text: &str) -> usize {
    // Simple approximation: words + punctuation
    text.split_whitespace().count() +
    text.chars().filter(|c| c.is_ascii_punctuation()).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_estimate_tokens() {
        let text = "def hello_world():\n    print('Hello, world!')";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
    }
}
