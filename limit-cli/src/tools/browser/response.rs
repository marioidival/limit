//! JSON response builder for browser tool
//!
//! Provides a fluent builder pattern for constructing JSON responses
//! with consistent structure across all browser operations.

use serde_json::{json, Value};

/// Builder for constructing JSON responses
#[derive(Default)]
pub struct Response {
    success: bool,
    message: Option<String>,
    data: Vec<(&'static str, Value)>,
}

impl Response {
    /// Create a successful response
    pub fn success() -> Self {
        Self {
            success: true,
            ..Default::default()
        }
    }

    /// Create a failed response
    pub fn failure() -> Self {
        Self {
            success: false,
            ..Default::default()
        }
    }

    /// Add a message to the response
    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    /// Add a key-value data pair
    pub fn data(mut self, key: &'static str, value: impl Into<Value>) -> Self {
        self.data.push((key, value.into()));
        self
    }

    /// Build the final JSON value
    pub fn build(self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("success".to_string(), json!(self.success));

        if let Some(msg) = self.message {
            map.insert("message".to_string(), json!(msg));
        }

        for (key, value) in self.data {
            map.insert(key.to_string(), value);
        }

        Value::Object(map)
    }
}

/// Convenience function for simple success with message
pub fn ok_msg(msg: impl Into<String>) -> Value {
    Response::success().message(msg).build()
}

/// Convenience function for simple success without message
pub fn ok() -> Value {
    Response::success().build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_success() {
        let response = Response::success().build();
        assert_eq!(response["success"], true);
    }

    #[test]
    fn test_response_failure() {
        let response = Response::failure().build();
        assert_eq!(response["success"], false);
    }

    #[test]
    fn test_response_with_message() {
        let response = Response::success().message("Test message").build();
        assert_eq!(response["success"], true);
        assert_eq!(response["message"], "Test message");
    }

    #[test]
    fn test_response_with_data() {
        let response = Response::success()
            .data("url", "https://example.com")
            .data("count", 42)
            .build();
        assert_eq!(response["success"], true);
        assert_eq!(response["url"], "https://example.com");
        assert_eq!(response["count"], 42);
    }

    #[test]
    fn test_response_with_message_and_data() {
        let response = Response::success()
            .message("Opened page")
            .data("url", "https://example.com")
            .data("title", "Example Page")
            .build();
        assert_eq!(response["success"], true);
        assert_eq!(response["message"], "Opened page");
        assert_eq!(response["url"], "https://example.com");
        assert_eq!(response["title"], "Example Page");
    }

    #[test]
    fn test_ok_msg() {
        let response = ok_msg("Test message");
        assert_eq!(response["success"], true);
        assert_eq!(response["message"], "Test message");
    }

    #[test]
    fn test_ok() {
        let response = ok();
        assert_eq!(response["success"], true);
        assert!(!response.as_object().unwrap().contains_key("message"));
    }

    #[test]
    fn test_response_builder_chain() {
        let response = Response::success()
            .message("Snapshot taken")
            .data("content", "<html>...</html>")
            .data("title", "Test Page")
            .data("url", "https://example.com")
            .build();

        assert_eq!(response["success"], true);
        assert_eq!(response["message"], "Snapshot taken");
        assert_eq!(response["content"], "<html>...</html>");
        assert_eq!(response["title"], "Test Page");
        assert_eq!(response["url"], "https://example.com");
    }
}
