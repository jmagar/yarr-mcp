use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::process::Command;

use super::{LIVE_PORT, assertions, live_base_url, matrix, process, report};

const MCPORTER_TIMEOUT_MS: &str = "20000";

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
            let cases = action_cases(service, action)?;
            if cases.is_empty() {
                bail!(
                    "no mcporter action case registered for {}.{action}",
                    tool.name
                );
            }
            for case in cases {
                total_calls += 1;
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
struct ToolDef {
    name: String,
    actions: Vec<String>,
}

#[derive(Debug)]
struct ActionCase {
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
enum OutcomeExpectation {
    Success(Vec<matrix::Expectation>),
    ExpectedError(Vec<String>),
}

#[derive(Debug)]
enum CallOutcome {
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
    let mut actions = action_enum
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow::anyhow!("non-string action enum value: {value}"))
        })
        .collect::<Result<Vec<_>>>()?;
    actions.sort();
    actions.dedup();
    Ok(actions)
}

fn action_cases(service: &matrix::ServiceCase, action: &str) -> Result<Vec<ActionCase>> {
    let mut cases = Vec::new();
    match action {
        "integrations" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_path_contains("supported", &service.name)],
            ));
        }
        "help" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_path_contains("help", "api_get")],
            ));
        }
        "service_status" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![service.status.clone()],
            ));
        }
        "api_get" => {
            for get in &service.get {
                cases.push(ActionCase::ok(
                    format!("api_get {}", get.path),
                    json!({ "action": "api_get", "path": get.path }),
                    vec![get.expectation.clone()],
                ));
            }
        }
        "api_post" | "api_put" | "api_delete" => {
            let mut args = json!({
                "action": action,
                "path": service.post_blocked.path,
                "confirm": false,
            });
            if action != "api_delete" {
                args["body"] = service.post_blocked.body.clone();
            }
            cases.push(ActionCase::expected_error(
                action,
                args,
                &["confirm=true", "confirm", "execution_error", action],
            ));
        }
        "quality_profiles" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_type("array")],
            ));
        }
        "list" | "rootfolders" | "health" | "indexers" | "download_queue" | "media_sessions"
        | "media_libraries" | "stats_activity" | "stats_users" | "stats_libraries" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_type("array_or_object")],
            ));
        }
        "wanted" | "queue" | "history" | "indexer_stats" | "requests" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_type("object")],
            ));
        }
        "indexer_search" | "media_search" | "request_search" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action, "query": "star" }),
                vec![expect_type("array_or_object")],
            ));
        }
        "stats_history" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action, "start": 0, "length": 1 }),
                vec![expect_type("array_or_object")],
            ));
        }
        "set_quality" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({
                    "action": action,
                    "to": "__rustarr_live_missing_profile__",
                    "confirm": false
                }),
                &[
                    "quality profile",
                    "available profiles",
                    "confirm_required",
                    "confirm",
                    "execution_error",
                    action,
                ],
            ));
        }
        "search" | "refresh" | "monitor" | "unmonitor" | "indexer_test" | "media_scan"
        | "download_pause" | "download_resume" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({ "action": action, "confirm": false }),
                &[
                    "confirm=true",
                    "confirm_required",
                    "confirm",
                    "execution_error",
                    action,
                ],
            ));
        }
        "add" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({
                    "action": action,
                    "term": "__rustarr_live_missing_title__",
                    "quality_profile": "__rustarr_live_missing_profile__",
                    "root_folder": "/__rustarr_live_missing_root__",
                    "confirm": false
                }),
                &[
                    "quality profile",
                    "lookup",
                    "confirm_required",
                    "confirm",
                    "execution_error",
                    action,
                ],
            ));
        }
        "delete" | "request_approve" | "request_decline" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({ "action": action, "id": "1", "confirm": false }),
                &[
                    "confirm=true",
                    "confirm_required",
                    "confirm",
                    "execution_error",
                    action,
                ],
            ));
        }
        "download_add" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({
                    "action": action,
                    "url": "magnet:?xt=urn:btih:0000000000000000000000000000000000000000",
                    "confirm": false
                }),
                &["confirm=true", "confirm", "execution_error", action],
            ));
        }
        "download_remove" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({
                    "action": action,
                    "id": "__rustarr_live_missing_download__",
                    "delete_files": false,
                    "confirm": false
                }),
                &["confirm=true", "confirm", "execution_error", action],
            ));
        }
        "request_create" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({
                    "action": action,
                    "media_type": "movie",
                    "media_id": 603,
                    "confirm": false
                }),
                &["confirm=true", "confirm", "execution_error", action],
            ));
        }
        other => bail!(
            "action {other} is advertised for {} but xtask has no safe mcporter fixture",
            service.name
        ),
    }
    Ok(cases)
}

fn call_tool(mcp_url: &str, tool: &str, arguments: &Value) -> Result<CallOutcome> {
    let args_json = serde_json::to_string(arguments)?;
    let output = Command::new("mcporter")
        .args([
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
        ])
        .output()
        .with_context(|| format!("failed to run mcporter call for {tool} {args_json}"))?;
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

fn expect_type(value_type: &str) -> matrix::Expectation {
    matrix::Expectation {
        json_path: None,
        equals: None,
        equals_any: None,
        value_type: Some(value_type.to_owned()),
        contains: None,
        xml_root: None,
    }
}

fn expect_path_contains(path: &str, needle: &str) -> matrix::Expectation {
    matrix::Expectation {
        json_path: Some(path.to_owned()),
        equals: None,
        equals_any: None,
        value_type: None,
        contains: Some(needle.to_owned()),
        xml_root: None,
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

fn compact(text: &str) -> String {
    let mut out = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.len() > 180 {
        out.truncate(180);
        out.push_str("...");
    }
    out
}
