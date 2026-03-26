use crate::error::LlmError;
use crate::providers::{LlmProvider, ProviderResponseChunk};

use crate::types::{Message, Tool, Usage};
use async_stream::stream;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::boxed::Box;
use std::pin::Pin;
use std::time::Duration;
use tracing::{debug, error, info, instrument, trace, warn};

pub struct AnthropicClient {
    api_key: String,
    client: Client,
    base_url: String,
    model: String,
    max_tokens: u32,
}

#[derive(Debug)]
struct SseEvent {
    data: String,
}

impl Clone for AnthropicClient {
    fn clone(&self) -> Self {
        Self {
            api_key: self.api_key.clone(),
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            max_tokens: self.max_tokens,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicClient {
    async fn send(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + '_>>,
        LlmError,
    > {
        Ok(self.send(messages, tools).await)
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn clone_box(&self) -> Box<dyn LlmProvider> {
        Box::new(self.clone())
    }
}

impl AnthropicClient {
    pub fn new(
        api_key: String,
        base_url: Option<&str>,
        timeout: u64,
        model: &str,
        max_tokens: u32,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            api_key,
            client,
            base_url: base_url
                .unwrap_or("https://api.anthropic.com/v1/messages")
                .to_string(),
            model: model.to_string(),
            max_tokens,
        }
    }

    #[instrument(skip(self, messages, tools))]
    pub async fn send(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + '_>> {
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let model = self.model.clone();
        let max_tokens = self.max_tokens;
        let messages_cloned = messages.clone();
        let tools_cloned = tools.clone();
        let client_clone = self.client.clone();

        Box::pin(stream! {
            info!("API request: model={}, max_tokens={}", self.model, self.max_tokens);

            let request_body = match build_request_body(&messages_cloned, &tools_cloned, &model, max_tokens) {
                Ok(body) => body,
                Err(e) => {
                    error!("API error: {}", e);
                    yield Err(e);
                    return;
                }
            };

            for attempt in 0..3 {
                let delay = Duration::from_secs(2_u64.pow(attempt));

                match do_request(&client_clone, &api_key, &base_url, &request_body).await {
                    Ok(mut stream) => {
                        while let Some(chunk) = stream.next().await {
                            yield chunk;
                        }
                        return;
                    }
                    Err(e) => {
                        if attempt == 2 {
                            error!("API error: {}", e);
                            yield Err(e);
                            return;
                        }
                        warn!("API retry: attempt={}, delay_ms={}", attempt, delay.as_millis());
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        })
    }
}

#[instrument(skip_all)]
#[allow(clippy::type_complexity)]
async fn do_request(
    client: &Client,
    api_key: &str,
    base_url: &str,
    request_body: &Value,
) -> Result<
    Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + 'static>>,
    LlmError,
> {
    let response = client
        .post(base_url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(request_body)
        .send()
        .await
        .map_err(|e| LlmError::NetworkError(e.to_string()))?;

    let status = response.status();
    debug!("API response received: status={}", status.as_u16());

    if status.is_client_error() || status.is_server_error() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        if status.as_u16() == 429 {
            error!("API error: Rate limited");
            return Err(LlmError::ApiError(format!("Rate limited: {}", error_text)));
        }
        error!("API error: HTTP {}: {}", status, error_text);
        return Err(LlmError::ApiError(format!(
            "HTTP {}: {}",
            status, error_text
        )));
    }

    let byte_stream = response.bytes_stream();
    let stream = parse_sse_stream(byte_stream);
    Ok(stream)
}

fn build_request_body(
    messages: &[Message],
    tools: &[Tool],
    model: &str,
    max_tokens: u32,
) -> Result<Value, LlmError> {
    let cache_count = messages
        .iter()
        .filter(|m| m.cache_control.is_some())
        .count();
    if cache_count > 0 {
        debug!(
            "Anthropic request has {} messages with cache_control",
            cache_count
        );
        for m in messages.iter().filter(|m| m.cache_control.is_some()) {
            if let Some(cc) = &m.cache_control {
                debug!(
                    "  - role={:?}, type={}, ttl={:?}",
                    m.role, cc.cache_type, cc.ttl
                );
            }
        }
    }

    let mut request = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": messages,
        "stream": true
    });

    if !tools.is_empty() {
        request["tools"] = serde_json::to_value(tools)
            .map_err(|e| LlmError::ApiError(format!("Failed to serialize tools: {}", e)))?;
    }

    Ok(request)
}
/// Parse potentially incomplete JSON during streaming.
/// Returns empty object if parsing fails.
fn parse_partial_json(json: &str) -> serde_json::Value {
    if json.trim().is_empty() {
        return serde_json::json!({});
    }

    // Try standard parsing first
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json) {
        return value;
    }

    // If parsing fails, return empty object
    // (In future, could use partial-json crate for better handling)
    serde_json::json!({})
}

fn parse_sse_stream(
    byte_stream: impl Stream<Item = reqwest::Result<bytes::Bytes>> + Send + Unpin + 'static,
) -> Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + 'static>> {
    Box::pin(stream! {
            let mut buffer = String::new();
            let mut tool_calls_by_id: std::collections::HashMap<u64, (String, String)> = std::collections::HashMap::new();

            let mut tool_partial_json: std::collections::HashMap<u64, String> = std::collections::HashMap::new();

            let mut lines = byte_stream
                .map(|chunk| chunk.map_err(|e| LlmError::NetworkError(e.to_string())));

            while let Some(chunk_result) = lines.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(e);
                        continue;
                    }
                };

                let text = String::from_utf8_lossy(&chunk);

                buffer.push_str(&text);

                while let Some(event) = parse_sse_line(&mut buffer) {

                    if event.data == "[DONE]" {
                        return;
                    }

                    if let Ok(parsed) = serde_json::from_str::<Value>(&event.data) {
                        trace!("SSE: {}", &event.data.chars().take(200).collect::<String>());
                        let chunk_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");

                        match chunk_type {
                            "content_block_delta" => {
                                if let Some(delta) = parsed.get("delta") {
                                    // Handle text deltas
                                    if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                        yield Ok(ProviderResponseChunk::ContentDelta(text.to_string()));
                                    }

                                    // Handle tool argument deltas (input_json_delta)
                                    let delta_type = delta.get("type").and_then(|v| v.as_str());
                                    if delta_type == Some("input_json_delta") {
                                        if let Some(partial_json) = delta.get("partial_json").and_then(|v| v.as_str()) {
                                            // Get tool call index
                                            if let Some(index) = parsed.get("index").and_then(|v| v.as_u64()) {
                                                // Accumulate partial JSON
    tool_partial_json.entry(index)
                                                    .or_default()
                                                    .push_str(partial_json);

                                                // Look up tool call metadata and parse accumulated JSON
                                                if let Some((id, name)) = tool_calls_by_id.get(&index) {
                                                    let accumulated = tool_partial_json.get(&index).unwrap();
                                                    let args = parse_partial_json(accumulated);

                                                    yield Ok(ProviderResponseChunk::ToolCallDelta {
                                                        id: id.clone(),
                                                        name: name.clone(),
                                                        arguments: args,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "content_block_start" => {
                                if let Some(content_block) = parsed.get("content_block") {
                                    let block_type = content_block.get("type").and_then(|v| v.as_str());
                                    if block_type == Some("tool_use") {
                                        let id = content_block.get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let name = content_block.get("name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();

                                        // Track tool call by index
                                        if let Some(index) = parsed.get("index").and_then(|v| v.as_u64()) {
                                            tool_calls_by_id.insert(index, (id.clone(), name.clone()));
                                        }

                                        yield Ok(ProviderResponseChunk::ToolCallDelta {
                                            id,
                                            name,
                                            arguments: serde_json::json!({}),
                                        });
                                    }
                                }
                            }
                            "content_block_stop" => {
                                // Tool call completed
                            }
                            "message_delta" => {
                                if let Some(delta) = parsed.get("delta") {
                                    if let Some(stop_reason) = delta.get("stop_reason").and_then(|v| v.as_str()) {
                                        debug!("stop_reason: {}", stop_reason);
                                        if stop_reason == "end_turn" || stop_reason == "tool_use" {
                                            if let Some(usage) = parsed.get("usage") {
                                                if let Ok(usage_obj) = serde_json::from_value::<Usage>(usage.clone()) {
                                                    if usage_obj.cache_read_tokens > 0 || usage_obj.cache_write_tokens > 0 {
                                                        debug!(
                                                            "Anthropic cache tokens: read={}, write={}",
                                                            usage_obj.cache_read_tokens, usage_obj.cache_write_tokens
                                                        );
                                                    }
                                                    yield Ok(ProviderResponseChunk::Done(usage_obj));
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                debug!("Unknown chunk_type: {}", chunk_type);
                            }
                        }
                    }
                }
            }
        })
}

fn parse_sse_line(buffer: &mut String) -> Option<SseEvent> {
    loop {
        let newline_pos = buffer.find('\n')?;
        let line = buffer[..newline_pos].trim().to_string();
        *buffer = buffer[newline_pos + 1..].to_string();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(':') {
            continue;
        }

        // Skip event: lines (we only care about data)
        if line.starts_with("event:") {
            continue;
        }

        // Parse data: lines
        if let Some(data_pos) = line.find("data: ") {
            let data = line[data_pos + 6..].trim();
            return Some(SseEvent {
                data: data.to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_streaming() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_chunked_body(|w| {
                w.write_all(b"data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello\"}}\n\n")?;
                w.write_all(b"data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\" world\"}}\n\n")?;
                w.write_all(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"input_tokens\":10,\"output_tokens\":5}}\n\n")?;
                Ok::<(), std::io::Error>(())
            })
            .create_async()
            .await;

        let client = AnthropicClient::new(
            "test-key".to_string(),
            None,
            300,
            "claude-3-5-sonnet-20241022",
            4096,
        );
        let messages = vec![Message {
            role: crate::types::Role::User,
            content: Some(crate::MessageContent::text("Hello")),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        let base_url = format!("{}/v1/messages", server.url());
        let client_with_url = AnthropicClient {
            api_key: "test-key".to_string(),
            client: client.client,
            base_url,
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
        };

        let stream = client_with_url.send(messages, vec![]).await;
        let chunks: Vec<_> = stream.collect().await;
        assert!(chunks.len() >= 3);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_retry_on_429() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(429)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":{"type":"rate_limit_error","message":"Rate limited"}}"#)
            .expect(2)
            .create_async()
            .await;

        let success_mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_chunked_body(|w| {
                w.write_all(b"data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello\"}}\n\n")?;
                w.write_all(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"input_tokens\":10,\"output_tokens\":5}}\n\n")?;
                Ok::<(), std::io::Error>(())
            })
            .expect(1)
            .create_async()
            .await;

        let client = AnthropicClient::new(
            "test-key".to_string(),
            None,
            300,
            "claude-3-5-sonnet-20241022",
            4096,
        );
        let messages = vec![Message {
            role: crate::types::Role::User,
            content: Some(crate::MessageContent::text("Hello")),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        let base_url = format!("{}/v1/messages", server.url());
        let client_with_url = AnthropicClient {
            api_key: "test-key".to_string(),
            client: client.client,
            base_url,
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
        };

        let stream = client_with_url.send(messages, vec![]).await;
        let chunks: Vec<_> = stream.collect().await;
        assert!(!chunks.is_empty());

        mock.assert_async().await;
        success_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_timeout() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_chunked_body(|w| {
                // Sleep to simulate slow response
                std::thread::sleep(std::time::Duration::from_millis(500));
                w.write_all(
                    b"data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello\"}}\n\n",
                )?;
                Ok::<(), std::io::Error>(())
            })
            .create_async()
            .await;

        let client = AnthropicClient::new(
            "test-key".to_string(),
            None,
            300,
            "claude-3-5-sonnet-20241022",
            4096,
        );
        let messages = vec![Message {
            role: crate::types::Role::User,
            content: Some(crate::MessageContent::text("Hello")),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        let base_url = format!("{}/v1/messages", server.url());
        let client_with_url = AnthropicClient {
            api_key: "test-key".to_string(),
            client: client.client,
            base_url,
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
        };

        // The test should pass since timeout is 300s
        let stream = client_with_url.send(messages, vec![]).await;
        let chunks: Vec<_> = stream.collect().await;
        assert!(!chunks.is_empty());
    }

    #[tokio::test]
    async fn test_tool_call_streaming() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_chunked_body(|w| {
                w.write_all(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_123\",\"name\":\"test_tool\"}}\n\n")?;
                w.write_all(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"partial_json\":\"{\\\"arg\\\":\\\"value\\\"}\"}}\n\n")?;
                w.write_all(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"input_tokens\":15,\"output_tokens\":20}}\n\n")?;
                Ok::<(), std::io::Error>(())
            })
            .create_async()
            .await;

        let client = AnthropicClient::new(
            "test-key".to_string(),
            None,
            300,
            "claude-3-5-sonnet-20241022",
            4096,
        );
        let messages = vec![Message {
            role: crate::types::Role::User,
            content: Some(crate::MessageContent::text("Use test_tool")),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];

        let tools = vec![Tool {
            tool_type: "function".to_string(),
            function: crate::types::ToolFunction {
                name: "test_tool".to_string(),
                description: "A test tool".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
        }];

        let base_url = format!("{}/v1/messages", server.url());
        let client_with_url = AnthropicClient {
            api_key: "test-key".to_string(),
            client: client.client,
            base_url,
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
        };

        let stream = client_with_url.send(messages, tools).await;
        let chunks: Vec<_> = stream.collect().await;
        assert!(!chunks.is_empty());

        mock.assert_async().await;
    }

    #[test]
    fn test_parse_sse_line() {
        let mut buffer = String::from("data: {\"type\":\"test\"}\n\nother data");
        let event = parse_sse_line(&mut buffer);
        assert!(event.is_some());
        assert_eq!(event.unwrap().data, "{\"type\":\"test\"}");
        assert_eq!(buffer, "\nother data");
    }

    #[test]
    fn test_parse_sse_line_empty() {
        let mut buffer = String::from("\n\ndata: test");
        let event = parse_sse_line(&mut buffer);
        assert!(event.is_none());
        assert_eq!(buffer, "data: test");
    }

    #[test]
    fn test_parse_sse_line_comment() {
        let mut buffer = String::from(": comment\n\ndata: test");
        let event = parse_sse_line(&mut buffer);
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_sse_line_zai_format() {
        let mut buffer = String::from("event: content_block_start\ndata: {\"type\":\"test\"}\n\n");
        let event = parse_sse_line(&mut buffer);
        assert!(event.is_some());
        assert_eq!(event.unwrap().data, "{\"type\":\"test\"}");
    }
}
