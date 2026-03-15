use super::super::args::ArgsExt;
use super::super::client::BrowserClient;
use super::super::client_ext::QueryExt;
use limit_agent::error::AgentError;
use serde_json::Value;

pub async fn get(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let what = args.get_str("get_what", "get")?;
    let content = client.get(what).await?;
    Ok(serde_json::json!({
        "success": true,
        "content": content
    }))
}

pub async fn get_attr(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "get_attr")?;
    let attr = args.get_str("attr", "get_attr")?;
    let value = client.get_attr(selector, attr).await?;
    Ok(serde_json::json!({
        "success": true,
        "value": value,
        "message": format!("Attribute '{}' = '{}'", attr, value)
    }))
}

pub async fn get_count(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "get_count")?;
    let count = client.get_count(selector).await?;
    Ok(serde_json::json!({
        "success": true,
        "count": count,
        "message": format!("Found {} elements matching {}", count, selector)
    }))
}

pub async fn get_box(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "get_box")?;
    let bbox = client.get_box(selector).await?;
    Ok(serde_json::json!({
        "success": true,
        "x": bbox.x,
        "y": bbox.y,
        "width": bbox.width,
        "height": bbox.height,
        "message": format!("Element bounding box: ({}, {}) {}x{}", bbox.x, bbox.y, bbox.width, bbox.height)
    }))
}

pub async fn get_styles(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let selector = args.get_str("selector", "get_styles")?;
    let styles = client.get_styles(selector).await?;
    Ok(serde_json::json!({
        "success": true,
        "styles": styles,
        "message": format!("Got {} computed styles for {}", styles.len(), selector)
    }))
}

pub async fn eval(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let script = args.get_str("script", "eval")?;
    let result = client.eval(script).await?;
    Ok(serde_json::json!({
        "success": true,
        "result": result
    }))
}
