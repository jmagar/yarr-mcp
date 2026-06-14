use anyhow::{bail, Result};
use serde_json::{json, Value};

pub fn get_text(url: &str) -> Result<(u16, String)> {
    match ureq::get(url).call() {
        Ok(response) => Ok((response.status(), response.into_string()?)),
        Err(ureq::Error::Status(status, response)) => Ok((status, response.into_string()?)),
        Err(err) => bail!(err),
    }
}

pub fn mcp(base_url: &str, method: &str, params: Option<Value>, id: u64) -> Result<Value> {
    mcp_with_auth(base_url, method, params, id, None)
}

pub fn mcp_with_auth(
    base_url: &str,
    method: &str,
    params: Option<Value>,
    id: u64,
    bearer: Option<&str>,
) -> Result<Value> {
    let mut body = json!({"jsonrpc":"2.0","id":id,"method":method});
    if let Some(params) = params {
        body["params"] = params;
    }
    let mut request = ureq::post(&format!("{base_url}/mcp"))
        .set("accept", "application/json, text/event-stream")
        .set("content-type", "application/json");
    if let Some(token) = bearer {
        request = request.set("authorization", &format!("Bearer {token}"));
    }
    let response = request.send_json(body)?;
    let payload: Value = response.into_json()?;
    if let Some(error) = payload.get("error") {
        bail!("{error}");
    }
    Ok(payload["result"].clone())
}

pub fn mcp_status(
    base_url: &str,
    method: &str,
    params: Option<Value>,
    bearer: Option<&str>,
) -> Result<u16> {
    let mut body = json!({"jsonrpc":"2.0","id":9999,"method":method});
    if let Some(params) = params {
        body["params"] = params;
    }
    let mut request = ureq::post(&format!("{base_url}/mcp"))
        .set("accept", "application/json, text/event-stream")
        .set("content-type", "application/json");
    if let Some(token) = bearer {
        request = request.set("authorization", &format!("Bearer {token}"));
    }
    match request.send_json(body) {
        Ok(response) => Ok(response.status()),
        Err(ureq::Error::Status(status, _)) => Ok(status),
        Err(err) => bail!(err),
    }
}

pub fn mcp_tool(base_url: &str, arguments: Value, id: u64) -> Result<Value> {
    let result = mcp(
        base_url,
        "tools/call",
        Some(json!({"name":"rustarr","arguments":arguments})),
        id,
    )?;
    let text = result["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("MCP tool did not return text content"))?;
    Ok(serde_json::from_str(text)?)
}
