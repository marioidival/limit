use crate::error::AgentError;
use crate::tool::Tool;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        ToolRegistry {
            tools: HashMap::new(),
        }
    }

    /// Register a tool with the registry
    pub fn register<T>(&mut self, tool: T) -> Result<(), AgentError>
    where
        T: Tool + 'static,
    {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::new(tool));
        Ok(())
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// List all registered tool names
    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }

    /// Execute a tool by name with the given arguments
    pub async fn execute(&self, name: &str, args: Value) -> Result<Value, AgentError> {
        let tool = self
            .get(name)
            .ok_or_else(|| AgentError::ToolError(format!("Tool '{}' not found", name)))?;
        tool.execute(args).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::EchoTool;

    #[tokio::test]
    async fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }

    #[tokio::test]
    async fn test_registry_default() {
        let registry = ToolRegistry::default();
        assert_eq!(registry.list().len(), 0);
    }

    #[tokio::test]
    async fn test_registry_register() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool::new()).unwrap();

        assert_eq!(registry.list().len(), 1);
        assert_eq!(registry.list()[0], "echo");
    }

    #[tokio::test]
    async fn test_registry_get() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool::new()).unwrap();

        let tool = registry.get("echo");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "echo");
    }

    #[tokio::test]
    async fn test_registry_get_nonexistent() {
        let registry = ToolRegistry::new();
        let tool = registry.get("nonexistent");
        assert!(tool.is_none());
    }

    #[tokio::test]
    async fn test_registry_list() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool::new()).unwrap();

        let names = registry.list();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "echo");
    }

    #[tokio::test]
    async fn test_registry_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool::new()).unwrap();

        let args = serde_json::json!({"test": "value"});
        let result = registry.execute("echo", args.clone()).await.unwrap();
        assert_eq!(result, args);
    }

    #[tokio::test]
    async fn test_registry_execute_nonexistent() {
        let registry = ToolRegistry::new();

        let args = serde_json::json!({"test": "value"});
        let result = registry.execute("nonexistent", args).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AgentError::ToolError(_)));
    }

    #[tokio::test]
    async fn test_registry_multiple_tools() {
        use async_trait::async_trait;

        struct AnotherTool;

        #[async_trait]
        impl Tool for AnotherTool {
            fn name(&self) -> &str {
                "another"
            }

            async fn execute(&self, _args: Value) -> Result<Value, AgentError> {
                Ok(serde_json::json!({"status": "ok"}))
            }
        }

        let mut registry = ToolRegistry::new();
        registry.register(EchoTool::new()).unwrap();
        registry.register(AnotherTool).unwrap();

        let names = registry.list();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"echo".to_string()));
        assert!(names.contains(&"another".to_string()));
    }
}
