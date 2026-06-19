use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::collections::BTreeSet;

use super::assertions;
use super::{CallOutcome, call_tool, matrix, report};

mod arr;
mod download;
mod fixtures;

pub(super) fn run_confirmed_write_state_checks(
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
            arr::run_tag_lifecycle(report, mcp_url, service, prefix)?;
        }
    }

    run_confirmed_generic_error_checks(report, mcp_url, matrix)?;

    if services.contains("sonarr") {
        arr::run_arr_item_lifecycle(
            report,
            mcp_url,
            "sonarr",
            "/api/v3",
            "Firefly",
            "/data/rustarr-live-sonarr",
        )?;
    }

    if services.contains("radarr") {
        arr::run_arr_item_lifecycle(
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
        let fixture = download::start_fixture_server()?;
        download::run_sabnzbd_lifecycle(report, mcp_url, &fixture)?;
    }

    if services.contains("qbittorrent") {
        download::run_qbittorrent_lifecycle(report, mcp_url)?;
    }

    if services.contains("tautulli") {
        download::run_tautulli_maintenance_lifecycle(report, mcp_url)?;
    }

    if services.contains("bazarr") {
        fixtures::run_bazarr_blacklist_delete(report, mcp_url)?;
    }

    if services.contains("tracearr") {
        fixtures::run_tracearr_debug_delete(report, mcp_url)?;
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

pub(super) fn expect_success(tool: &str, action: &str, outcome: CallOutcome) -> Result<Value> {
    match outcome {
        CallOutcome::Success(value) => Ok(value),
        CallOutcome::Failure(text) => {
            bail!("mcporter {tool}.{action} expected success, got {text}")
        }
    }
}

pub(super) fn assert_object_field_eq(value: &Value, field: &str, expected: &str) -> Result<()> {
    match value.get(field).and_then(Value::as_str) {
        Some(actual) if actual == expected => Ok(()),
        _ => bail!("expected `{field}` to equal `{expected}` in {value}"),
    }
}

pub(super) fn assert_eq_i64(value: &Value, field: &str, expected: i64) -> Result<()> {
    match value.get(field).and_then(Value::as_i64) {
        Some(actual) if actual == expected => Ok(()),
        _ => bail!("expected `{field}` to equal `{expected}` in {value}"),
    }
}
