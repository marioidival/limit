//! Browser configuration types
//!
//! Provides configuration options for browser automation.

use std::path::PathBuf;

/// Supported browser engines
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum BrowserEngine {
    /// Google Chrome (default)
    #[default]
    Chrome,
    /// Lightpanda - lightweight browser
    Lightpanda,
}

impl BrowserEngine {
    /// Get the CLI argument value for this engine
    pub fn as_arg(&self) -> &'static str {
        match self {
            BrowserEngine::Chrome => "chrome",
            BrowserEngine::Lightpanda => "lightpanda",
        }
    }
}

/// Configuration for browser automation
#[derive(Clone, Debug)]
pub struct BrowserConfig {
    /// Path to agent-browser binary (defaults to "agent-browser" in PATH)
    pub binary_path: Option<PathBuf>,
    /// Browser engine to use
    pub engine: BrowserEngine,
    /// Run in headless mode (no visible window)
    pub headless: bool,
    /// Timeout for operations in milliseconds
    pub timeout_ms: u64,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            binary_path: None,
            engine: BrowserEngine::default(),
            headless: true,
            timeout_ms: 30_000,
        }
    }
}

impl BrowserConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the binary path
    pub fn with_binary_path(mut self, path: PathBuf) -> Self {
        self.binary_path = Some(path);
        self
    }

    /// Set the browser engine
    pub fn with_engine(mut self, engine: BrowserEngine) -> Self {
        self.engine = engine;
        self
    }

    /// Set headless mode
    pub fn with_headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    /// Set timeout in milliseconds
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Get the binary name/path to execute
    pub fn binary(&self) -> &std::path::Path {
        self.binary_path
            .as_deref()
            .unwrap_or(std::path::Path::new("agent-browser"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_engine_default() {
        let engine = BrowserEngine::default();
        assert_eq!(engine, BrowserEngine::Chrome);
    }

    #[test]
    fn test_browser_engine_as_arg() {
        assert_eq!(BrowserEngine::Chrome.as_arg(), "chrome");
        assert_eq!(BrowserEngine::Lightpanda.as_arg(), "lightpanda");
    }

    #[test]
    fn test_browser_config_default() {
        let config = BrowserConfig::default();
        assert!(config.binary_path.is_none());
        assert_eq!(config.engine, BrowserEngine::Chrome);
        assert!(config.headless);
        assert_eq!(config.timeout_ms, 30_000);
    }

    #[test]
    fn test_browser_config_builder() {
        let config = BrowserConfig::new()
            .with_engine(BrowserEngine::Lightpanda)
            .with_headless(false)
            .with_timeout_ms(60_000);

        assert_eq!(config.engine, BrowserEngine::Lightpanda);
        assert!(!config.headless);
        assert_eq!(config.timeout_ms, 60_000);
    }

    #[test]
    fn test_browser_config_binary() {
        let config = BrowserConfig::default();
        assert_eq!(config.binary().to_str().unwrap(), "agent-browser");

        let config =
            BrowserConfig::new().with_binary_path(PathBuf::from("/usr/local/bin/agent-browser"));
        assert_eq!(
            config.binary().to_str().unwrap(),
            "/usr/local/bin/agent-browser"
        );
    }
}
