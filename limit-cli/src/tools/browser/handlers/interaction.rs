use super::super::args::ArgsExt;
use super::super::client::BrowserClient;
use super::super::client_ext::{InteractionExt, QueryExt};
use super::super::response::ok_msg;
use limit_agent::error::AgentError;
use serde_json::Value;

pub async fn click(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "click")?;
    client.click(selector).await?;
    Ok(ok_msg(format!("Clicked element: {}", selector)))
}

pub async fn fill(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "fill")?;
    let text = args.get_str("text", "fill")?;
    client.fill(selector, text).await?;
    Ok(ok_msg(format!("Filled element {} with text", selector)))
}

pub async fn type_(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "type")?;
    let text = args.get_str("text", "type")?;
    client.type_text(selector, text).await?;
    Ok(ok_msg(format!("Typed text into element: {}", selector)))
}

pub async fn press(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let key = args.get_str("key", "press")?;
    client.press(key).await?;
    Ok(ok_msg(format!("Pressed key: {}", key)))
}

pub async fn hover(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "hover")?;
    client.hover(selector).await?;
    Ok(ok_msg(format!("Hovered over element: {}", selector)))
}

pub async fn select(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "select")?;
    let value = args.get_str("value", "select")?;
    client.select_option(selector, value).await?;
    Ok(ok_msg(format!(
        "Selected option '{}' in element: {}",
        value, selector
    )))
}

pub async fn dblclick(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "dblclick")?;
    client.dblclick(selector).await?;
    Ok(ok_msg(format!("Double-clicked element: {}", selector)))
}

pub async fn focus(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "focus")?;
    client.focus(selector).await?;
    Ok(ok_msg(format!("Focused element: {}", selector)))
}

pub async fn check(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "check")?;
    client.check(selector).await?;
    Ok(ok_msg(format!("Checked element: {}", selector)))
}

pub async fn uncheck(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "uncheck")?;
    client.uncheck(selector).await?;
    Ok(ok_msg(format!("Unchecked element: {}", selector)))
}

pub async fn scrollintoview(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "scrollintoview")?;
    client.scrollintoview(selector).await?;
    Ok(ok_msg(format!("Scrolled element into view: {}", selector)))
}

pub async fn drag(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let source = args.get_str("source", "drag")?;
    let target = args.get_str("target", "drag")?;
    client.drag(source, target).await?;
    Ok(ok_msg(format!("Dragged from {} to {}", source, target)))
}

pub async fn upload(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "upload")?;
    let files = args
        .get("files")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            AgentError::ToolError("Missing 'files' argument for upload action".to_string())
        })?;

    let file_paths: Vec<&str> = files.iter().filter_map(|f| f.as_str()).collect();

    if file_paths.is_empty() {
        return Err(AgentError::ToolError(
            "At least one file path required".to_string(),
        ));
    }

    client.upload(selector, &file_paths).await?;
    Ok(ok_msg(format!(
        "Uploaded {} file(s) to {}",
        file_paths.len(),
        selector
    )))
}

pub async fn pdf(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let path = args.get_str("path", "pdf")?;
    client.pdf(path).await?;
    Ok(ok_msg(format!("Saved page as PDF: {}", path)))
}
