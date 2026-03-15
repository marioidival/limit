//! Data types for browser operations

use serde::{Deserialize, Serialize};

/// Result from snapshot operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResult {
    /// The accessibility tree as markdown
    pub content: String,
    /// Page title if available
    pub title: Option<String>,
    /// Current URL
    pub url: Option<String>,
}

/// Information about a browser tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    /// Tab index
    pub index: usize,
    /// Tab title
    pub title: String,
}

/// Element bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Width
    pub width: f64,
    /// Height
    pub height: f64,
}

/// Cookie information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    /// Cookie name
    pub name: String,
    /// Cookie value
    pub value: String,
}

/// Network request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// HTTP method
    pub method: String,
    /// Request URL
    pub url: String,
}
