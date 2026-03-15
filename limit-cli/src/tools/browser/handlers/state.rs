use super::super::args::ArgsExt;
use super::super::client::BrowserClient;
use super::super::client_ext::{InteractionExt, QueryExt};
use super::super::response::ok_msg;
use limit_agent::error::AgentError;
use serde_json::Value;

pub async fn find(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let locator = args.get_str("locator", "find")?;
    let value = args.get_str("value", "find")?;
    let action = args.get_str("action", "find")?;
    let action_value = args.get_opt_str("action_value");
    let result = client.find(locator, value, action, action_value).await?;
    Ok(serde_json::json!({
        "success": true,
        "result": result,
        "message": format!("Find {}='{}' with action '{}'", locator, value, action)
    }))
}

pub async fn scroll(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let direction = args.get_str("direction", "scroll")?;
    let pixels = args.get_opt_u64("pixels").map(|p| p as u32);
    client.scroll(direction, pixels).await?;
    Ok(ok_msg(format!("Scrolled {}", direction)))
}

pub async fn is(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let what = args.get_str("what", "is")?;
    let selector = args.get_str("selector", "is")?;
    let result = client.is_(what, selector).await?;
    Ok(serde_json::json!({
        "success": true,
        "result": result,
        "message": format!("Element {} is {}: {}", selector, what, result)
    }))
}

pub async fn download(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "download")?;
    let path = args.get_str("path", "download")?;
    let download_path = client.download(selector, path).await?;
    Ok(serde_json::json!({
        "success": true,
        "path": download_path,
        "message": format!("Downloaded to: {}", download_path)
    }))
}
