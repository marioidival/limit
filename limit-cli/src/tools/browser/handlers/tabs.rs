use super::super::args::ArgsExt;
use super::super::client::BrowserClient;
use super::super::client_ext::TabsExt;
use super::super::response::ok_msg;
use limit_agent::error::AgentError;
use serde_json::Value;

pub async fn tab_list(client: &BrowserClient, _args: &Value) -> Result<Value, AgentError> {
    let tabs = client.tab_list().await?;

    let tabs_json: Vec<Value> = tabs
        .iter()
        .map(|t| {
            serde_json::json!({
                "index": t.index,
                "title": t.title
            })
        })
        .collect();

    Ok(serde_json::json!({
        "success": true,
        "tabs": tabs_json,
        "count": tabs.len()
    }))
}

pub async fn tab_new(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let url = args.get_opt_str("url");
    client.tab_new(url).await?;
    Ok(ok_msg("Opened new tab"))
}

pub async fn tab_close(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let index = args.get_opt_u64("index").map(|i| i as usize);
    client.tab_close(index).await?;
    Ok(ok_msg("Tab closed"))
}

pub async fn tab_select(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let index = args.get_u64("index", "tab_select")? as usize;
    client.tab_select(index).await?;
    Ok(ok_msg(format!("Switched to tab {}", index)))
}
