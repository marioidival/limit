use super::super::args::ArgsExt;
use super::super::client::BrowserClient;
use super::super::client_ext::StorageExt;
use super::super::response::ok_msg;
use limit_agent::error::AgentError;
use serde_json::Value;

pub async fn cookies(client: &BrowserClient, _args: &Value) -> Result<Value, AgentError> {
    let cookies = client.cookies().await?;

    let cookies_json: Vec<Value> = cookies
        .iter()
        .map(|c| {
            serde_json::json!({
                "name": c.name,
                "value": c.value
            })
        })
        .collect();

    Ok(serde_json::json!({
        "success": true,
        "cookies": cookies_json,
        "count": cookies.len()
    }))
}

pub async fn cookies_set(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let name = args.get_str("name", "cookies_set")?;
    let value = args.get_str("value", "cookies_set")?;
    client.cookies_set(name, value).await?;
    Ok(ok_msg(format!("Set cookie: {}={}", name, value)))
}

pub async fn storage_get(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let storage_type = args.get_str("storage_type", "storage_get")?;
    let key = args.get_opt_str("key");
    let value = client.storage_get(storage_type, key).await?;
    Ok(serde_json::json!({
        "success": true,
        "value": value
    }))
}

pub async fn storage_set(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let storage_type = args.get_str("storage_type", "storage_set")?;
    let key = args.get_str("key", "storage_set")?;
    let value = args.get_str("value", "storage_set")?;
    client.storage_set(storage_type, key, value).await?;
    Ok(ok_msg(format!(
        "Set {} storage: {}={}",
        storage_type, key, value
    )))
}

pub async fn network_requests(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let filter = args.get_opt_str("filter");
    let requests = client.network_requests(filter).await?;

    let requests_json: Vec<Value> = requests
        .iter()
        .map(|r| {
            serde_json::json!({
                "method": r.method,
                "url": r.url
            })
        })
        .collect();

    Ok(serde_json::json!({
        "success": true,
        "requests": requests_json,
        "count": requests.len()
    }))
}
