use anyhow::{Context, Result};
use serde_json::{Map, Value};

use crate::live::process;

/// Invoke `yarr <svc> op <name> --args <json> [--confirm]`. Returns the parsed
/// JSON result on a 2xx, `None` for an empty body, or an error with the upstream
/// message on a non-2xx / CLI error.
const CONTRACT_INVOKE_ATTEMPTS: usize = 3;

pub(super) fn invoke(
    yarr: &process::YarrProcess,
    svc: &str,
    name: &str,
    args: &Map<String, Value>,
    confirm: bool,
) -> Result<Option<Value>> {
    let mut last_error = None;
    for attempt in 1..=CONTRACT_INVOKE_ATTEMPTS {
        match invoke_once(yarr, svc, name, args, confirm) {
            Ok(value) => return Ok(value),
            Err(err) => {
                let detail = err.to_string();
                if attempt < CONTRACT_INVOKE_ATTEMPTS && is_retryable_contract_error(&detail) {
                    last_error = Some(detail);
                    std::thread::sleep(std::time::Duration::from_millis(750));
                    continue;
                }
                if attempt > 1 {
                    let prior = last_error
                        .map(|e| format!("; previous retryable error: {e}"))
                        .unwrap_or_default();
                    anyhow::bail!("after {attempt} attempts: {detail}{prior}");
                }
                return Err(err);
            }
        }
    }
    unreachable!("contract invoke loop always returns");
}

pub(in crate::live) fn is_retryable_contract_error(detail: &str) -> bool {
    detail.contains("request failed")
        || detail.contains("tcp connect error")
        || detail.contains("connection closed")
        || detail.contains("error sending request")
}

fn invoke_once(
    yarr: &process::YarrProcess,
    svc: &str,
    name: &str,
    args: &Map<String, Value>,
    confirm: bool,
) -> Result<Option<Value>> {
    let args_json = serde_json::to_string(args)?;
    let mut argv: Vec<&str> = vec![svc, "op", name, "--args", &args_json];
    if confirm {
        argv.push("--confirm");
    }
    let output = yarr.output(&argv)?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{}", err.trim().trim_start_matches("Error: "));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    // A non-empty 2xx body MUST parse as JSON. Swallowing a parse error here (the old
    // `.ok()`) made unparseable output masquerade as an empty body, silently SKIPPING
    // schema validation and counting as a clean pass — a false PASS. Surface it as a
    // failure with a preview of the offending output instead.
    let value: Option<Value> = match serde_json::from_str(trimmed) {
        Ok(v) => Some(v),
        Err(e) => anyhow::bail!(
            "non-empty 2xx body did not parse as JSON ({e}): {}",
            trimmed.chars().take(180).collect::<String>()
        ),
    };
    // `YarrClient` returns `{"ok":true,"status":<code>}` for an empty 2xx body
    // (204 etc.). That's a "no body" sentinel, not a response to validate against
    // the op's schema — treat it like an empty body so it counts as a clean 2xx.
    if let Some(Value::Object(m)) = &value
        && m.len() == 2
        && m.get("ok") == Some(&Value::Bool(true))
        && m.get("status").is_some_and(Value::is_number)
    {
        return Ok(None);
    }
    Ok(value)
}

pub(super) fn write_detail(svc: &str, results: &[super::OpResult]) -> Result<()> {
    let dir = std::path::Path::new("target/live-full");
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("contract-{svc}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(results)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
