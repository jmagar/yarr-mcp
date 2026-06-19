use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::{LIVE_PORT, assertions, live_base_url, matrix, process, report};

const MCPORTER_TIMEOUT_MS: &str = "20000";
const MCPORTER_PROCESS_TIMEOUT: Duration = Duration::from_secs(35);
pub(super) const FIXTURE_PORT: u16 = 40175;
pub(super) const SAB_FIXTURE_NZB: &str = r#"<?xml version="1.0" encoding="iso-8859-1" ?>
<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="rustarr@example.invalid" date="1710000000" subject="rustarr-live-test">
    <groups><group>alt.binaries.test</group></groups>
    <segments><segment bytes="1" number="1">rustarr-live-test@example.invalid</segment></segments>
  </file>
</nzb>
"#;

mod cases;
mod state;

pub(super) fn run(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
    matrix: &matrix::Matrix,
) -> Result<()> {
    ensure_mcporter()?;
    ensure_protected_credentials(rustarr, matrix)?;

    let base = live_base_url();
    let mcp_url = format!("{base}/mcp");
    let mut server = rustarr.start_server(LIVE_PORT)?;
    server.wait_healthy(&base)?;

    let discovered = discover_tools(&mcp_url)?;
    let matrix_services: BTreeSet<&str> = matrix
        .services
        .iter()
        .map(|service| service.name.as_str())
        .collect();
    let discovered_services: BTreeSet<&str> =
        discovered.iter().map(|tool| tool.name.as_str()).collect();
    if discovered_services != matrix_services {
        bail!(
            "mcporter discovered service tools {discovered_services:?}, expected matrix services {matrix_services:?}"
        );
    }
    report.pass(
        "mcporter list service tools",
        format!("{} service tools discovered", discovered.len()),
    );

    let mut total_calls = 0usize;
    for tool in &discovered {
        let service = matrix
            .services
            .iter()
            .find(|service| service.name == tool.name)
            .ok_or_else(|| anyhow::anyhow!("missing matrix service for tool {}", tool.name))?;
        if tool.actions.is_empty() {
            bail!("tool {} advertised no actions", tool.name);
        }

        for action in &tool.actions {
            let cases = cases::action_cases(service, action)?;
            if cases.is_empty() {
                bail!(
                    "no mcporter action case registered for {}.{action}",
                    tool.name
                );
            }
            for case in cases {
                total_calls += 1;
                drop(server);
                server = rustarr.start_server(LIVE_PORT)?;
                server.wait_healthy(&base)?;
                if action == "quality_profiles" {
                    restart_mcporter_daemon();
                }
                let outcome = call_tool(&mcp_url, &tool.name, &case.arguments)?;
                validate_outcome(&tool.name, action, &case, &outcome)?;
                report.pass(
                    format!("mcporter {} {}", tool.name, case.label),
                    case.pass_detail(&outcome),
                );
            }
        }
    }

    report.pass(
        "mcporter exhaustive service-action coverage",
        format!("{total_calls} mcporter tool calls covered advertised actions"),
    );

    state::run_confirmed_write_state_checks(report, &mcp_url, matrix)?;

    Ok(())
}

fn ensure_protected_credentials(
    rustarr: &process::RustarrProcess,
    matrix: &matrix::Matrix,
) -> Result<()> {
    let mut missing = Vec::new();
    for service in &matrix.services {
        match service.kind.as_str() {
            "jellyfin" => {
                if rustarr
                    .env
                    .get("RUSTARR_JELLYFIN_TOKEN")
                    .is_none_or(|value| value.trim().is_empty())
                {
                    missing.push("RUSTARR_JELLYFIN_TOKEN");
                }
            }
            "plex" => {
                if rustarr
                    .env
                    .get("RUSTARR_PLEX_TOKEN")
                    .is_none_or(|value| value.trim().is_empty())
                {
                    missing.push("RUSTARR_PLEX_TOKEN");
                }
            }
            "tracearr" => {
                if rustarr
                    .env
                    .get("RUSTARR_TRACEARR_TOKEN")
                    .is_none_or(|value| value.trim().is_empty())
                {
                    missing.push("RUSTARR_TRACEARR_TOKEN");
                }
            }
            _ => {}
        }
    }
    missing.sort_unstable();
    missing.dedup();
    if !missing.is_empty() {
        bail!(
            "mcporter exhaustive suite requires protected shart credentials; missing {} in /home/jmagar/.rustarr-shart/.env",
            missing.join(", ")
        );
    }
    Ok(())
}

#[derive(Debug)]
pub(super) struct ToolDef {
    name: String,
    actions: Vec<String>,
}

#[derive(Debug)]
pub(super) struct ActionCase {
    label: String,
    arguments: Value,
    expectation: OutcomeExpectation,
}

impl ActionCase {
    fn ok(
        label: impl Into<String>,
        arguments: Value,
        assertions: Vec<matrix::Expectation>,
    ) -> Self {
        Self {
            label: label.into(),
            arguments,
            expectation: OutcomeExpectation::Success(assertions),
        }
    }

    fn expected_error(label: impl Into<String>, arguments: Value, tokens: &[&str]) -> Self {
        Self {
            label: label.into(),
            arguments,
            expectation: OutcomeExpectation::ExpectedError(
                tokens.iter().map(|token| (*token).to_owned()).collect(),
            ),
        }
    }

    fn pass_detail(&self, outcome: &CallOutcome) -> String {
        match (&self.expectation, outcome) {
            (OutcomeExpectation::Success(_), CallOutcome::Success(value)) => {
                format!("success {} bytes", value.to_string().len())
            }
            (OutcomeExpectation::ExpectedError(_), CallOutcome::Success(value)) => {
                format!("non-mutating preview {} bytes", value.to_string().len())
            }
            (OutcomeExpectation::ExpectedError(_), CallOutcome::Failure(text)) => {
                format!("expected guard/error matched: {}", compact(text))
            }
            (OutcomeExpectation::Success(_), CallOutcome::Failure(text)) => {
                format!("unexpected failure: {}", compact(text))
            }
        }
    }
}

#[derive(Debug)]
pub(super) enum OutcomeExpectation {
    Success(Vec<matrix::Expectation>),
    ExpectedError(Vec<String>),
}

#[derive(Debug)]
pub(super) enum CallOutcome {
    Success(Value),
    Failure(String),
}

fn ensure_mcporter() -> Result<()> {
    let output = Command::new("mcporter")
        .arg("--version")
        .output()
        .context("mcporter not found in PATH")?;
    if !output.status.success() {
        bail!(
            "mcporter --version failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn discover_tools(mcp_url: &str) -> Result<Vec<ToolDef>> {
    let output = Command::new("mcporter")
        .args([
            "list",
            "--http-url",
            mcp_url,
            "--allow-http",
            "--json",
            "--schema",
            "--timeout",
            MCPORTER_TIMEOUT_MS,
        ])
        .output()
        .context("failed to run mcporter list")?;
    if !output.status.success() {
        bail!(
            "mcporter list failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let json = parse_json_stdout(&output.stdout).context("failed to parse mcporter list JSON")?;
    let mut raw_tools = Vec::new();
    collect_tool_like_objects(&json, &mut raw_tools);

    let mut tools = Vec::new();
    let mut seen = BTreeSet::new();
    for tool in raw_tools {
        let Some(name) = tool.get("name").and_then(Value::as_str) else {
            continue;
        };
        if !seen.insert(name.to_owned()) {
            continue;
        }
        let actions = tool_actions(tool)?;
        tools.push(ToolDef {
            name: name.to_owned(),
            actions,
        });
    }
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    if tools.is_empty() {
        bail!("mcporter list did not expose any tool schemas: {json}");
    }
    Ok(tools)
}

fn collect_tool_like_objects<'a>(value: &'a Value, out: &mut Vec<&'a Map<String, Value>>) {
    match value {
        Value::Object(map) => {
            if map.get("name").and_then(Value::as_str).is_some() && map.get("inputSchema").is_some()
            {
                out.push(map);
            }
            for child in map.values() {
                collect_tool_like_objects(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_tool_like_objects(item, out);
            }
        }
        _ => {}
    }
}

fn tool_actions(tool: &Map<String, Value>) -> Result<Vec<String>> {
    let action_enum = tool
        .get("inputSchema")
        .and_then(|schema| schema.get("properties"))
        .and_then(|props| props.get("action"))
        .and_then(|action| action.get("enum"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("tool {} missing action enum", tool["name"]))?;
    let actions = action_enum
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow::anyhow!("non-string action enum value: {value}"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut seen = BTreeSet::new();
    let actions = actions
        .into_iter()
        .filter(|action| seen.insert(action.clone()))
        .collect();
    Ok(actions)
}

pub(super) fn call_tool(mcp_url: &str, tool: &str, arguments: &Value) -> Result<CallOutcome> {
    let args_json = serde_json::to_string(arguments)?;
    let args = [
        "call",
        "--http-url",
        mcp_url,
        "--allow-http",
        "--tool",
        tool,
        "--args",
        &args_json,
        "--timeout",
        MCPORTER_TIMEOUT_MS,
        "--output",
        "json",
    ];
    let output = match mcporter_output_with_timeout(&args)? {
        Some(output) => output,
        None => {
            restart_mcporter_daemon();
            mcporter_output_with_timeout(&args)?.ok_or_else(|| {
                anyhow::anyhow!("mcporter call for {tool} {args_json} timed out after retry")
            })?
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !output.status.success() {
        let combined = format!("{stdout}\n{stderr}");
        return Ok(CallOutcome::Failure(combined.trim().to_owned()));
    }
    let value = parse_json_stdout(&output.stdout).with_context(|| {
        format!("mcporter call for {tool} returned non-JSON stdout: {stdout}; stderr={stderr}")
    })?;
    if value.get("error").is_some() || value.get("kind").and_then(Value::as_str) == Some("error") {
        return Ok(CallOutcome::Failure(value.to_string()));
    }
    Ok(CallOutcome::Success(value))
}

fn mcporter_output_with_timeout(args: &[&str]) -> Result<Option<Output>> {
    let mut child = Command::new("mcporter")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to run mcporter {}", args.join(" ")))?;
    let deadline = Instant::now() + MCPORTER_PROCESS_TIMEOUT;
    while Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            return child
                .wait_with_output()
                .map(Some)
                .context("failed to collect mcporter output");
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    Ok(None)
}

fn restart_mcporter_daemon() {
    let _ = Command::new("mcporter")
        .args(["daemon", "restart"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn validate_outcome(
    tool: &str,
    action: &str,
    case: &ActionCase,
    outcome: &CallOutcome,
) -> Result<()> {
    match (&case.expectation, outcome) {
        (OutcomeExpectation::Success(assertions), CallOutcome::Success(value)) => {
            for assertion in assertions {
                assertions::assert_value(value, assertion)
                    .with_context(|| format!("semantic assertion failed for {tool}.{action}"))?;
            }
            Ok(())
        }
        (OutcomeExpectation::Success(_), CallOutcome::Failure(text)) => {
            bail!("mcporter {tool}.{action} expected success, got {text}")
        }
        (OutcomeExpectation::ExpectedError(tokens), CallOutcome::Failure(text)) => {
            assertions::assert_expected_error(text, tokens)
        }
        (OutcomeExpectation::ExpectedError(tokens), CallOutcome::Success(value)) => {
            let text = value.to_string();
            assertions::assert_expected_error(&text, tokens)
                .with_context(|| format!("expected guard/error-ish preview for {tool}.{action}"))
        }
    }
}

fn parse_json_stdout(stdout: &[u8]) -> Result<Value> {
    serde_json::from_slice(stdout).or_else(|_| {
        let text = String::from_utf8_lossy(stdout);
        let start = text
            .find(|ch| ['{', '[', '"'].contains(&ch))
            .ok_or_else(|| anyhow::anyhow!("stdout did not contain JSON: {text}"))?;
        serde_json::from_str(&text[start..]).context("failed to parse JSON substring from stdout")
    })
}

pub(super) fn compact(text: &str) -> String {
    let mut out = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.len() > 180 {
        out.truncate(180);
        out.push_str("...");
    }
    out
}
