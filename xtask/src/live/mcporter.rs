use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::{LIVE_PORT, assertions, live_base_url, matrix, process, report};

const MCPORTER_TIMEOUT_MS: &str = "20000";
const MCPORTER_PROCESS_TIMEOUT: Duration = Duration::from_secs(35);
const FIXTURE_PORT: u16 = 40175;
const SAB_FIXTURE_NZB: &str = r#"<?xml version="1.0" encoding="iso-8859-1" ?>
<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="rustarr@example.invalid" date="1710000000" subject="rustarr-live-test">
    <groups><group>alt.binaries.test</group></groups>
    <segments><segment bytes="1" number="1">rustarr-live-test@example.invalid</segment></segments>
  </file>
</nzb>
"#;

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

    run_confirmed_write_state_checks(report, &mcp_url, matrix)?;

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
        "list" | "rootfolders" | "health" | "download_queue" | "media_sessions"
        | "stats_activity" | "stats_users" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_type("array_or_object")],
            ));
        }
        "indexers" => {
            let assertions = if service.name == "prowlarr" {
                vec![matrix::Expectation {
                    json_path: None,
                    equals: None,
                    equals_any: None,
                    value_type: Some("array".into()),
                    contains: Some("Rustarr Live LinuxTracker".into()),
                    xml_root: None,
                }]
            } else {
                vec![expect_type("array_or_object")]
            };
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                assertions,
            ));
        }
        "media_libraries" => {
            let assertions = match service.name.as_str() {
                "plex" | "jellyfin" => vec![matrix::Expectation {
                    json_path: Some("libraries".into()),
                    equals: None,
                    equals_any: None,
                    value_type: Some("array".into()),
                    contains: Some("Rustarr Live Movies".into()),
                    xml_root: None,
                }],
                _ => vec![expect_type("array_or_object")],
            };
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                assertions,
            ));
        }
        "stats_libraries" => {
            let assertions = if service.name == "tautulli" {
                vec![matrix::Expectation {
                    json_path: None,
                    equals: None,
                    equals_any: None,
                    value_type: Some("array".into()),
                    contains: Some("Rustarr Live Movies".into()),
                    xml_root: None,
                }]
            } else {
                vec![expect_type("array_or_object")]
            };
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                assertions,
            ));
        }
        "wanted" | "queue" | "history" | "indexer_stats" | "requests" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_type("object")],
            ));
        }
        "indexer_search" => {
            let (query, mut payload, assertions) = if service.name == "prowlarr" {
                (
                    "ubuntu",
                    json!({ "action": action, "query": "ubuntu", "ids": [1] }),
                    vec![matrix::Expectation {
                        json_path: None,
                        equals: None,
                        equals_any: None,
                        value_type: Some("array".into()),
                        contains: Some("Rustarr Live LinuxTracker".into()),
                        xml_root: None,
                    }],
                )
            } else {
                (
                    "star",
                    json!({ "action": action, "query": "star" }),
                    vec![expect_type("array_or_object")],
                )
            };
            payload["query"] = json!(query);
            cases.push(ActionCase::ok(action, payload, assertions));
        }
        "request_search" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action, "query": "star" }),
                vec![expect_type("array_or_object")],
            ));
        }
        "media_search" => {
            let (query, assertions) = match service.name.as_str() {
                "plex" | "jellyfin" => (
                    "Rustarr",
                    vec![matrix::Expectation {
                        json_path: Some("results".into()),
                        equals: None,
                        equals_any: None,
                        value_type: Some("array".into()),
                        contains: Some("Rustarr Fixture Movie".into()),
                        xml_root: None,
                    }],
                ),
                _ => ("star", vec![expect_type("array_or_object")]),
            };
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action, "query": query }),
                assertions,
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
            "action {other} is advertised for {} but xtask has no stateful mcporter test case",
            service.name
        ),
    }
    Ok(cases)
}

fn run_confirmed_write_state_checks(
    report: &mut report::Report,
    mcp_url: &str,
    matrix: &matrix::Matrix,
) -> Result<()> {
    let services: BTreeSet<&str> = matrix
        .services
        .iter()
        .map(|service| service.name.as_str())
        .collect();

    for (service, prefix) in [
        ("sonarr", "/api/v3"),
        ("radarr", "/api/v3"),
        ("prowlarr", "/api/v1"),
    ] {
        if services.contains(service) {
            run_tag_lifecycle(report, mcp_url, service, prefix)?;
        }
    }

    run_confirmed_generic_error_checks(report, mcp_url, matrix)?;

    if services.contains("sonarr") {
        run_arr_item_lifecycle(
            report,
            mcp_url,
            "sonarr",
            "/api/v3",
            "Firefly",
            "/data/rustarr-live-sonarr",
        )?;
    }

    if services.contains("radarr") {
        run_arr_item_lifecycle(
            report,
            mcp_url,
            "radarr",
            "/api/v3",
            "The Matrix",
            "/data/rustarr-live-radarr",
        )?;
    }

    if services.contains("prowlarr") {
        run_prowlarr_indexer_test(report, mcp_url)?;
    }

    if services.contains("overseerr") {
        run_overseerr_request_lifecycle(report, mcp_url)?;
    }

    if services.contains("jellyfin") {
        run_jellyfin_scan(report, mcp_url)?;
    }

    if services.contains("plex") {
        run_plex_scan_error(report, mcp_url)?;
    }

    if services.contains("sabnzbd") {
        let fixture = start_fixture_server()?;
        run_sabnzbd_lifecycle(report, mcp_url, &fixture)?;
    }

    if services.contains("qbittorrent") {
        run_qbittorrent_lifecycle(report, mcp_url)?;
    }

    Ok(())
}

fn run_confirmed_generic_error_checks(
    report: &mut report::Report,
    mcp_url: &str,
    matrix: &matrix::Matrix,
) -> Result<()> {
    for service in &matrix.services {
        for action in ["api_post", "api_put", "api_delete"] {
            let mut args = json!({
                "action": action,
                "path": service.post_expected_error.path,
                "confirm": true,
            });
            if action != "api_delete" {
                args["body"] = service.post_expected_error.body.clone();
            }
            let outcome = call_tool(mcp_url, &service.name, &args)?;
            assert_confirmed_generic_error(&service.name, action, &outcome)?;
            report.pass(
                format!("mcporter confirmed generic error {} {action}", service.name),
                "confirm=true reached upstream and returned the expected service error shape",
            );
        }
    }
    Ok(())
}

fn assert_confirmed_generic_error(
    service: &str,
    action: &str,
    outcome: &CallOutcome,
) -> Result<()> {
    let text = match outcome {
        CallOutcome::Success(value) => {
            if is_service_error_value(value) {
                return Ok(());
            }
            let text = value.to_string();
            if is_service_error_text(&text) {
                return Ok(());
            }
            text
        }
        CallOutcome::Failure(text) => text.clone(),
    };
    if is_service_error_text(&text) {
        return Ok(());
    }
    if text.contains("confirm=true") {
        bail!(
            "{service}.{action} confirmed generic call hit confirm guard instead of upstream: {text}"
        );
    }
    assertions::assert_expected_error(&text, &["execution_error".into(), action.into()])
        .with_context(|| format!("{service}.{action} confirmed generic error shape mismatch"))
}

fn is_service_error_value(value: &Value) -> bool {
    value.get("status").and_then(Value::as_bool) == Some(false) || value.get("error").is_some()
}

fn is_service_error_text(text: &str) -> bool {
    if text.contains("\"status\":false") || text.contains("\"error\"") {
        return true;
    }
    serde_json::from_str::<Value>(text)
        .map(|value| is_service_error_value(&value))
        .unwrap_or(false)
}

fn run_prowlarr_indexer_test(report: &mut report::Report, mcp_url: &str) -> Result<()> {
    let value = expect_success(
        "prowlarr",
        "indexer_test",
        call_tool(
            mcp_url,
            "prowlarr",
            &json!({ "action": "indexer_test", "confirm": true }),
        )?,
    )?;
    if !value.is_array() {
        bail!("prowlarr indexer_test confirmed call did not return array: {value}");
    }
    let count = value.as_array().map_or(0, Vec::len);
    if count == 0 {
        bail!("prowlarr indexer_test returned no indexer results; expected seeded LinuxTracker");
    }
    let tested_seed = value.as_array().is_some_and(|rows| {
        rows.iter().any(|row| {
            row.get("id").and_then(Value::as_i64) == Some(1)
                && row.get("isValid").and_then(Value::as_bool) == Some(true)
        })
    });
    if !tested_seed {
        bail!("prowlarr indexer_test did not validate seeded LinuxTracker id=1: {value}");
    }
    report.pass(
        "mcporter confirmed write prowlarr indexer_test",
        format!("test-all accepted by upstream; {count} indexer result(s) returned"),
    );
    Ok(())
}

fn run_jellyfin_scan(report: &mut report::Report, mcp_url: &str) -> Result<()> {
    let value = expect_success(
        "jellyfin",
        "media_scan",
        call_tool(
            mcp_url,
            "jellyfin",
            &json!({ "action": "media_scan", "confirm": true }),
        )?,
    )?;
    if value.get("ok").and_then(Value::as_bool) != Some(true)
        || value.get("status").and_then(Value::as_i64) != Some(204)
    {
        bail!("jellyfin media_scan did not report ok=true status=204: {value}");
    }
    report.pass(
        "mcporter confirmed write jellyfin media_scan",
        "server-wide library refresh returned 204",
    );
    Ok(())
}

fn run_plex_scan_error(report: &mut report::Report, mcp_url: &str) -> Result<()> {
    let libraries = expect_success(
        "plex",
        "media_libraries",
        call_tool(mcp_url, "plex", &json!({ "action": "media_libraries" }))?,
    )?;
    let maybe_library = libraries
        .get("libraries")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("key"))
        .and_then(Value::as_str);
    if let Some(library) = maybe_library {
        let value = expect_success(
            "plex",
            "media_scan",
            call_tool(
                mcp_url,
                "plex",
                &json!({ "action": "media_scan", "library": library, "confirm": true }),
            )?,
        )?;
        report.pass(
            "mcporter confirmed write plex media_scan",
            format!(
                "library {library} refresh accepted: {} bytes",
                value.to_string().len()
            ),
        );
    } else {
        let outcome = call_tool(
            mcp_url,
            "plex",
            &json!({ "action": "media_scan", "confirm": true }),
        )?;
        match outcome {
            CallOutcome::Failure(text) => {
                assertions::assert_expected_error(
                    &text,
                    &["library".into(), "requires".into(), "plex".into()],
                )?;
            }
            CallOutcome::Success(value) => {
                bail!("plex media_scan without fixture library unexpectedly succeeded: {value}");
            }
        }
        report.pass(
            "mcporter confirmed write plex media_scan fixture-missing error",
            "confirmed call produced the expected missing-library error; shart Plex has no libraries",
        );
    }
    Ok(())
}

fn run_overseerr_request_lifecycle(report: &mut report::Report, mcp_url: &str) -> Result<()> {
    cleanup_overseerr_requests(mcp_url)?;
    let created = expect_success(
        "overseerr",
        "request_create",
        call_tool(
            mcp_url,
            "overseerr",
            &json!({
                "action": "request_create",
                "media_type": "movie",
                "media_id": 603,
                "confirm": true,
            }),
        )?,
    )?;
    let id = created
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("overseerr request_create did not return id: {created}"))?;
    assert_overseerr_request_present(mcp_url, id, None)?;

    let approved = expect_success(
        "overseerr",
        "request_approve",
        call_tool(
            mcp_url,
            "overseerr",
            &json!({ "action": "request_approve", "id": id.to_string(), "confirm": true }),
        )?,
    )?;
    assert_eq_i64(&approved, "id", id)?;
    assert_overseerr_request_present(mcp_url, id, Some(2))?;

    let declined = expect_success(
        "overseerr",
        "request_decline",
        call_tool(
            mcp_url,
            "overseerr",
            &json!({ "action": "request_decline", "id": id.to_string(), "confirm": true }),
        )?,
    )?;
    assert_eq_i64(&declined, "id", id)?;
    assert_overseerr_request_present(mcp_url, id, Some(3))?;

    let deleted = expect_success(
        "overseerr",
        "api_delete request cleanup",
        call_tool(
            mcp_url,
            "overseerr",
            &json!({
                "action": "api_delete",
                "path": format!("/api/v1/request/{id}"),
                "confirm": true,
            }),
        )?,
    )?;
    if deleted.get("ok").and_then(Value::as_bool) != Some(true) {
        bail!("overseerr request cleanup did not report ok=true: {deleted}");
    }
    assert_overseerr_request_absent(mcp_url, id)?;

    report.pass(
        "mcporter confirmed write overseerr request lifecycle",
        "request_create/request_approve/request_decline changed observable request state and cleaned up",
    );
    Ok(())
}

fn cleanup_overseerr_requests(mcp_url: &str) -> Result<()> {
    let requests = overseerr_requests(mcp_url)?;
    for request in request_rows(&requests) {
        let Some(id) = request.get("id").and_then(Value::as_i64) else {
            continue;
        };
        let _ = call_tool(
            mcp_url,
            "overseerr",
            &json!({
                "action": "api_delete",
                "path": format!("/api/v1/request/{id}"),
                "confirm": true,
            }),
        )?;
    }
    Ok(())
}

fn assert_overseerr_request_present(mcp_url: &str, id: i64, status: Option<i64>) -> Result<()> {
    let requests = overseerr_requests(mcp_url)?;
    let request = request_rows(&requests)
        .find(|request| request.get("id").and_then(Value::as_i64) == Some(id))
        .ok_or_else(|| anyhow::anyhow!("overseerr requests did not contain id {id}: {requests}"))?;
    if let Some(status) = status {
        assert_eq_i64(request, "status", status)?;
    }
    Ok(())
}

fn assert_overseerr_request_absent(mcp_url: &str, id: i64) -> Result<()> {
    let requests = overseerr_requests(mcp_url)?;
    if request_rows(&requests).any(|request| request.get("id").and_then(Value::as_i64) == Some(id))
    {
        bail!("overseerr requests still contained id {id}: {requests}");
    }
    Ok(())
}

fn overseerr_requests(mcp_url: &str) -> Result<Value> {
    expect_success(
        "overseerr",
        "requests",
        call_tool(mcp_url, "overseerr", &json!({ "action": "requests" }))?,
    )
}

fn request_rows(value: &Value) -> impl Iterator<Item = &Value> {
    value
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn run_tag_lifecycle(
    report: &mut report::Report,
    mcp_url: &str,
    service: &str,
    api_prefix: &str,
) -> Result<()> {
    let label = format!("rustarr-live-mcporter-{service}-{}", std::process::id());
    cleanup_matching_tags(mcp_url, service, api_prefix, "rustarr-live-mcporter-")?;

    let create_args = json!({
        "action": "api_post",
        "path": format!("{api_prefix}/tag"),
        "body": { "label": label },
        "confirm": true,
    });
    let created = expect_success(
        service,
        "api_post tag create",
        call_tool(mcp_url, service, &create_args)?,
    )?;
    let tag_id = created.get("id").and_then(Value::as_i64).ok_or_else(|| {
        anyhow::anyhow!("{service} tag create did not return numeric id: {created}")
    })?;
    assert_object_field_eq(&created, "label", &label)
        .with_context(|| format!("{service} tag create did not echo label"))?;

    let list_path = format!("{api_prefix}/tag");
    assert_tag_present(mcp_url, service, &list_path, &label)?;

    let updated_label = format!("{label}-updated");
    let put_args = json!({
        "action": "api_put",
        "path": format!("{api_prefix}/tag/{tag_id}"),
        "body": { "id": tag_id, "label": updated_label },
        "confirm": true,
    });
    let updated = expect_success(
        service,
        "api_put tag update",
        call_tool(mcp_url, service, &put_args)?,
    )?;
    assert_object_field_eq(&updated, "label", &updated_label)
        .with_context(|| format!("{service} tag update did not echo updated label"))?;
    assert_tag_present(mcp_url, service, &list_path, &updated_label)?;

    let delete_args = json!({
        "action": "api_delete",
        "path": format!("{api_prefix}/tag/{tag_id}"),
        "confirm": true,
    });
    let _ = expect_success(
        service,
        "api_delete tag delete",
        call_tool(mcp_url, service, &delete_args)?,
    )?;
    assert_tag_absent(mcp_url, service, &list_path, &updated_label)?;

    report.pass(
        format!("mcporter confirmed write lifecycle {service} tag"),
        "api_post/api_put/api_delete changed observable state and cleaned up",
    );
    Ok(())
}

fn cleanup_matching_tags(
    mcp_url: &str,
    service: &str,
    api_prefix: &str,
    label_prefix: &str,
) -> Result<()> {
    let list_path = format!("{api_prefix}/tag");
    let tags = expect_success(
        service,
        "api_get tag cleanup list",
        call_tool(
            mcp_url,
            service,
            &json!({ "action": "api_get", "path": list_path }),
        )?,
    )?;
    for tag in tags.as_array().into_iter().flatten() {
        let Some(label) = tag.get("label").and_then(Value::as_str) else {
            continue;
        };
        let Some(id) = tag.get("id").and_then(Value::as_i64) else {
            continue;
        };
        if label.starts_with(label_prefix) {
            let _ = call_tool(
                mcp_url,
                service,
                &json!({
                    "action": "api_delete",
                    "path": format!("{api_prefix}/tag/{id}"),
                    "confirm": true,
                }),
            )?;
        }
    }
    Ok(())
}

fn assert_tag_present(mcp_url: &str, service: &str, list_path: &str, label: &str) -> Result<()> {
    let tags = expect_success(
        service,
        "api_get tag present",
        call_tool(
            mcp_url,
            service,
            &json!({ "action": "api_get", "path": list_path }),
        )?,
    )?;
    if tag_labels(&tags).any(|candidate| candidate == label) {
        return Ok(());
    }
    bail!("{service} tag list did not contain `{label}` after confirmed write: {tags}");
}

fn assert_tag_absent(mcp_url: &str, service: &str, list_path: &str, label: &str) -> Result<()> {
    let tags = expect_success(
        service,
        "api_get tag absent",
        call_tool(
            mcp_url,
            service,
            &json!({ "action": "api_get", "path": list_path }),
        )?,
    )?;
    if tag_labels(&tags).any(|candidate| candidate == label) {
        bail!("{service} tag list still contained `{label}` after confirmed delete: {tags}");
    }
    Ok(())
}

fn tag_labels(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|tag| tag.get("label").and_then(Value::as_str))
}

fn run_arr_item_lifecycle(
    report: &mut report::Report,
    mcp_url: &str,
    service: &str,
    api_prefix: &str,
    term: &str,
    root_folder: &str,
) -> Result<()> {
    ensure_root_folder(mcp_url, service, api_prefix, root_folder)?;
    cleanup_arr_title(mcp_url, service, term)?;

    let added = expect_success(
        service,
        "add",
        call_tool(
            mcp_url,
            service,
            &json!({
                "action": "add",
                "term": term,
                "quality_profile": "Any",
                "root_folder": root_folder,
                "confirm": true,
            }),
        )?,
    )?;
    let item_id = added
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("{service} add did not return numeric id: {added}"))?;
    assert_arr_item_present(mcp_url, service, item_id, Some(term), None)?;

    expect_arr_editor_write(mcp_url, service, "unmonitor", item_id)?;
    assert_arr_item_present(mcp_url, service, item_id, Some(term), Some(false))?;

    expect_arr_editor_write(mcp_url, service, "monitor", item_id)?;
    assert_arr_item_present(mcp_url, service, item_id, Some(term), Some(true))?;

    expect_arr_editor_write_with_args(
        mcp_url,
        service,
        "set_quality",
        json!({ "to": "Any", "ids": [item_id], "confirm": true }),
    )?;
    assert_arr_item_present(mcp_url, service, item_id, Some(term), Some(true))?;

    for action in ["search", "refresh"] {
        let value = expect_success(
            service,
            action,
            call_tool(
                mcp_url,
                service,
                &json!({ "action": action, "ids": [item_id], "confirm": true }),
            )?,
        )?;
        if value.get("async").and_then(Value::as_bool) != Some(true) {
            bail!("{service} {action} did not report async=true: {value}");
        }
    }

    let deleted = expect_success(
        service,
        "delete",
        call_tool(
            mcp_url,
            service,
            &json!({
                "action": "delete",
                "id": item_id.to_string(),
                "delete_files": false,
                "confirm": true,
            }),
        )?,
    )?;
    if deleted.get("deleted").and_then(Value::as_i64) != Some(item_id) {
        bail!("{service} delete did not echo deleted id {item_id}: {deleted}");
    }
    assert_arr_item_absent(mcp_url, service, item_id)?;

    report.pass(
        format!("mcporter confirmed arr item lifecycle {service}"),
        "add/monitor/unmonitor/set_quality/search/refresh/delete changed observable item state and cleaned up",
    );
    Ok(())
}

fn ensure_root_folder(
    mcp_url: &str,
    service: &str,
    api_prefix: &str,
    root_folder: &str,
) -> Result<()> {
    let list_path = format!("{api_prefix}/rootfolder");
    let roots = expect_success(
        service,
        "api_get rootfolder",
        call_tool(
            mcp_url,
            service,
            &json!({ "action": "api_get", "path": list_path }),
        )?,
    )?;
    if root_folder_paths(&roots).any(|candidate| candidate == root_folder) {
        return Ok(());
    }

    let created = expect_success(
        service,
        "api_post rootfolder",
        call_tool(
            mcp_url,
            service,
            &json!({
                "action": "api_post",
                "path": format!("{api_prefix}/rootfolder"),
                "body": { "path": root_folder },
                "confirm": true,
            }),
        )?,
    )?;
    assert_object_field_eq(&created, "path", root_folder)
        .with_context(|| format!("{service} rootfolder create did not echo path"))?;
    Ok(())
}

fn root_folder_paths(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|root| root.get("path").and_then(Value::as_str))
}

fn cleanup_arr_title(mcp_url: &str, service: &str, title: &str) -> Result<()> {
    let items = arr_list(mcp_url, service)?;
    for item in items.as_array().into_iter().flatten() {
        let Some(candidate_title) = item.get("title").and_then(Value::as_str) else {
            continue;
        };
        let Some(id) = item.get("id").and_then(Value::as_i64) else {
            continue;
        };
        if candidate_title.eq_ignore_ascii_case(title) {
            let _ = call_tool(
                mcp_url,
                service,
                &json!({
                    "action": "delete",
                    "id": id.to_string(),
                    "delete_files": false,
                    "confirm": true,
                }),
            )?;
        }
    }
    Ok(())
}

fn expect_arr_editor_write(mcp_url: &str, service: &str, action: &str, id: i64) -> Result<Value> {
    expect_arr_editor_write_with_args(
        mcp_url,
        service,
        action,
        json!({ "ids": [id], "confirm": true }),
    )
}

fn expect_arr_editor_write_with_args(
    mcp_url: &str,
    service: &str,
    action: &str,
    mut args: Value,
) -> Result<Value> {
    args["action"] = json!(action);
    let value = expect_success(service, action, call_tool(mcp_url, service, &args)?)?;
    if value.get("changed").and_then(Value::as_i64).is_none()
        && value
            .get("upstream_count")
            .and_then(Value::as_i64)
            .is_none()
    {
        bail!("{service} {action} did not return an editor mutation summary: {value}");
    }
    Ok(value)
}

fn assert_arr_item_present(
    mcp_url: &str,
    service: &str,
    id: i64,
    title: Option<&str>,
    monitored: Option<bool>,
) -> Result<()> {
    let items = arr_list(mcp_url, service)?;
    let item = items
        .as_array()
        .into_iter()
        .flatten()
        .find(|item| item.get("id").and_then(Value::as_i64) == Some(id))
        .ok_or_else(|| anyhow::anyhow!("{service} list did not contain id {id}: {items}"))?;
    if let Some(title) = title {
        let actual = item.get("title").and_then(Value::as_str).unwrap_or("");
        if actual != title {
            bail!("{service} item {id} title mismatch: expected {title}, got {actual}");
        }
    }
    if let Some(monitored) = monitored
        && item.get("monitored").and_then(Value::as_bool) != Some(monitored)
    {
        bail!("{service} item {id} monitored state mismatch, expected {monitored}: {item}");
    }
    Ok(())
}

fn assert_arr_item_absent(mcp_url: &str, service: &str, id: i64) -> Result<()> {
    let items = arr_list(mcp_url, service)?;
    if items
        .as_array()
        .into_iter()
        .flatten()
        .any(|item| item.get("id").and_then(Value::as_i64) == Some(id))
    {
        bail!("{service} list still contained id {id} after delete: {items}");
    }
    Ok(())
}

fn arr_list(mcp_url: &str, service: &str) -> Result<Value> {
    expect_success(
        service,
        "list",
        call_tool(mcp_url, service, &json!({ "action": "list" }))?,
    )
}

struct FixtureServer {
    base_url: String,
}

fn start_fixture_server() -> Result<FixtureServer> {
    let host =
        std::env::var("RUSTARR_LIVE_FIXTURE_HOST").unwrap_or_else(|_| "100.88.16.79".to_string());
    let listener = TcpListener::bind(("0.0.0.0", FIXTURE_PORT)).with_context(|| {
        format!(
            "failed to bind live fixture server on 0.0.0.0:{FIXTURE_PORT}; \
             set RUSTARR_LIVE_FIXTURE_HOST to a host/IP reachable from shart"
        )
    })?;
    let body = Arc::new(SAB_FIXTURE_NZB.as_bytes().to_vec());
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let body = Arc::clone(&body);
            thread::spawn(move || {
                let _ = serve_fixture_request(stream, &body);
            });
        }
    });
    Ok(FixtureServer {
        base_url: format!("http://{host}:{FIXTURE_PORT}"),
    })
}

fn serve_fixture_request(mut stream: std::net::TcpStream, body: &[u8]) -> Result<()> {
    let mut buf = [0_u8; 1024];
    let _ = stream.read(&mut buf);
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/x-nzb\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    let _ = stream.flush();
    Ok(())
}

fn run_sabnzbd_lifecycle(
    report: &mut report::Report,
    mcp_url: &str,
    fixture: &FixtureServer,
) -> Result<()> {
    cleanup_sabnzbd_queue(mcp_url)?;
    let url = format!("{}/rustarr-live-test.nzb", fixture.base_url);
    let added = expect_success(
        "sabnzbd",
        "download_add",
        call_tool(
            mcp_url,
            "sabnzbd",
            &json!({ "action": "download_add", "url": url, "confirm": true }),
        )?,
    )?;
    if added.get("status").and_then(Value::as_bool) != Some(true) {
        bail!("SABnzbd download_add did not report status=true: {added}");
    }
    let id = added
        .get("nzo_ids")
        .and_then(Value::as_array)
        .and_then(|ids| ids.first())
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("SABnzbd download_add did not return nzo_ids: {added}"))?
        .to_string();
    wait_for_sab_job(mcp_url, &id, true)?;

    for action in ["download_pause", "download_resume"] {
        let value = expect_success(
            "sabnzbd",
            action,
            call_tool(
                mcp_url,
                "sabnzbd",
                &json!({ "action": action, "id": id, "confirm": true }),
            )?,
        )?;
        assert_sab_status(&value, &id, action)?;
    }

    let removed = expect_success(
        "sabnzbd",
        "download_remove",
        call_tool(
            mcp_url,
            "sabnzbd",
            &json!({
                "action": "download_remove",
                "id": id,
                "delete_files": false,
                "confirm": true,
            }),
        )?,
    )?;
    assert_sab_status(&removed, &id, "download_remove")?;
    wait_for_sab_job(mcp_url, &id, false)?;

    report.pass(
        "mcporter confirmed write sabnzbd download lifecycle",
        "download_add/pause/resume/remove changed observable queue state and cleaned up",
    );
    Ok(())
}

fn cleanup_sabnzbd_queue(mcp_url: &str) -> Result<()> {
    let queue = sab_queue(mcp_url)?;
    for id in sab_ids(&queue).collect::<Vec<_>>() {
        let _ = call_tool(
            mcp_url,
            "sabnzbd",
            &json!({
                "action": "download_remove",
                "id": id,
                "delete_files": false,
                "confirm": true,
            }),
        )?;
    }
    Ok(())
}

fn wait_for_sab_job(mcp_url: &str, id: &str, should_exist: bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut last_queue = Value::Null;
    while Instant::now() < deadline {
        let queue = sab_queue(mcp_url)?;
        let exists = sab_ids(&queue).any(|candidate| candidate == id);
        if exists == should_exist {
            return Ok(());
        }
        last_queue = queue;
        thread::sleep(Duration::from_millis(500));
    }
    let expected = if should_exist {
        "appear in"
    } else {
        "disappear from"
    };
    bail!("SABnzbd test job {id} did not {expected} queue: {last_queue}");
}

fn sab_queue(mcp_url: &str) -> Result<Value> {
    expect_success(
        "sabnzbd",
        "download_queue",
        call_tool(mcp_url, "sabnzbd", &json!({ "action": "download_queue" }))?,
    )
}

fn sab_ids(value: &Value) -> impl Iterator<Item = String> + '_ {
    value
        .get("slots")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|slot| slot.get("nzo_id").and_then(Value::as_str))
        .map(str::to_owned)
}

fn assert_sab_status(value: &Value, id: &str, action: &str) -> Result<()> {
    if value.get("status").and_then(Value::as_bool) != Some(true) {
        bail!("SABnzbd {action} did not report status=true: {value}");
    }
    let has_id = value
        .get("nzo_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|candidate| candidate.as_str() == Some(id));
    if !has_id {
        bail!("SABnzbd {action} response did not include nzo id {id}: {value}");
    }
    Ok(())
}

fn run_qbittorrent_lifecycle(report: &mut report::Report, mcp_url: &str) -> Result<()> {
    let hash = "1111111111111111111111111111111111111111";
    let magnet = format!("magnet:?xt=urn:btih:{hash}&dn=rustarr-live-mcporter-stateful");

    let _ = call_tool(
        mcp_url,
        "qbittorrent",
        &json!({
            "action": "download_remove",
            "id": hash,
            "delete_files": false,
            "confirm": true,
        }),
    )?;

    let add = expect_success(
        "qbittorrent",
        "download_add",
        call_tool(
            mcp_url,
            "qbittorrent",
            &json!({
                "action": "download_add",
                "url": magnet,
                "confirm": true,
            }),
        )?,
    )?;
    if !add.to_string().contains("Ok") {
        bail!("qBittorrent add did not return expected accepted response: {add}");
    }
    wait_for_torrent_presence(mcp_url, hash, true)?;

    for action in ["download_pause", "download_resume"] {
        let value = expect_success(
            "qbittorrent",
            action,
            call_tool(
                mcp_url,
                "qbittorrent",
                &json!({ "action": action, "id": hash, "confirm": true }),
            )?,
        )?;
        if value.get("submitted").and_then(Value::as_bool) != Some(true) {
            bail!("qBittorrent {action} did not report submitted=true: {value}");
        }
        wait_for_torrent_presence(mcp_url, hash, true)?;
    }

    let removed = expect_success(
        "qbittorrent",
        "download_remove",
        call_tool(
            mcp_url,
            "qbittorrent",
            &json!({
                "action": "download_remove",
                "id": hash,
                "delete_files": false,
                "confirm": true,
            }),
        )?,
    )?;
    if removed.get("submitted").and_then(Value::as_bool) != Some(true) {
        bail!("qBittorrent remove did not report submitted=true: {removed}");
    }
    wait_for_torrent_presence(mcp_url, hash, false)?;

    report.pass(
        "mcporter confirmed write lifecycle qbittorrent torrent",
        "download_add/pause/resume/remove changed observable queue state and cleaned up",
    );
    Ok(())
}

fn wait_for_torrent_presence(mcp_url: &str, hash: &str, should_exist: bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut last_queue = Value::Null;
    while Instant::now() < deadline {
        let queue = expect_success(
            "qbittorrent",
            "download_queue",
            call_tool(
                mcp_url,
                "qbittorrent",
                &json!({ "action": "download_queue" }),
            )?,
        )?;
        let exists = torrent_hashes(&queue).any(|candidate| candidate.eq_ignore_ascii_case(hash));
        if exists == should_exist {
            return Ok(());
        }
        last_queue = queue;
        thread::sleep(Duration::from_millis(500));
    }
    let expected = if should_exist {
        "appear in"
    } else {
        "disappear from"
    };
    bail!("qBittorrent test torrent {hash} did not {expected} queue: {last_queue}");
}

fn torrent_hashes(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|torrent| torrent.get("hash").and_then(Value::as_str))
}

fn expect_success(tool: &str, action: &str, outcome: CallOutcome) -> Result<Value> {
    match outcome {
        CallOutcome::Success(value) => Ok(value),
        CallOutcome::Failure(text) => {
            bail!("mcporter {tool}.{action} expected success, got {text}")
        }
    }
}

fn assert_object_field_eq(value: &Value, field: &str, expected: &str) -> Result<()> {
    match value.get(field).and_then(Value::as_str) {
        Some(actual) if actual == expected => Ok(()),
        _ => bail!("expected `{field}` to equal `{expected}` in {value}"),
    }
}

fn assert_eq_i64(value: &Value, field: &str, expected: i64) -> Result<()> {
    match value.get(field).and_then(Value::as_i64) {
        Some(actual) if actual == expected => Ok(()),
        _ => bail!("expected `{field}` to equal `{expected}` in {value}"),
    }
}

fn call_tool(mcp_url: &str, tool: &str, arguments: &Value) -> Result<CallOutcome> {
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
