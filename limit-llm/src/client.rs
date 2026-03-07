use crate::error::LlmError;
use crate::types::{Message, Tool, Usage};
use async_stream::stream;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::pin::Pin;
use std::time::Duration;

pub struct AnthropicClient {
    api_key: String,
    client: Client,
    base_url: String,
}

pub enum ResponseChunk {
    ContentDelta(String),
    ToolCallDelta {
        id: String,
        name: String,
        arguments: Value,
    },
    Done(Usage),
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
        }
    }
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            api_key,
            client,
            base_url: "https://api.anthropic.com/v1/messages".to_string(),
        }
    }

    pub async fn send(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Pin<Box<dyn Stream<Item = Result<ResponseChunk, LlmError>> + Send + '_>> {
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let messages_cloned = messages.clone();
        let tools_cloned = tools.clone();
        let client_clone = self.client.clone();

        Box::pin(stream! {
            let request_body = match build_request_body(&messages_cloned, &tools_cloned) {
                Ok(body) => body,
                Err(e) => {
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
                            yield Err(e);
                            return;
                        }
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        })
    }
}

async fn do_request(
    client: &Client,
    api_key: &str,
    base_url: &str,
    request_body: &Value,
) -> Result<Pin<Box<dyn Stream<Item = Result<ResponseChunk, LlmError>> + Send + 'static>>, LlmError>
{
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
    if status.is_client_error() || status.is_server_error() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        if status.as_u16() == 429 {
            return Err(LlmError::ApiError(format!("Rate limited: {}", error_text)));
        }
        return Err(LlmError::ApiError(format!(
            "HTTP {}: {}",
            status, error_text
        )));
    }

    let byte_stream = response.bytes_stream();
    let stream = parse_sse_stream(byte_stream);
    Ok(stream)
}

fn build_request_body(messages: &[Message], tools: &[Tool]) -> Result<Value, LlmError> {
    let mut request = serde_json::json!({
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 4096,
        "messages": messages,
        "stream": true
    });

    if !tools.is_empty() {
        request["tools"] = serde_json::to_value(tools)
            .map_err(|e| LlmError::ApiError(format!("Failed to serialize tools: {}", e)))?;
    }

    Ok(request)
}

fn parse_sse_stream(
    byte_stream: impl Stream<Item = reqwest::Result<bytes::Bytes>> + Send + Unpin + 'static,
) -> Pin<Box<dyn Stream<Item = Result<ResponseChunk, LlmError>> + Send + 'static>> {
    Box::pin(stream! {
        let mut buffer = String::new();

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
                    let chunk_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    match chunk_type {
                        "content_block_delta" => {
                            if let Some(delta) = parsed.get("delta") {
                                if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                    yield Ok(ResponseChunk::ContentDelta(text.to_string()));
                                }
                                if let Some(partial_json) = delta.get("partial_json").and_then(|v| v.as_str()) {
                                    if let Ok(value) = serde_json::from_str::<Value>(partial_json) {
                                        yield Ok(ResponseChunk::ContentDelta(value.to_string()));
                                    }
                                }
                            }
                        }
                        "content_block_start" => {
                            if let Some(content_block) = parsed.get("content_block") {
                                if let Some(tool_use) = content_block.get("tool_use") {
                                    let id = tool_use.get("id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let name = tool_use.get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    yield Ok(ResponseChunk::ToolCallDelta {
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
                                    if stop_reason == "end_turn" || stop_reason == "tool_use" {
                                        if let Some(usage) = parsed.get("usage") {
                                            if let Ok(usage_obj) = serde_json::from_value::<Usage>(usage.clone()) {
                                                yield Ok(ResponseChunk::Done(usage_obj));
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    })
}

fn parse_sse_line(buffer: &mut String) -> Option<SseEvent> {
    let newline_pos = buffer.find('\n')?;
    let line = buffer[..newline_pos].trim().to_string();
    *buffer = buffer[newline_pos + 1..].to_string();

    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    if let Some(data_pos) = line.find("data: ") {
        let data = line[data_pos + 6..].trim();
        return Some(SseEvent {
            data: data.to_string(),
        });
    }

    None
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

        let client = AnthropicClient::new("test-key".to_string());
        let messages = vec![Message {
            role: crate::types::Role::User,
            content: "Hello".to_string(),
            tool_calls: None,
        }];

        let base_url = format!("{}/v1/messages", server.url());
        let client_with_url = AnthropicClient {
            api_key: "test-key".to_string(),
            client: client.client,
            base_url,
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

        let client = AnthropicClient::new("test-key".to_string());
        let messages = vec![Message {
            role: crate::types::Role::User,
            content: "Hello".to_string(),
            tool_calls: None,
        }];

        let base_url = format!("{}/v1/messages", server.url());
        let client_with_url = AnthropicClient {
            api_key: "test-key".to_string(),
            client: client.client,
            base_url,
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

        let client = AnthropicClient::new("test-key".to_string());
        let messages = vec![Message {
            role: crate::types::Role::User,
            content: "Hello".to_string(),
            tool_calls: None,
        }];

        let base_url = format!("{}/v1/messages", server.url());
        let client_with_url = AnthropicClient {
            api_key: "test-key".to_string(),
            client: client.client,
            base_url,
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

        let client = AnthropicClient::new("test-key".to_string());
        let messages = vec![Message {
            role: crate::types::Role::User,
            content: "Use test_tool".to_string(),
            tool_calls: None,
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
        assert_eq!(buffer, "\ndata: test");
    }

    #[test]
    fn test_parse_sse_line_comment() {
        let mut buffer = String::from(": comment\n\ndata: test");
        let event = parse_sse_line(&mut buffer);
        assert!(event.is_none());
    }
}
