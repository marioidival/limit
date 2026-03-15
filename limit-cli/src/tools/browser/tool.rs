//! Browser tool for LLM agent
//!
//! Implements the Tool trait for browser automation in the agent system.

use super::action::BrowserAction;
use super::client::BrowserClient;
use super::config::{BrowserConfig, BrowserEngine};
use super::executor::{BrowserExecutor, CliExecutor};
use super::handlers;
use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;

/// Browser automation tool for the LLM agent
pub struct BrowserTool {
    client: BrowserClient,
}

impl BrowserTool {
    /// Create a new browser tool with default configuration
    pub fn new() -> Self {
        Self::with_config(BrowserConfig::default())
    }

    /// Create a browser tool with custom configuration
    pub fn with_config(config: BrowserConfig) -> Self {
        let executor = Arc::new(CliExecutor::new(config));
        Self {
            client: BrowserClient::new(executor),
        }
    }

    /// Create a browser tool with a custom executor (for testing)
    pub fn with_executor(executor: Arc<dyn BrowserExecutor>) -> Self {
        Self {
            client: BrowserClient::new(executor),
        }
    }

    /// Parse engine from string
    #[allow(dead_code)]
    fn parse_engine(s: &str) -> Option<BrowserEngine> {
        match s.to_lowercase().as_str() {
            "chrome" => Some(BrowserEngine::Chrome),
            "lightpanda" => Some(BrowserEngine::Lightpanda),
            _ => None,
        }
    }
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for BrowserTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserTool").finish()
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::ToolError("Missing 'action' argument".to_string()))?;

        match BrowserAction::from_str(action)? {
            BrowserAction::Open => handlers::navigation::open(&self.client, &args).await,
            BrowserAction::Close => handlers::navigation::close(&self.client, &args).await,
            BrowserAction::Snapshot => handlers::navigation::snapshot(&self.client, &args).await,
            BrowserAction::Screenshot => {
                handlers::navigation::screenshot(&self.client, &args).await
            }
            BrowserAction::Back => handlers::navigation::back(&self.client, &args).await,
            BrowserAction::Forward => handlers::navigation::forward(&self.client, &args).await,
            BrowserAction::Reload => handlers::navigation::reload(&self.client, &args).await,
            BrowserAction::Click => handlers::interaction::click(&self.client, &args).await,
            BrowserAction::Fill => handlers::interaction::fill(&self.client, &args).await,
            BrowserAction::Type => handlers::interaction::type_(&self.client, &args).await,
            BrowserAction::Press => handlers::interaction::press(&self.client, &args).await,
            BrowserAction::Hover => handlers::interaction::hover(&self.client, &args).await,
            BrowserAction::Select => handlers::interaction::select(&self.client, &args).await,
            BrowserAction::Dblclick => handlers::interaction::dblclick(&self.client, &args).await,
            BrowserAction::Focus => handlers::interaction::focus(&self.client, &args).await,
            BrowserAction::Check => handlers::interaction::check(&self.client, &args).await,
            BrowserAction::Uncheck => handlers::interaction::uncheck(&self.client, &args).await,
            BrowserAction::Scrollintoview => {
                handlers::interaction::scrollintoview(&self.client, &args).await
            }
            BrowserAction::Drag => handlers::interaction::drag(&self.client, &args).await,
            BrowserAction::Upload => handlers::interaction::upload(&self.client, &args).await,
            BrowserAction::Pdf => handlers::interaction::pdf(&self.client, &args).await,
            BrowserAction::Get => handlers::query::get(&self.client, &args).await,
            BrowserAction::GetAttr => handlers::query::get_attr(&self.client, &args).await,
            BrowserAction::GetCount => handlers::query::get_count(&self.client, &args).await,
            BrowserAction::GetBox => handlers::query::get_box(&self.client, &args).await,
            BrowserAction::GetStyles => handlers::query::get_styles(&self.client, &args).await,
            BrowserAction::Wait => handlers::wait::wait(&self.client, &args).await,
            BrowserAction::WaitForText => handlers::wait::wait_for_text(&self.client, &args).await,
            BrowserAction::WaitForUrl => handlers::wait::wait_for_url(&self.client, &args).await,
            BrowserAction::WaitForLoad => handlers::wait::wait_for_load(&self.client, &args).await,
            BrowserAction::WaitForDownload => {
                handlers::wait::wait_for_download(&self.client, &args).await
            }
            BrowserAction::WaitForFn => handlers::wait::wait_for_fn(&self.client, &args).await,
            BrowserAction::WaitForState => {
                handlers::wait::wait_for_state(&self.client, &args).await
            }
            BrowserAction::Find => handlers::state::find(&self.client, &args).await,
            BrowserAction::Scroll => handlers::state::scroll(&self.client, &args).await,
            BrowserAction::Is => handlers::state::is(&self.client, &args).await,
            BrowserAction::Download => handlers::state::download(&self.client, &args).await,
            BrowserAction::TabList => handlers::tabs::tab_list(&self.client, &args).await,
            BrowserAction::TabNew => handlers::tabs::tab_new(&self.client, &args).await,
            BrowserAction::TabClose => handlers::tabs::tab_close(&self.client, &args).await,
            BrowserAction::TabSelect => handlers::tabs::tab_select(&self.client, &args).await,
            BrowserAction::DialogAccept => {
                handlers::dialog::dialog_accept(&self.client, &args).await
            }
            BrowserAction::DialogDismiss => {
                handlers::dialog::dialog_dismiss(&self.client, &args).await
            }
            BrowserAction::Cookies => handlers::storage::cookies(&self.client, &args).await,
            BrowserAction::CookiesSet => handlers::storage::cookies_set(&self.client, &args).await,
            BrowserAction::StorageGet => handlers::storage::storage_get(&self.client, &args).await,
            BrowserAction::StorageSet => handlers::storage::storage_set(&self.client, &args).await,
            BrowserAction::NetworkRequests => {
                handlers::storage::network_requests(&self.client, &args).await
            }
            BrowserAction::SetViewport => {
                handlers::settings::set_viewport(&self.client, &args).await
            }
            BrowserAction::SetDevice => handlers::settings::set_device(&self.client, &args).await,
            BrowserAction::SetGeo => handlers::settings::set_geo(&self.client, &args).await,
            BrowserAction::Eval => handlers::query::eval(&self.client, &args).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_tool_name() {
        let tool = BrowserTool::new();
        assert_eq!(tool.name(), "browser");
    }

    #[test]
    fn test_browser_tool_default() {
        let tool = BrowserTool::default();
        assert_eq!(tool.name(), "browser");
    }

    #[test]
    fn test_parse_engine() {
        assert_eq!(
            BrowserTool::parse_engine("chrome"),
            Some(BrowserEngine::Chrome)
        );
        assert_eq!(
            BrowserTool::parse_engine("CHROME"),
            Some(BrowserEngine::Chrome)
        );
        assert_eq!(
            BrowserTool::parse_engine("lightpanda"),
            Some(BrowserEngine::Lightpanda)
        );
        assert_eq!(
            BrowserTool::parse_engine("LightPanda"),
            Some(BrowserEngine::Lightpanda)
        );
        assert_eq!(BrowserTool::parse_engine("invalid"), None);
    }

    #[tokio::test]
    async fn test_browser_tool_missing_action() {
        let tool = BrowserTool::new();
        let args = serde_json::json!({
            "url": "https://example.com"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'action'"));
    }

    #[tokio::test]
    async fn test_browser_tool_unknown_action() {
        let tool = BrowserTool::new();
        let args = serde_json::json!({
            "action": "unknown"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unknown browser action"));
    }

    #[tokio::test]
    async fn test_browser_tool_open_missing_url() {
        let tool = BrowserTool::new();
        let args = serde_json::json!({
            "action": "open"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'url'"));
    }

    #[tokio::test]
    async fn test_browser_tool_click_missing_selector() {
        let tool = BrowserTool::new();
        let args = serde_json::json!({
            "action": "click"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'selector'"));
    }

    #[tokio::test]
    async fn test_browser_tool_screenshot_missing_path() {
        let tool = BrowserTool::new();
        let args = serde_json::json!({
            "action": "screenshot"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'path'"));
    }
}
