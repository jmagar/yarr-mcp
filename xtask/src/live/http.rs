use anyhow::{Result, bail};
use serde_json::{Value, json};
use ureq::Agent;

/// Agent that surfaces non-2xx responses as `Ok` instead of `Error::StatusCode`,
/// so callers can read the body of an error response (ureq 3.x default is to error).
fn lenient_agent() -> Agent {
    Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

pub fn get_text(url: &str) -> Result<(u16, String)> {
    let mut response = lenient_agent().get(url).call()?;
    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string()?;
    Ok((status, body))
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
    let mut request = ureq::post(format!("{base_url}/mcp"))
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json");
    if let Some(token) = bearer {
        request = request.header("authorization", &format!("Bearer {token}"));
    }
    let mut response = request.send_json(&body)?;
    let payload: Value = response.body_mut().read_json()?;
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
    let mut request = ureq::post(format!("{base_url}/mcp"))
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json");
    if let Some(token) = bearer {
        request = request.header("authorization", &format!("Bearer {token}"));
    }
    match request.send_json(&body) {
        Ok(response) => Ok(response.status().as_u16()),
        Err(ureq::Error::StatusCode(status)) => Ok(status),
        Err(err) => bail!(err),
    }
}

pub fn mcp_tool(base_url: &str, tool: &str, arguments: Value, id: u64) -> Result<Value> {
    let result = mcp(
        base_url,
        "tools/call",
        Some(json!({"name":tool,"arguments":arguments})),
        id,
    )?;
    let text = result["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("MCP tool did not return text content"))?;
    Ok(serde_json::from_str(text)?)
}

/// Call the single published MCP tool, `yarr`, with a Code Mode script and return
/// the parsed `{result, calls, logs, artifacts}` envelope. A script that throws is
/// still a successful MCP call — the error is captured in `result.__codemode_error`.
pub fn yarr(base_url: &str, code: &str, id: u64) -> Result<Value> {
    mcp_tool(base_url, "yarr", json!({ "code": code }), id)
}
