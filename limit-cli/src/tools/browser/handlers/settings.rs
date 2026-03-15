use super::super::args::ArgsExt;
use super::super::client::BrowserClient;
use super::super::client_ext::StorageExt;
use super::super::response::ok_msg;
use limit_agent::error::AgentError;
use serde_json::Value;

pub async fn set_viewport(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let width = args.get_u64("width", "set_viewport")? as u32;
    let height = args.get_u64("height", "set_viewport")? as u32;
    let scale = args.get_opt_f64("scale").map(|s| s as f32);
    client.set_viewport(width, height, scale).await?;
    Ok(ok_msg(format!("Set viewport to {}x{}", width, height)))
}

pub async fn set_device(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let name = args.get_str("name", "set_device")?;
    client.set_device(name).await?;
    Ok(ok_msg(format!("Set device to: {}", name)))
}

pub async fn set_geo(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let latitude = args.get_f64("latitude", "set_geo")?;
    let longitude = args.get_f64("longitude", "set_geo")?;
    client.set_geo(latitude, longitude).await?;
    Ok(ok_msg(format!(
        "Set geolocation to ({}, {})",
        latitude, longitude
    )))
}
