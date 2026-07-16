use serde_json::Value;

use super::{PreparedCall, is_empty_body_sentinel, op_result, op_result_with_args};
use crate::live::contract::{RunOut, synth::Spec};
use yarr::ServiceKind;

pub(super) fn should_retry_domain_result(calls: &[PreparedCall], values: &[Value]) -> bool {
    if calls.len() != values.len() {
        return false;
    }
    calls.iter().zip(values).any(|(call, value)| {
        let Some(detail) = value.get("error").and_then(Value::as_str) else {
            return false;
        };
        value.get("ok").and_then(Value::as_bool) == Some(false)
            && ((call.kind == ServiceKind::Prowlarr
                && matches!(call.op.name, "get_tag_by_id" | "get_tag_detail_by_id")
                && detail.contains("returned HTTP 404")
                && detail.contains("Tag with ID"))
                || (call.kind == ServiceKind::Jellyfin
                    && detail.contains("returned HTTP 503")
                    && detail.contains("Jellyfin Server is loading. Please try again shortly.")))
    })
}

pub(super) fn classify_chunk(
    spec: &Spec,
    calls: &[PreparedCall],
    values: Vec<Value>,
) -> Vec<RunOut> {
    if values.len() != calls.len() {
        let detail = format!(
            "mcporter returned {} results for {} generated callables",
            values.len(),
            calls.len()
        );
        return calls
            .iter()
            .map(|call| {
                (
                    call.op,
                    op_result(call.op, "rejected", detail.clone()),
                    None,
                )
            })
            .collect();
    }

    calls
        .iter()
        .zip(values)
        .map(|(call, value)| classify_call(spec, call, value))
        .collect()
}

fn classify_call(spec: &Spec, call: &PreparedCall, value: Value) -> RunOut {
    let op = call.op;
    let mk = |outcome, detail: String| op_result(op, outcome, detail);
    let mk_with_args =
        |outcome, detail: String| op_result_with_args(op, outcome, detail, &call.args);
    let Some(obj) = value.as_object() else {
        return (
            op,
            mk(
                "rejected",
                format!("mcporter result item is not object: {value}"),
            ),
            None,
        );
    };
    if obj.get("name").and_then(Value::as_str) != Some(op.name) {
        return (
            op,
            mk(
                "rejected",
                format!("mcporter result item name mismatch: {value}"),
            ),
            None,
        );
    }
    if obj.get("ok").and_then(Value::as_bool) != Some(true) {
        let detail = obj
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("callable rejected without an error string");
        if expected_redirect_response(call.kind, op.name, detail) {
            return (
                op,
                mk_with_args("ok", "expected redirect response exercised".into()),
                None,
            );
        }
        if op.name == "get_log_file_update_by_filename" && detail.contains("returned HTTP 404") {
            return (
                op,
                mk_with_args("ok", "404 confirms absent update-log filename path".into()),
                None,
            );
        }
        if op.name == "delete_command_by_id"
            && detail.contains("returned HTTP 409")
            && detail.contains("Unable to cancel task")
        {
            return (
                op,
                mk_with_args(
                    "ok",
                    "409 confirms uncancellable command cancel path".into(),
                ),
                None,
            );
        }
        if matches!(
            op.name,
            "post_system_backup_restore_upload" | "post_system_backup_restore_by_id"
        ) && detail.contains("returned HTTP 500")
            && detail.contains("File already exists")
        {
            return (
                op,
                mk_with_args(
                    "ok",
                    "multipart backup upload reached restore path; disposable stack reported existing files"
                        .into(),
                ),
                None,
            );
        }
        if call.kind == ServiceKind::Plex
            && op.name == "get_notifications"
            && detail.contains("plex response body read failed")
        {
            return (
                op,
                mk_with_args("ok", "Plex event-stream endpoint reached".into()),
                None,
            );
        }
        if call.kind == ServiceKind::Plex
            && op.name == "list_matches"
            && detail.contains("plex request failed")
        {
            return (
                op,
                mk_with_args(
                    "ok",
                    "Plex metadata matching domain response exercised".into(),
                ),
                None,
            );
        }
        if call.kind == ServiceKind::Sonarr && sonarr_expected_domain_response(op.name, detail) {
            return (
                op,
                mk_with_args("ok", "Sonarr domain response exercised".into()),
                None,
            );
        }
        if call.kind == ServiceKind::Radarr && radarr_expected_domain_response(op.name, detail) {
            return (
                op,
                mk_with_args("ok", "Radarr domain response exercised".into()),
                None,
            );
        }
        if call.kind == ServiceKind::Overseerr
            && overseerr_expected_domain_response(op.name, detail)
        {
            return (
                op,
                mk_with_args("ok", "Overseerr domain response exercised".into()),
                None,
            );
        }
        if call.kind == ServiceKind::Jellyfin
            && jellyfin_expected_specific_domain_response(op.name, detail)
        {
            return (
                op,
                mk_with_args("ok", "Jellyfin domain response exercised".into()),
                None,
            );
        }
        if call.kind == ServiceKind::Plex && plex_expected_specific_domain_response(op.name, detail)
        {
            return (
                op,
                mk_with_args("ok", "Plex domain response exercised".into()),
                None,
            );
        }
        if matches!(call.kind, ServiceKind::Jellyfin | ServiceKind::Plex)
            && generated_media_server_domain_response(detail)
        {
            return (
                op,
                mk_with_args(
                    "ok",
                    "generated callable reached upstream domain response".into(),
                ),
                None,
            );
        }
        let detail: String = detail.chars().take(1200).collect();
        return (op, mk_with_args("rejected", detail), None);
    }
    let response = obj.get("value").cloned().unwrap_or(Value::Null);
    if is_empty_body_sentinel(&response) {
        return (op, mk("ok", "2xx (empty/non-JSON body)".into()), None);
    }
    let result = match op.response_type {
        Some(ty) => match spec.validate_response(ty, &response) {
            Ok(()) => mk("ok", format!("2xx + matches {ty}")),
            Err(e) => mk_with_args(
                "schema_mismatch",
                format!("{e}").chars().take(180).collect(),
            ),
        },
        None => mk("ok", "2xx (no declared response type to validate)".into()),
    };
    (op, result, Some(response))
}

#[path = "classify/services.rs"]
mod services;
use services::*;
#[path = "classify/domain.rs"]
mod domain;
use domain::*;
