use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Web search tool using Exa AI MCP endpoint
pub struct WebSearchTool {
    client: Client,
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    const EXA_MCP_URL: &'static str = "https://mcp.exa.ai/mcp";
    const DEFAULT_NUM_RESULTS: u32 = 8;
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct McpRequest {
    jsonrpc: &'static str,
    id: u32,
    method: &'static str,
    params: McpParams,
}

#[derive(Serialize)]
struct McpParams {
    name: &'static str,
    arguments: McpArguments,
}

#[derive(Serialize)]
struct McpArguments {
    query: String,
    #[serde(rename = "numResults")]
    num_results: u32,
    #[serde(rename = "type")]
    search_type: &'static str,
}

#[derive(Deserialize)]
struct McpResponse {
    result: Option<McpResult>,
    error: Option<McpError>,
}

#[derive(Deserialize)]
struct McpResult {
    content: Vec<McpContent>,
}

#[derive(Deserialize)]
struct McpContent {
    text: String,
}

#[derive(Deserialize)]
struct McpError {
    message: String,
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::ToolError("Missing 'query' argument".to_string()))?;

        let num_results = args
            .get("numResults")
            .and_then(|v| v.as_u64())
            .unwrap_or(Self::DEFAULT_NUM_RESULTS as u64) as u32;

        let request = McpRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "tools/call",
            params: McpParams {
                name: "web_search_exa",
                arguments: McpArguments {
                    query: query.to_string(),
                    num_results,
                    search_type: "auto",
                },
            },
        };

        let response = self
            .client
            .post(Self::EXA_MCP_URL)
            .header("Accept", "application/json, text/event-stream")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AgentError::ToolError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AgentError::ToolError(format!(
                "Search failed ({}): {}",
                status, body
            )));
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| AgentError::ToolError(format!("Failed to read response: {}", e)))?;

        // Parse SSE response format: "data: {...}"
        let result_text = parse_sse_response(&response_text)?;

        Ok(serde_json::json!({
            "query": query,
            "results": result_text
        }))
    }
}

/// Parse SSE response format from Exa MCP
fn parse_sse_response(text: &str) -> Result<String, AgentError> {
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            let response: McpResponse = serde_json::from_str(data)
                .map_err(|e| AgentError::ToolError(format!("Failed to parse response: {}", e)))?;

            if let Some(error) = response.error {
                return Err(AgentError::ToolError(format!(
                    "Search error: {}",
                    error.message
                )));
            }

            if let Some(result) = response.result {
                if let Some(content) = result.content.first() {
                    return Ok(content.text.clone());
                }
            }
        }
    }

    Ok("No search results found. Please try a different query.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_tool_name() {
        let tool = WebSearchTool::new();
        assert_eq!(tool.name(), "web_search");
    }

    #[test]
    fn test_web_search_tool_default() {
        let tool = WebSearchTool::new();
        assert_eq!(tool.name(), "web_search");
    }

    #[tokio::test]
    async fn test_web_search_missing_query() {
        let tool = WebSearchTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'query'"));
    }

    #[test]
    fn test_parse_sse_response() {
        let sse_response = r#"event: message
data: {"result":{"content":[{"type":"text","text":"Title: Test Result\nURL: https://example.com\nText: Sample content"}]},"jsonrpc":"2.0","id":1}"#;

        let result = parse_sse_response(sse_response).unwrap();
        assert!(result.contains("Test Result"));
    }

    #[test]
    fn test_parse_sse_response_error() {
        let sse_response =
            r#"data: {"error":{"message":"Rate limit exceeded"},"jsonrpc":"2.0","id":1}"#;

        let result = parse_sse_response(sse_response);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Rate limit"));
    }
}
