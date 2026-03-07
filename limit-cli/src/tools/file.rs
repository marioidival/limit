use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use serde_json::Value;
use std::fs;
use std::path::Path;

const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB

pub struct FileReadTool;

impl FileReadTool {
    pub fn new() -> Self {
        FileReadTool
    }
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let path: String = serde_json::from_value(args["path"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid path argument: {}", e)))?;

        let path_obj = Path::new(&path);

        // Check if file exists
        if !path_obj.exists() {
            return Err(AgentError::ToolError(format!("File not found: {}", path)));
        }

        // Check if it's a file
        if !path_obj.is_file() {
            return Err(AgentError::ToolError(format!(
                "Path is not a file: {}",
                path
            )));
        }

        // Check file size
        let metadata = fs::metadata(&path)
            .map_err(|e| AgentError::IoError(format!("Failed to read file metadata: {}", e)))?;

        if metadata.len() > MAX_FILE_SIZE {
            return Err(AgentError::ToolError(format!(
                "File too large: {} bytes (max: {} bytes)",
                metadata.len(),
                MAX_FILE_SIZE
            )));
        }

        // Read file
        let content = fs::read_to_string(&path)
            .map_err(|e| AgentError::IoError(format!("Failed to read file: {}", e)))?;

        // Check for binary content (null bytes)
        if content.contains('\0') {
            return Err(AgentError::ToolError(format!(
                "Binary file detected: {}",
                path
            )));
        }

        Ok(serde_json::json!({
            "content": content,
            "size": metadata.len()
        }))
    }
}

pub struct FileWriteTool;

impl FileWriteTool {
    pub fn new() -> Self {
        FileWriteTool
    }
}

impl Default for FileWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let path: String = serde_json::from_value(args["path"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid path argument: {}", e)))?;

        let content: String = serde_json::from_value(args["content"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid content argument: {}", e)))?;

        let path_obj = Path::new(&path);

        // Create parent directories if they don't exist
        if let Some(parent) = path_obj.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    AgentError::IoError(format!("Failed to create directories: {}", e))
                })?;
            }
        }

        // Write file
        fs::write(&path, &content)
            .map_err(|e| AgentError::IoError(format!("Failed to write file: {}", e)))?;

        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "size": content.len()
        }))
    }
}

pub struct FileEditTool;

impl FileEditTool {
    pub fn new() -> Self {
        FileEditTool
    }
}

impl Default for FileEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let path: String = serde_json::from_value(args["path"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid path argument: {}", e)))?;

        let old_text: String = serde_json::from_value(args["old_text"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid old_text argument: {}", e)))?;

        let new_text: String = serde_json::from_value(args["new_text"].clone())
            .map_err(|e| AgentError::ToolError(format!("Invalid new_text argument: {}", e)))?;

        let path_obj = Path::new(&path);

        // Check if file exists
        if !path_obj.exists() {
            return Err(AgentError::ToolError(format!("File not found: {}", path)));
        }

        // Read file content
        let current_content = fs::read_to_string(&path)
            .map_err(|e| AgentError::IoError(format!("Failed to read file: {}", e)))?;

        // Check file size
        let metadata = fs::metadata(&path)
            .map_err(|e| AgentError::IoError(format!("Failed to read file metadata: {}", e)))?;

        if metadata.len() > MAX_FILE_SIZE {
            return Err(AgentError::ToolError(format!(
                "File too large: {} bytes (max: {} bytes)",
                metadata.len(),
                MAX_FILE_SIZE
            )));
        }

        // Check for binary content
        if current_content.contains('\0') {
            return Err(AgentError::ToolError(format!(
                "Binary file detected: {}",
                path
            )));
        }

        // Check if old_text exists in file
        if !current_content.contains(&old_text) {
            return Err(AgentError::ToolError(
                "old_text not found in file".to_string(),
            ));
        }

        // Replace old_text with new_text
        let new_content = current_content.replace(&old_text, &new_text);

        // Write modified content back
        fs::write(&path, new_content)
            .map_err(|e| AgentError::IoError(format!("Failed to write file: {}", e)))?;

        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "replacements": current_content.matches(&old_text).count()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_read_tool_name() {
        let tool = FileReadTool::new();
        assert_eq!(tool.name(), "file_read");
    }

    #[tokio::test]
    async fn test_file_read_tool_execute() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, World!").unwrap();
        writeln!(temp_file, "This is a test.").unwrap();

        let tool = FileReadTool::new();
        let args = serde_json::json!({
            "path": temp_file.path().to_str().unwrap()
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result["content"].is_string());
        assert!(result["size"].is_u64());
        assert!(result["content"]
            .as_str()
            .unwrap()
            .contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_file_read_tool_file_not_found() {
        let tool = FileReadTool::new();
        let args = serde_json::json!({
            "path": "/nonexistent/file.txt"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[tokio::test]
    async fn test_file_read_tool_invalid_path() {
        let tool = FileReadTool::new();
        let args = serde_json::json!({}); // Missing path

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_write_tool_name() {
        let tool = FileWriteTool::new();
        assert_eq!(tool.name(), "file_write");
    }

    #[tokio::test]
    async fn test_file_write_tool_execute() {
        let temp_file = NamedTempFile::new().unwrap();
        let tool = FileWriteTool::new();
        let content = "Hello from test!";

        let args = serde_json::json!({
            "path": temp_file.path().to_str().unwrap(),
            "content": content
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["size"], content.len());

        // Verify content was written
        let written = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(written, content);
    }

    #[tokio::test]
    async fn test_file_write_tool_create_dirs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let nested_path = temp_dir.path().join("nested/dir/file.txt");

        let tool = FileWriteTool::new();
        let args = serde_json::json!({
            "path": nested_path.to_str().unwrap(),
            "content": "Test content"
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(result["success"], true);
        assert!(nested_path.exists());
    }

    #[tokio::test]
    async fn test_file_edit_tool_name() {
        let tool = FileEditTool::new();
        assert_eq!(tool.name(), "file_edit");
    }

    #[tokio::test]
    async fn test_file_edit_tool_execute() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, World!").unwrap();
        writeln!(temp_file, "Goodbye, World!").unwrap();

        let tool = FileEditTool::new();
        let args = serde_json::json!({
            "path": temp_file.path().to_str().unwrap(),
            "old_text": "Hello, World!",
            "new_text": "Hello, Rust!"
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["replacements"], 1);

        // Verify edit
        let content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("Hello, Rust!"));
        assert!(!content.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_file_edit_tool_old_text_not_found() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, World!").unwrap();

        let tool = FileEditTool::new();
        let args = serde_json::json!({
            "path": temp_file.path().to_str().unwrap(),
            "old_text": "Nonexistent text",
            "new_text": "Replacement"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("old_text not found"));
    }

    #[tokio::test]
    async fn test_file_read_tool_binary_detection() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // Write binary content with null byte
        temp_file.write_all(b"Hello\x00World").unwrap();

        let tool = FileReadTool::new();
        let args = serde_json::json!({
            "path": temp_file.path().to_str().unwrap()
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Binary file"));
    }
}
