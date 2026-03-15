//! Argument extraction extension trait for JSON values
//!
//! Provides convenient methods for extracting typed arguments from `serde_json::Value`
//! with standardized error formatting.

use limit_agent::error::AgentError;
use serde_json::Value;

/// Extension trait for extracting arguments from JSON values
///
/// This trait provides methods for extracting typed values from `serde_json::Value`
/// with standardized error messages for missing arguments.
///
/// # Example
///
/// ```ignore
/// use serde_json::json;
/// use limit_cli::tools::browser::args::ArgsExt;
///
/// let args = json!({
///     "url": "https://example.com",
///     "timeout": 5000,
///     "optional": "value"
/// });
///
/// // Required arguments
/// let url = args.get_str("url", "open")?;
/// let timeout = args.get_u64("timeout", "configure")?;
///
/// // Optional arguments
/// let optional = args.get_opt_str("optional");
/// ```
pub trait ArgsExt {
    /// Get a required string argument
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up in the JSON object
    /// * `action` - The action name for error reporting
    ///
    /// # Returns
    ///
    /// * `Ok(&str)` - The string value if found
    /// * `Err(AgentError)` - If the key is missing or not a string
    ///
    /// # Example
    ///
    /// ```ignore
    /// let url = args.get_str("url", "open")?;
    /// ```
    fn get_str(&self, key: &str, action: &str) -> Result<&str, AgentError>;

    /// Get an optional string argument
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up in the JSON object
    ///
    /// # Returns
    ///
    /// * `Some(&str)` - The string value if found
    /// * `None` - If the key is missing or not a string
    ///
    /// # Example
    ///
    /// ```ignore
    /// let path = args.get_opt_str("path");
    /// ```
    fn get_opt_str(&self, key: &str) -> Option<&str>;

    /// Get a required u64 argument
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up in the JSON object
    /// * `action` - The action name for error reporting
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The u64 value if found
    /// * `Err(AgentError)` - If the key is missing or not a u64
    ///
    /// # Example
    ///
    /// ```ignore
    /// let index = args.get_u64("index", "tab_select")?;
    /// ```
    fn get_u64(&self, key: &str, action: &str) -> Result<u64, AgentError>;

    /// Get an optional u64 argument
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up in the JSON object
    ///
    /// # Returns
    ///
    /// * `Some(u64)` - The u64 value if found
    /// * `None` - If the key is missing or not a u64
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pixels = args.get_opt_u64("pixels");
    /// ```
    fn get_opt_u64(&self, key: &str) -> Option<u64>;

    /// Get a required f64 argument
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up in the JSON object
    /// * `action` - The action name for error reporting
    ///
    /// # Returns
    ///
    /// * `Ok(f64)` - The f64 value if found
    /// * `Err(AgentError)` - If the key is missing or not an f64
    ///
    /// # Example
    ///
    /// ```ignore
    /// let latitude = args.get_f64("latitude", "set_geo")?;
    /// ```
    fn get_f64(&self, key: &str, action: &str) -> Result<f64, AgentError>;

    /// Get an optional f64 argument
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up in the JSON object
    ///
    /// # Returns
    ///
    /// * `Some(f64)` - The f64 value if found
    /// * `None` - If the key is missing or not an f64
    ///
    /// # Example
    ///
    /// ```ignore
    /// let scale = args.get_opt_f64("scale");
    /// ```
    fn get_opt_f64(&self, key: &str) -> Option<f64>;
}

impl ArgsExt for Value {
    fn get_str(&self, key: &str, action: &str) -> Result<&str, AgentError> {
        self.get(key).and_then(|v| v.as_str()).ok_or_else(|| {
            AgentError::ToolError(format!("Missing '{}' argument for {} action", key, action))
        })
    }

    fn get_opt_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    fn get_u64(&self, key: &str, action: &str) -> Result<u64, AgentError> {
        self.get(key).and_then(|v| v.as_u64()).ok_or_else(|| {
            AgentError::ToolError(format!("Missing '{}' argument for {} action", key, action))
        })
    }

    fn get_opt_u64(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(|v| v.as_u64())
    }

    fn get_f64(&self, key: &str, action: &str) -> Result<f64, AgentError> {
        self.get(key).and_then(|v| v.as_f64()).ok_or_else(|| {
            AgentError::ToolError(format!("Missing '{}' argument for {} action", key, action))
        })
    }

    fn get_opt_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_str_success() {
        let args = serde_json::json!({"url": "https://example.com"});
        let result = args.get_str("url", "open");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com");
    }

    #[test]
    fn test_get_str_missing_key() {
        let args = serde_json::json!({});
        let result = args.get_str("url", "open");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("Missing 'url' argument for open action"));
    }

    #[test]
    fn test_get_str_wrong_type() {
        let args = serde_json::json!({"url": 123});
        let result = args.get_str("url", "open");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("Missing 'url' argument for open action"));
    }

    #[test]
    fn test_get_opt_str_some() {
        let args = serde_json::json!({"path": "/tmp/screenshot.png"});
        let result = args.get_opt_str("path");
        assert_eq!(result, Some("/tmp/screenshot.png"));
    }

    #[test]
    fn test_get_opt_str_none() {
        let args = serde_json::json!({});
        let result = args.get_opt_str("path");
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_opt_str_wrong_type() {
        let args = serde_json::json!({"path": 123});
        let result = args.get_opt_str("path");
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_u64_success() {
        let args = serde_json::json!({"index": 42});
        let result = args.get_u64("index", "tab_select");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_get_u64_missing_key() {
        let args = serde_json::json!({});
        let result = args.get_u64("index", "tab_select");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("Missing 'index' argument for tab_select action"));
    }

    #[test]
    fn test_get_u64_wrong_type() {
        let args = serde_json::json!({"index": "not a number"});
        let result = args.get_u64("index", "tab_select");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_opt_u64_some() {
        let args = serde_json::json!({"pixels": 1000});
        let result = args.get_opt_u64("pixels");
        assert_eq!(result, Some(1000));
    }

    #[test]
    fn test_get_opt_u64_none() {
        let args = serde_json::json!({});
        let result = args.get_opt_u64("pixels");
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_f64_success() {
        let args = serde_json::json!({"latitude": 40.7128});
        let result = args.get_f64("latitude", "set_geo");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 40.7128);
    }

    #[test]
    fn test_get_f64_missing_key() {
        let args = serde_json::json!({});
        let result = args.get_f64("latitude", "set_geo");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("Missing 'latitude' argument for set_geo action"));
    }

    #[test]
    fn test_get_f64_wrong_type() {
        let args = serde_json::json!({"latitude": "not a number"});
        let result = args.get_f64("latitude", "set_geo");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_opt_f64_some() {
        let args = serde_json::json!({"scale": 1.5});
        let result = args.get_opt_f64("scale");
        assert_eq!(result, Some(1.5));
    }

    #[test]
    fn test_get_opt_f64_none() {
        let args = serde_json::json!({});
        let result = args.get_opt_f64("scale");
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_u64_with_float() {
        // serde_json can parse floats as f64, but as_u64() should work for integers
        let args = serde_json::json!({"index": 42u64});
        let result = args.get_u64("index", "tab_select");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_get_f64_with_integer() {
        // f64 can be parsed from integers
        let args = serde_json::json!({"scale": 2});
        let result = args.get_f64("scale", "set_viewport");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2.0);
    }
}
