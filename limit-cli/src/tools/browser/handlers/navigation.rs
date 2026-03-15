use super::super::args::ArgsExt;
use super::super::client::BrowserClient;
use super::super::client_ext::{NavigationExt, QueryExt};
use super::super::response::ok_msg;
use limit_agent::error::AgentError;
use serde_json::Value;

pub async fn open(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let url = args.get_str("url", "open")?;
    client.open(url).await?;
    Ok(ok_msg(format!("Opened {}", url)))
}

pub async fn close(client: &BrowserClient, _args: &Value) -> Result<Value, AgentError> {
    client.close().await?;
    Ok(ok_msg("Browser closed"))
}

pub async fn snapshot(client: &BrowserClient, _args: &Value) -> Result<Value, AgentError> {
    let result = client.snapshot().await?;
    Ok(serde_json::json!({
        "success": true,
        "content": result.content,
        "title": result.title,
        "url": result.url
    }))
}

pub async fn screenshot(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let path = args.get_str("path", "screenshot")?;
    client.screenshot(path).await?;
    Ok(ok_msg(format!("Screenshot saved to {}", path)))
}

pub async fn back(client: &BrowserClient, _args: &Value) -> Result<Value, AgentError> {
    client.back().await?;
    Ok(ok_msg("Navigated back"))
}

pub async fn forward(client: &BrowserClient, _args: &Value) -> Result<Value, AgentError> {
    client.forward().await?;
    Ok(ok_msg("Navigated forward"))
}

pub async fn reload(client: &BrowserClient, _args: &Value) -> Result<Value, AgentError> {
    client.reload().await?;
    Ok(ok_msg("Page reloaded"))
}
