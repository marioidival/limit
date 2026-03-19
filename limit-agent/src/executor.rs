use crate::error::AgentError;
use crate::registry::ToolRegistry;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tracing::instrument;

/// Represents a single tool call
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

impl ToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, args: Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            args,
        }
    }
}

/// Represents the result of a tool execution
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub call_id: String,
    pub output: Result<Value, AgentError>,
}

/// Executor for tool calls with parallel/conditional execution
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    max_concurrent: usize,
    timeout: Duration,
}

impl ToolExecutor {
    /// Create a new ToolExecutor with default settings
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
            max_concurrent: 5,
            timeout: Duration::from_secs(60),
        }
    }

    /// Set maximum concurrent tool executions
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Set timeout for each tool execution
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Execute multiple tool calls with conditional parallel/sequential logic
    #[instrument(skip(self, calls))]
    pub async fn execute_tools(&self, calls: Vec<ToolCall>) -> Vec<ToolResult> {
        if calls.is_empty() {
            return Vec::new();
        }

        // Analyze dependencies and categorize calls
        let (independent, dependent) = self.categorize_calls(&calls);

        // Execute independent calls in parallel (with concurrency limit)
        let mut results = self.execute_parallel(independent).await;

        // Execute dependent calls sequentially
        results.extend(self.execute_sequential(dependent).await);

        results
    }

    /// Categorize tool calls into independent and dependent groups
    fn categorize_calls(&self, calls: &[ToolCall]) -> (Vec<ToolCall>, Vec<ToolCall>) {
        let mut independent = Vec::new();
        let mut dependent = Vec::new();

        for call in calls {
            if self.has_dependencies(call) {
                dependent.push(call.clone());
            } else {
                independent.push(call.clone());
            }
        }

        (independent, dependent)
    }

    /// Check if a tool call has dependencies on previous calls
    fn has_dependencies(&self, call: &ToolCall) -> bool {
        // Simple heuristic: if args contain references to variable patterns like $var or $output_N
        let args_str = serde_json::to_string(&call.args).unwrap_or_default();

        // Look for dependency markers in arguments
        args_str.contains("$output_") || args_str.contains("$var_") || args_str.contains("$result_")
    }

    /// Execute independent tool calls in parallel with concurrency limit
    #[instrument(skip(self, calls))]
    async fn execute_parallel(&self, calls: Vec<ToolCall>) -> Vec<ToolResult> {
        if calls.is_empty() {
            return Vec::new();
        }

        use futures::stream::{self, StreamExt};

        stream::iter(calls)
            .map(|call| {
                let registry = Arc::clone(&self.registry);
                let timeout = self.timeout;
                async move {
                    let result = tokio::time::timeout(
                        timeout,
                        registry.execute(&call.name, call.args.clone()),
                    )
                    .await;

                    ToolResult {
                        call_id: call.id,
                        output: result.unwrap_or(Err(AgentError::ToolError(
                            "Tool execution timed out".to_string(),
                        ))),
                    }
                }
            })
            .buffer_unordered(self.max_concurrent)
            .collect()
            .await
    }

    /// Execute dependent tool calls sequentially
    #[instrument(skip(self, calls))]
    async fn execute_sequential(&self, calls: Vec<ToolCall>) -> Vec<ToolResult> {
        let mut results = Vec::new();

        for call in calls {
            let result = tokio::time::timeout(
                self.timeout,
                self.registry.execute(&call.name, call.args.clone()),
            )
            .await;

            results.push(ToolResult {
                call_id: call.id,
                output: result.unwrap_or(Err(AgentError::ToolError(
                    "Tool execution timed out".to_string(),
                ))),
            });
        }

        results
    }
}

impl Clone for ToolExecutor {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            max_concurrent: self.max_concurrent,
            timeout: self.timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::EchoTool;
    use async_trait::async_trait;
    use serde_json::json;
    #[tokio::test]
    async fn test_tool_call_new() {
        let call = ToolCall::new("1", "echo", json!({"test": "value"}));
        assert_eq!(call.id, "1");
        assert_eq!(call.name, "echo");
    }

    #[tokio::test]
    async fn test_executor_new() {
        let registry = ToolRegistry::new();
        let executor = ToolExecutor::new(registry);

        assert_eq!(executor.max_concurrent, 5);
        assert_eq!(executor.timeout, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_executor_with_config() {
        let registry = ToolRegistry::new();
        let executor = ToolExecutor::new(registry)
            .with_max_concurrent(10)
            .with_timeout(Duration::from_secs(30));

        assert_eq!(executor.max_concurrent, 10);
        assert_eq!(executor.timeout, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_execute_tools_empty() {
        let registry = ToolRegistry::new();
        let executor = ToolExecutor::new(registry);
        let results = executor.execute_tools(vec![]).await;

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_execute_tools_single() {
        let registry = ToolRegistry::new();
        registry.register(EchoTool::new()).unwrap();

        let executor = ToolExecutor::new(registry);
        let call = ToolCall::new("1", "echo", json!({"test": "value"}));
        let results = executor.execute_tools(vec![call]).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].call_id, "1");
        assert!(results[0].output.is_ok());
    }

    #[tokio::test]
    async fn test_execute_tools_parallel() {
        let registry = ToolRegistry::new();
        registry.register(EchoTool::new()).unwrap();

        let executor = ToolExecutor::new(registry);

        // Create multiple independent calls
        let calls = vec![
            ToolCall::new("1", "echo", json!({"id": 1})),
            ToolCall::new("2", "echo", json!({"id": 2})),
            ToolCall::new("3", "echo", json!({"id": 3})),
        ];

        let results = executor.execute_tools(calls).await;

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.output.is_ok()));
    }

    #[tokio::test]
    async fn test_execute_tools_sequential_with_dependencies() {
        let registry = ToolRegistry::new();
        registry.register(EchoTool::new()).unwrap();

        let executor = ToolExecutor::new(registry);

        // Create calls with dependencies (detected via $output_ marker)
        let calls = vec![
            ToolCall::new("1", "echo", json!({"id": 1})),
            ToolCall::new("2", "echo", json!({"input": "$output_1", "id": 2})),
        ];

        let results = executor.execute_tools(calls).await;

        assert_eq!(results.len(), 2);
        // Both should complete successfully
        assert!(results[0].output.is_ok());
        assert!(results[1].output.is_ok());
    }

    #[tokio::test]
    async fn test_execute_tools_timeout() {
        use async_trait::async_trait;

        // Create a slow tool
        struct SlowTool;

        #[async_trait]
        impl crate::tool::Tool for SlowTool {
            fn name(&self) -> &str {
                "slow"
            }

            async fn execute(&self, _args: Value) -> Result<Value, AgentError> {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok(json!({"status": "done"}))
            }
        }

        let registry = ToolRegistry::new();
        registry.register(SlowTool).unwrap();

        let executor = ToolExecutor::new(registry).with_timeout(Duration::from_millis(100));
        let call = ToolCall::new("1", "slow", json!({}));
        let results = executor.execute_tools(vec![call]).await;

        assert_eq!(results.len(), 1);
        assert!(results[0].output.is_err());
        assert!(matches!(
            results[0].output.as_ref().unwrap_err(),
            AgentError::ToolError(_)
        ));
    }

    #[tokio::test]
    async fn test_execute_tools_tool_not_found() {
        let registry = ToolRegistry::new();
        let executor = ToolExecutor::new(registry);

        let call = ToolCall::new("1", "nonexistent", json!({}));
        let results = executor.execute_tools(vec![call]).await;

        assert_eq!(results.len(), 1);
        assert!(results[0].output.is_err());
    }

    #[tokio::test]
    async fn test_categorize_calls_no_dependencies() {
        let registry = ToolRegistry::new();
        let executor = ToolExecutor::new(registry);

        let calls = vec![
            ToolCall::new("1", "echo", json!({"id": 1})),
            ToolCall::new("2", "echo", json!({"id": 2})),
        ];

        let (independent, dependent) = executor.categorize_calls(&calls);

        assert_eq!(independent.len(), 2);
        assert_eq!(dependent.len(), 0);
    }

    #[tokio::test]
    async fn test_categorize_calls_with_dependencies() {
        let registry = ToolRegistry::new();
        let executor = ToolExecutor::new(registry);

        let calls = vec![
            ToolCall::new("1", "echo", json!({"id": 1})),
            ToolCall::new("2", "echo", json!({"input": "$output_1", "id": 2})),
            ToolCall::new("3", "echo", json!({"id": 3})),
        ];

        let (independent, dependent) = executor.categorize_calls(&calls);

        assert_eq!(independent.len(), 2); // calls 1 and 3
        assert_eq!(dependent.len(), 1); // call 2
    }

    #[tokio::test]
    async fn test_parallel_with_concurrency_limit() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::time::Instant;

        struct ConcurrentTool {
            counter: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl crate::tool::Tool for ConcurrentTool {
            fn name(&self) -> &str {
                "concurrent"
            }

            async fn execute(&self, _args: Value) -> Result<Value, AgentError> {
                let count = self.counter.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(100)).await;
                self.counter.fetch_sub(1, Ordering::SeqCst);
                Ok(json!({"count": count}))
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));

        let registry = ToolRegistry::new();
        registry
            .register(ConcurrentTool {
                counter: counter.clone(),
            })
            .unwrap();

        let executor = ToolExecutor::new(registry).with_max_concurrent(2);

        let calls: Vec<ToolCall> = (0..5)
            .map(|i| ToolCall::new(i.to_string(), "concurrent", json!({})))
            .collect();

        let start = Instant::now();
        let results = executor.execute_tools(calls).await;
        let duration = start.elapsed();

        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.output.is_ok()));

        // With concurrency limit of 2 and 100ms sleep per call:
        // Should take ~250ms (5 calls / 2 concurrency * 100ms)
        // Allow some margin for overhead
        assert!(duration.as_millis() >= 200 && duration.as_millis() <= 400);
    }
}
