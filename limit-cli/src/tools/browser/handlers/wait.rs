use super::super::args::ArgsExt;
use super::super::client::BrowserClient;
use super::super::client_ext::WaitingExt;
use super::super::response::ok_msg;
use limit_agent::error::AgentError;
use serde_json::Value;

pub async fn wait(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let condition = args.get_str("wait_for", "wait")?;
    client.wait_for(condition).await?;
    Ok(ok_msg(format!("Wait condition met: {}", condition)))
}

pub async fn wait_for_text(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let text = args.get_str("text", "wait_for_text")?;
    client.wait_for_text(text).await?;
    Ok(ok_msg(format!("Text found: {}", text)))
}

pub async fn wait_for_url(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let pattern = args.get_str("pattern", "wait_for_url")?;
    client.wait_for_url(pattern).await?;
    Ok(ok_msg(format!("URL pattern matched: {}", pattern)))
}

pub async fn wait_for_load(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let state = args.get_str("state", "wait_for_load")?;
    client.wait_for_load(state).await?;
    Ok(ok_msg(format!("Load state reached: {}", state)))
}

pub async fn wait_for_download(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let path = args.get_opt_str("path");
    let download_path = client.wait_for_download(path).await?;
    Ok(serde_json::json!({
        "success": true,
        "path": download_path,
        "message": "Download completed"
    }))
}

pub async fn wait_for_fn(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let js = args.get_str("js", "wait_for_fn")?;
    client.wait_for_fn(js).await?;
    Ok(ok_msg("JavaScript condition met"))
}

pub async fn wait_for_state(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "wait_for_state")?;
    let state = args.get_str("state", "wait_for_state")?;
    client.wait_for_state(selector, state).await?;
    Ok(ok_msg(format!(
        "Element {} reached state: {}",
        selector, state
    )))
}
