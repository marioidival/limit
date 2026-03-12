use crate::error::CliError;
use chrono::{DateTime, Utc};
use limit_llm::Message;
use std::fs;
use std::path::PathBuf;

/// Export format for session sharing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    /// Markdown format (default)
    Markdown,
    /// JSON format
    Json,
}

/// Session export data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionExport {
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub exported_at: DateTime<Utc>,
    pub model: Option<String>,
    pub messages: Vec<ExportedMessage>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

/// A single message in the export
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportedMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

impl SessionExport {
    /// Create a new session export
    pub fn new(
        session_id: String,
        messages: &[Message],
        total_input_tokens: u64,
        total_output_tokens: u64,
        model: Option<String>,
    ) -> Self {
        let exported_messages: Vec<ExportedMessage> = messages
            .iter()
            .filter(|m| {
                // Only include User and Assistant messages
                matches!(m.role, limit_llm::Role::User | limit_llm::Role::Assistant)
            })
            .filter(|m| {
                // Filter out empty messages
                m.content
                    .as_ref()
                    .map(|c| !c.trim().is_empty())
                    .unwrap_or(false)
            })
            .filter(|m| {
                // Filter out system messages that might be in User role
                // (e.g., "We've reached the iteration limit" or other auto-generated messages)
                let content = m.content.as_deref().unwrap_or("");
                !content.starts_with("We've reached the iteration limit")
            })
            .map(|m| ExportedMessage {
                role: format!("{:?}", m.role),
                content: m.content.clone().unwrap_or_default(),
                timestamp: None,
            })
            .collect();

        Self {
            session_id,
            created_at: Utc::now(),
            exported_at: Utc::now(),
            model,
            messages: exported_messages,
            total_input_tokens,
            total_output_tokens,
        }
    }

    /// Export to markdown format
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Header
        md.push_str("# Session Export\n\n");

        // Metadata
        md.push_str(&format!(
            "**Session ID:** `{}`\n\n",
            &self.session_id[..self.session_id.len().min(8)]
        ));
        md.push_str(&format!(
            "**Exported:** {}\n\n",
            self.exported_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        if let Some(ref model) = self.model {
            md.push_str(&format!("**Model:** {}\n\n", model));
        }

        md.push_str(&format!("**Messages:** {}\n\n", self.messages.len()));
        md.push_str(&format!(
            "**Tokens:** ↑{} ↓{}\n\n",
            self.total_input_tokens, self.total_output_tokens
        ));

        md.push_str("---\n\n");

        // Messages
        for msg in &self.messages {
            match msg.role.as_str() {
                "User" => {
                    md.push_str(&format!("### 👤 User\n\n{}\n\n", msg.content));
                }
                "Assistant" => {
                    md.push_str(&format!("### 🤖 Assistant\n\n{}\n\n", msg.content));
                }
                _ => {
                    md.push_str(&format!("### {}\n\n{}\n\n", msg.role, msg.content));
                }
            }
            md.push_str("---\n\n");
        }

        md
    }

    /// Export to JSON format
    pub fn to_json(&self) -> Result<String, CliError> {
        serde_json::to_string_pretty(&self)
            .map_err(|e| CliError::ConfigError(format!("Failed to serialize to JSON: {}", e)))
    }

    /// Save export to file
    pub fn save_to_file(&self, path: &PathBuf, format: ExportFormat) -> Result<(), CliError> {
        let content = match format {
            ExportFormat::Markdown => self.to_markdown(),
            ExportFormat::Json => self.to_json()?,
        };

        fs::write(path, content)
            .map_err(|e| CliError::ConfigError(format!("Failed to write export file: {}", e)))?;

        Ok(())
    }

    /// Copy to clipboard (returns the content that was copied)
    pub fn to_clipboard(&self, format: ExportFormat) -> Result<String, CliError> {
        let content = match format {
            ExportFormat::Markdown => self.to_markdown(),
            ExportFormat::Json => self.to_json()?,
        };

        Ok(content)
    }
}

/// Share session utility functions
pub struct SessionShare;

impl SessionShare {
    /// Export current session to a file in ~/.limit/exports/
    pub fn export_session(
        session_id: &str,
        messages: &[Message],
        total_input_tokens: u64,
        total_output_tokens: u64,
        model: Option<String>,
        format: ExportFormat,
    ) -> Result<(PathBuf, SessionExport), CliError> {
        // Create exports directory
        let home_dir = dirs::home_dir()
            .ok_or_else(|| CliError::ConfigError("Failed to get home directory".to_string()))?;
        let exports_dir = home_dir.join(".limit").join("exports");
        fs::create_dir_all(&exports_dir).map_err(|e| {
            CliError::ConfigError(format!("Failed to create exports directory: {}", e))
        })?;

        // Create export
        let export = SessionExport::new(
            session_id.to_string(),
            messages,
            total_input_tokens,
            total_output_tokens,
            model,
        );

        // Generate filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let extension = match format {
            ExportFormat::Markdown => "md",
            ExportFormat::Json => "json",
        };
        let short_id = &session_id[..session_id.len().min(8)];
        let filename = format!("session_{}_{}.{}", short_id, timestamp, extension);
        let filepath = exports_dir.join(&filename);

        // Save to file
        export.save_to_file(&filepath, format)?;

        Ok((filepath, export))
    }

    /// Generate shareable content for clipboard
    pub fn generate_share_content(
        session_id: &str,
        messages: &[Message],
        total_input_tokens: u64,
        total_output_tokens: u64,
        model: Option<String>,
        format: ExportFormat,
    ) -> Result<String, CliError> {
        let export = SessionExport::new(
            session_id.to_string(),
            messages,
            total_input_tokens,
            total_output_tokens,
            model,
        );

        export.to_clipboard(format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_export_markdown() {
        let messages = vec![
            Message {
                role: limit_llm::Role::User,
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: limit_llm::Role::Assistant,
                content: Some("Hi there!".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let export = SessionExport::new(
            "test-session-123".to_string(),
            &messages,
            100,
            50,
            Some("claude-3".to_string()),
        );

        let md = export.to_markdown();
        assert!(md.contains("Session Export"));
        assert!(md.contains("test-ses"));
        assert!(md.contains("👤 User"));
        assert!(md.contains("Hello"));
        assert!(md.contains("🤖 Assistant"));
        assert!(md.contains("Hi there!"));
    }

    #[test]
    fn test_session_export_json() {
        let messages = vec![Message {
            role: limit_llm::Role::User,
            content: Some("Test".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        let export = SessionExport::new("test-id".to_string(), &messages, 10, 5, None);

        let json = export.to_json().unwrap();
        assert!(json.contains("\"role\": \"User\""));
        assert!(json.contains("\"content\": \"Test\""));
    }

    #[test]
    fn test_export_filters_tool_messages() {
        let messages = vec![
            Message {
                role: limit_llm::Role::User,
                content: Some("User message".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: limit_llm::Role::Tool,
                content: Some("Tool result".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: limit_llm::Role::System,
                content: Some("System message".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let export = SessionExport::new("test-id".to_string(), &messages, 0, 0, None);

        // Only User and Assistant messages should be exported
        assert_eq!(export.messages.len(), 1);
        assert_eq!(export.messages[0].role, "User");
    }
}
