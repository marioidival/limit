use super::super::args::ArgsExt;
use super::super::client::BrowserClient;
use super::super::client_ext::TabsExt;
use super::super::response::ok_msg;
use limit_agent::error::AgentError;
use serde_json::Value;

pub async fn dialog_accept(client: &BrowserClient, args: &Value) -> Result<Value, AgentError> {
    let text = args.get_opt_str("text");
    client.dialog_accept(text).await?;
    Ok(ok_msg("Dialog accepted"))
}

pub async fn dialog_dismiss(client: &BrowserClient, _args: &Value) -> Result<Value, AgentError> {
    client.dialog_dismiss().await?;
    Ok(ok_msg("Dialog dismissed"))
}
