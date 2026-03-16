//! Mock LLM provider for testing.
//!
//! Returns pre-configured responses, cycling through them on each call.
//! Useful for unit and integration tests without real API calls.

use async_trait::async_trait;
use futures::stream;
use std::sync::{Arc, Mutex};

use crate::error::LlmError;
use crate::providers::{LlmProvider, ProviderResponseChunk};
use crate::types::{Message, Tool, Usage};

/// A mock provider that returns canned responses sequentially.
///
/// # Example
///
/// ```ignore
/// use limit_llm::MockLlmProvider;
/// use limit_llm::LlmProvider;
/// use futures::StreamExt;
/// use limit_llm::ProviderResponseChunk;
///
/// # #[tokio::main]
/// # async fn main() {
/// let mock = MockLlmProvider::new()
///     .with_response("Hello from mock!")
///     .with_response("Second response");
///
/// assert_eq!(mock.provider_name(), "mock");
/// assert_eq!(mock.model_name(), "mock-model");
///
/// let mut stream = mock.send(vec![], vec![]).await.unwrap();
/// let mut text = String::new();
/// while let Some(chunk) = stream.next().await {
///     if let Ok(ProviderResponseChunk::ContentDelta(t)) = chunk {
///         text.push_str(&t);
///     }
/// }
/// assert_eq!(text, "Hello from mock!");
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct MockLlmProvider {
    responses: Arc<Mutex<Vec<String>>>,
    index: Arc<Mutex<usize>>,
    name: String,
    model: String,
}

impl MockLlmProvider {
    /// Create a new mock provider with no responses (returns empty stream).
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            index: Arc::new(Mutex::new(0)),
            name: "mock".to_string(),
            model: "mock-model".to_string(),
        }
    }

    /// Add a response that will be returned on the next `send()` call.
    /// Responses cycle: when exhausted, the first response is reused.
    pub fn with_response(self, response: impl Into<String>) -> Self {
        self.responses.lock().unwrap().push(response.into());
        self
    }

    /// Add multiple responses at once.
    pub fn with_responses(self, responses: Vec<String>) -> Self {
        self.responses.lock().unwrap().extend(responses);
        self
    }

    /// Override the provider name (default: `"mock"`).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Override the model name (default: `"mock-model"`).
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// How many times `send()` has been called.
    pub fn call_count(&self) -> usize {
        *self.index.lock().unwrap()
    }

    /// Reset the call counter.
    pub fn reset(&self) {
        *self.index.lock().unwrap() = 0;
    }
}

impl Default for MockLlmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn send(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<Tool>,
    ) -> Result<
        std::pin::Pin<
            Box<dyn stream::Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + '_>,
        >,
        LlmError,
    > {
        let responses = self.responses.lock().unwrap();
        let mut idx = self.index.lock().unwrap();

        let text = if responses.is_empty() {
            String::new()
        } else {
            let i = *idx % responses.len();
            *idx += 1;
            responses[i].clone()
        };

        let output_tokens = text.len() as u64;

        let chunks: Vec<Result<ProviderResponseChunk, LlmError>> = vec![
            Ok(ProviderResponseChunk::ContentDelta(text)),
            Ok(ProviderResponseChunk::Done(Usage {
                input_tokens: 10,
                output_tokens,
            })),
        ];

        Ok(Box::pin(stream::iter(chunks)))
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn clone_box(&self) -> Box<dyn LlmProvider> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_mock_provider_single_response() {
        let mock = MockLlmProvider::new().with_response("hello");
        let mut stream = mock.send(vec![], vec![]).await.unwrap();

        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            if let Ok(ProviderResponseChunk::ContentDelta(t)) = chunk {
                text.push_str(&t);
            }
        }
        assert_eq!(text, "hello");
    }

    #[tokio::test]
    async fn test_mock_provider_cycles() {
        let mock = MockLlmProvider::new()
            .with_response("first")
            .with_response("second");

        for expected in &["first", "second", "first", "second"] {
            let mut stream = mock.send(vec![], vec![]).await.unwrap();
            let mut text = String::new();
            while let Some(chunk) = stream.next().await {
                if let Ok(ProviderResponseChunk::ContentDelta(t)) = chunk {
                    text.push_str(&t);
                }
            }
            assert_eq!(text, *expected);
        }
    }

    #[tokio::test]
    async fn test_mock_provider_empty() {
        let mock = MockLlmProvider::new();
        let mut stream = mock.send(vec![], vec![]).await.unwrap();

        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            if let Ok(ProviderResponseChunk::ContentDelta(t)) = chunk {
                text.push_str(&t);
            }
        }
        assert!(text.is_empty());
    }

    #[test]
    fn test_mock_provider_metadata() {
        let mock = MockLlmProvider::new()
            .with_name("test-provider")
            .with_model("gpt-test");
        assert_eq!(mock.provider_name(), "test-provider");
        assert_eq!(mock.model_name(), "gpt-test");
    }

    #[test]
    fn test_mock_provider_call_count() {
        let mock = MockLlmProvider::new().with_response("hi");
        assert_eq!(mock.call_count(), 0);
        mock.reset();
        assert_eq!(mock.call_count(), 0);
    }

    #[test]
    fn test_mock_provider_clone() {
        let mock = MockLlmProvider::new().with_response("clone test");
        let cloned = mock.clone_box();
        assert_eq!(cloned.provider_name(), "mock");
    }
}
