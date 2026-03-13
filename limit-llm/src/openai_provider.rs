use crate::error::LlmError;
use crate::providers::{LlmProvider, ProviderResponseChunk};
use crate::types::{Message, Tool, Usage};
use async_stream::stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::pin::Pin;
use std::time::Duration;
use tracing::{debug, error, info, instrument, trace};

#[derive(Clone)]
pub struct OpenAiProvider {
    api_key: String,
    client: Client,
    base_url: String,
    model: String,
    max_tokens: u32,
}

impl OpenAiProvider {
    pub fn new(
        api_key: String,
        base_url: Option<&str>,
        model: &str,
        max_tokens: u32,
        timeout: u64,
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
                .unwrap_or("https://api.openai.com/v1/chat/completions")
                .to_string(),
            model: model.to_string(),
            max_tokens,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    #[instrument(skip(self, messages, tools))]
    #[allow(clippy::type_complexity)]
    async fn send(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + '_>>,
        LlmError,
    > {
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let model = self.model.clone();
        let max_tokens = self.max_tokens;
        let client_clone = self.client.clone();

        Ok(Box::pin(stream! {
            info!("OpenAI API request: model={}, max_tokens={}", self.model, self.max_tokens);

            let request_body = match build_request_body(&messages, &tools, &model, max_tokens, None) {
                Ok(body) => body,
                Err(e) => {
                    error!("OpenAI API error: {}", e);
                    yield Err(e);
                    return;
                }
            };

            match do_request(&client_clone, &api_key, &base_url, &request_body).await {
                Ok(mut stream) => {
                    while let Some(chunk) = stream.next().await {
                        yield chunk;
                    }
                }
                Err(e) => {
                    error!("OpenAI API error: {}", e);
                    yield Err(e);
                }
            }
        }))
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn clone_box(&self) -> Box<dyn LlmProvider> {
        Box::new(self.clone())
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
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(request_body)
        .send()
        .await
        .map_err(|e| LlmError::NetworkError(e.to_string()))?;

    let status = response.status();
    debug!("OpenAI API response received: status={}", status.as_u16());

    if status.is_client_error() || status.is_server_error() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        if status.as_u16() == 429 {
            error!("OpenAI API error: Rate limited");
            return Err(LlmError::ApiError(format!("Rate limited: {}", error_text)));
        }
        error!("OpenAI API error: HTTP {}: {}", status, error_text);
        return Err(LlmError::ApiError(format!(
            "HTTP {}: {}",
            status, error_text
        )));
    }

    let byte_stream = response.bytes_stream();
    let stream = parse_openai_sse_stream(byte_stream);
    Ok(stream)
}

fn build_request_body(
    messages: &[Message],
    tools: &[Tool],
    model: &str,
    max_tokens: u32,
    extra_body: Option<serde_json::Map<String, serde_json::Value>>,
) -> Result<Value, LlmError> {
    let mut request = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "max_tokens": max_tokens
    });

    if !tools.is_empty() {
        request["tools"] = serde_json::to_value(tools)
            .map_err(|e| LlmError::ApiError(format!("Failed to serialize tools: {}", e)))?;
    }

    // Add any extra body parameters
    if let Some(extra) = extra_body {
        for (key, value) in extra {
            request[key] = value;
        }
    }

    Ok(request)
}

#[derive(Debug)]
struct SseEvent {
    data: String,
}

fn parse_openai_sse_stream(
    byte_stream: impl Stream<Item = reqwest::Result<bytes::Bytes>> + Send + Unpin + 'static,
) -> Pin<Box<dyn Stream<Item = Result<ProviderResponseChunk, LlmError>> + Send + 'static>> {
    Box::pin(stream! {
        let mut buffer = String::new();
        let mut tool_calls_by_id: std::collections::HashMap<u32, (String, String, String)> = std::collections::HashMap::new();

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
                    // Stream complete
                    return;
                }

                if let Ok(parsed) = serde_json::from_str::<Value>(&event.data) {
                    trace!("OpenAI SSE: {}", &event.data.chars().take(200).collect::<String>());

                    // OpenAI format: {"choices":[{"delta":{"content":"..."}}]}
                    if let Some(choices) = parsed.get("choices").and_then(|v| v.as_array()) {
                        if let Some(first_choice) = choices.first() {
                            if let Some(delta) = first_choice.get("delta") {
                                // Handle content deltas
                                if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                    yield Ok(ProviderResponseChunk::ContentDelta(content.to_string()));
                                }

                                // Handle tool calls
                                if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                                    for tool_call in tool_calls {
                                        if let Some(index) = tool_call.get("index").and_then(|v| v.as_u64()) {
                                            let index = index as u32;

                                            // Get tool call ID
                                            let id_from_json = tool_call.get("id").and_then(|v| v.as_str());
                                            let id = if let Some(id_str) = id_from_json {
                                                id_str.to_string()
                                            } else {
                                                tool_calls_by_id.get(&index).map(|t| &t.0).map_or(String::new(), |v| v.to_string())
                                            };

                                            // Get tool function name
                                            let name = if let Some(function) = tool_call.get("function") {
                                                let name_from_json = function.get("name").and_then(|v| v.as_str());
                                                if let Some(name_str) = name_from_json {
                                                    name_str.to_string()
                                                } else {
                                                    tool_calls_by_id.get(&index).map(|t| &t.1).map_or(String::new(), |v| v.to_string())
                                                }
                                            } else {
                                                tool_calls_by_id.get(&index).map(|t| &t.1).map_or(String::new(), |v| v.to_string())
                                            };

                                            // Get tool arguments (may be partial)
                                            let args = if let Some(function) = tool_call.get("function") {
                                                let args_from_json = function.get("arguments").and_then(|v| v.as_str());
                                                if let Some(args_str) = args_from_json {
                                                    args_str.to_string()
                                                } else {
                                                    tool_calls_by_id.get(&index).map(|t| &t.2).map_or(String::new(), |v| v.to_string())
                                                }
                                            } else {
                                                tool_calls_by_id.get(&index).map(|t| &t.2).map_or(String::new(), |v| v.to_string())
                                            };

                                            // Store tool call metadata
                                            if !id.is_empty() || !name.is_empty() {
                                                tool_calls_by_id.insert(index, (id.clone(), name.clone(), args.clone()));
                                            }

                                            // Try to parse accumulated arguments as JSON
                                            let args_json = parse_partial_json(&args);

                                            // Only yield if we have at least a name
                                            if !name.is_empty() {
                                                yield Ok(ProviderResponseChunk::ToolCallDelta {
                                                    id,
                                                    name,
                                                    arguments: args_json,
                                                });
                                            }
                                        }
                                    }
                                }
                            }

                            // Handle finish reason and usage
                            if let Some(finish_reason) = first_choice.get("finish_reason").and_then(|v| v.as_str()) {
                                debug!("OpenAI finish_reason: {}", finish_reason);
                                if finish_reason == "stop" || finish_reason == "tool_calls" {
                                    if let Some(usage) = parsed.get("usage") {
                                        // OpenAI uses prompt_tokens and completion_tokens,
                                        // but our Usage struct uses input_tokens and output_tokens
                                        let input_tokens = usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                                        let output_tokens = usage.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0);

                                        yield Ok(ProviderResponseChunk::Done(Usage {
                                            input_tokens,
                                            output_tokens,
                                        }));
                                        return;
                                    } else {
                                        // No usage info, but stream is done
                                        yield Ok(ProviderResponseChunk::Done(Usage {
                                            input_tokens: 0,
                                            output_tokens: 0,
                                        }));
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
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
    async fn test_openai_streaming() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_chunked_body(|w| {
                w.write_all(b"data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n")?;
                w.write_all(b"data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n")?;
                w.write_all(b"data: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n")?;
                w.write_all(b"data: [DONE]\n\n")?;
                Ok::<(), std::io::Error>(())
            })
            .create_async()
            .await;

        let client = OpenAiProvider::new("test-key".to_string(), None, "gpt-4", 4096, 60);
        let messages = vec![Message {
            role: crate::types::Role::User,
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        let base_url = format!("{}/v1/chat/completions", server.url());
        let client_with_url = OpenAiProvider {
            api_key: "test-key".to_string(),
            client: client.client,
            base_url,
            model: "gpt-4".to_string(),
            max_tokens: 4096,
        };

        let stream = client_with_url.send(messages, vec![]).await.unwrap();
        let chunks: Vec<_> = stream.collect().await;
        assert!(chunks.len() >= 3);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_openai_tool_call_streaming() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_chunked_body(|w| {
                w.write_all(b"data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_123\",\"type\":\"function\",\"function\":{\"name\":\"test_tool\",\"arguments\":\"{\\\"arg\\\":\\\"value\\\"}\"}}]}}]}\n\n")?;
                w.write_all(b"data: {\"choices\":[{\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":15,\"completion_tokens\":20}}\n\n")?;
                w.write_all(b"data: [DONE]\n\n")?;
                Ok::<(), std::io::Error>(())
            })
            .create_async()
            .await;

        let client = OpenAiProvider::new("test-key".to_string(), None, "gpt-4", 4096, 60);
        let messages = vec![Message {
            role: crate::types::Role::User,
            content: Some("Use test_tool".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        let tools = vec![Tool {
            tool_type: "function".to_string(),
            function: crate::types::ToolFunction {
                name: "test_tool".to_string(),
                description: "A test tool".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
        }];

        let base_url = format!("{}/v1/chat/completions", server.url());
        let client_with_url = OpenAiProvider {
            api_key: "test-key".to_string(),
            client: client.client,
            base_url,
            model: "gpt-4".to_string(),
            max_tokens: 4096,
        };

        let stream = client_with_url.send(messages, tools).await.unwrap();
        let chunks: Vec<_> = stream.collect().await;
        assert!(!chunks.is_empty());

        mock.assert_async().await;
    }

    #[test]
    fn test_parse_sse_line() {
        let mut buffer =
            String::from("data: {\"choices\":[{\"delta\":{\"content\":\"test\"}}]}\n\nother data");
        let event = parse_sse_line(&mut buffer);
        assert!(event.is_some());
        assert_eq!(
            event.unwrap().data,
            "{\"choices\":[{\"delta\":{\"content\":\"test\"}}]}"
        );
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
    fn test_parse_partial_json() {
        let json = r#"{"arg":"value"}"#;
        let parsed = parse_partial_json(json);
        assert!(parsed.is_object());
        assert_eq!(parsed.get("arg").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn test_parse_partial_json_empty() {
        let parsed = parse_partial_json("");
        assert!(parsed.is_object());
        assert_eq!(parsed.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_parse_partial_json_invalid() {
        let parsed = parse_partial_json("{invalid json");
        assert!(parsed.is_object());
        assert_eq!(parsed.as_object().unwrap().len(), 0);
    }
}
