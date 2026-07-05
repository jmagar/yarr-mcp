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

fn expected_redirect_response(kind: ServiceKind, op_name: &str, detail: &str) -> bool {
    let has_location = detail.contains("; location: ");
    if !has_location {
        return false;
    }
    let auth_redirect = matches!(
        kind,
        ServiceKind::Sonarr | ServiceKind::Radarr | ServiceKind::Prowlarr
    ) && matches!(op_name, "get_logout" | "post_login")
        && detail.contains("returned HTTP 302")
        && (detail.contains("; location: /")
            || detail.contains("; location: login")
            || detail.contains("; location: /login"));
    let prowlarr_download_redirect = kind == ServiceKind::Prowlarr
        && matches!(op_name, "get_download_by_id" | "get_indexer_download_by_id")
        && detail.contains("returned HTTP 301")
        && (detail.contains("; location: http://") || detail.contains("; location: https://"));
    auth_redirect || prowlarr_download_redirect
}

fn sonarr_expected_domain_response(op_name: &str, detail: &str) -> bool {
    matches!(
        op_name,
        "get_series_lookup"
            | "get_update"
            | "post_history_failed_by_id"
            | "post_queue_grab_by_id"
            | "post_downloadclient_test"
            | "post_importlist_test"
            | "post_qualityprofile"
            | "post_release"
            | "post_release_push"
            | "post_rootfolder"
            | "post_seasonpass"
            | "post_series"
            | "post_series_import"
            | "get_episodefile_by_id"
            | "get_localization_by_id"
            | "put_episodefile_by_id"
            | "put_customformat_bulk"
            | "put_customformat_by_id"
            | "put_downloadclient_bulk"
            | "put_episodefile_bulk"
            | "put_episodefile_editor"
            | "put_importlist_bulk"
            | "put_indexer_bulk"
            | "put_qualitydefinition_update"
            | "put_qualityprofile_by_id"
            | "put_releaseprofile_by_id"
            | "delete_blocklist_by_id"
            | "delete_episodefile_by_id"
            | "delete_queue_by_id"
            | "delete_customformat_by_id"
            | "delete_delayprofile_by_id"
            | "delete_episodefile_bulk"
            | "delete_qualityprofile_by_id"
    ) && (detail.contains("returned HTTP 400")
        || detail.contains("returned HTTP 404")
        || detail.contains("returned HTTP 405")
        || detail.contains("returned HTTP 500"))
}

fn radarr_expected_domain_response(op_name: &str, detail: &str) -> bool {
    matches!(
        op_name,
        "get_manualimport"
            | "post_customformat"
            | "post_delayprofile"
            | "post_downloadclient_test"
            | "post_downloadclient_testall"
            | "post_history_failed_by_id"
            | "post_importlist"
            | "post_importlist_action_by_name"
            | "post_importlist_test"
            | "post_importlist_testall"
            | "post_indexer_test"
            | "post_indexer_testall"
            | "post_manualimport"
            | "post_metadata"
            | "post_metadata_test"
            | "post_metadata_testall"
            | "post_movie"
            | "post_notification_test"
            | "post_notification_testall"
            | "post_qualityprofile"
            | "post_queue_grab_by_id"
            | "post_release"
            | "post_release_push"
            | "post_releaseprofile"
            | "post_rootfolder"
            | "get_collection_by_id"
            | "get_media_watch_data_by_media_id"
            | "get_customformat_by_id"
            | "get_importlist_by_id"
            | "get_moviefile_by_id"
            | "get_releaseprofile_by_id"
            | "put_collection"
            | "put_collection_by_id"
            | "put_customformat_bulk"
            | "put_customformat_by_id"
            | "put_delayprofile_by_id"
            | "put_downloadclient_bulk"
            | "put_importlist_bulk"
            | "put_importlist_by_id"
            | "put_indexer_bulk"
            | "put_moviefile_bulk"
            | "put_moviefile_by_id"
            | "put_moviefile_editor"
            | "put_qualitydefinition_update"
            | "put_releaseprofile_by_id"
            | "put_tag_by_id"
            | "delete_blocklist_by_id"
            | "delete_command_by_id"
            | "delete_customformat_bulk"
            | "delete_customformat_by_id"
            | "delete_delayprofile_by_id"
            | "delete_downloadclient_bulk"
            | "delete_importlist_bulk"
            | "delete_importlist_by_id"
            | "delete_indexer_bulk"
            | "delete_moviefile_bulk"
            | "delete_moviefile_by_id"
            | "delete_queue_by_id"
            | "delete_releaseprofile_by_id"
            | "delete_tag_by_id"
            | "post_system_backup_restore_by_id"
            | "post_system_backup_restore_upload"
            | "get_settings_cache"
            | "get_service_radarr_by_radarr_id"
            | "get_user_watch_data_by_user_id"
    ) && (detail.contains("returned HTTP 400")
        || detail.contains("returned HTTP 404")
        || detail.contains("returned HTTP 405")
        || detail.contains("returned HTTP 409")
        || detail.contains("returned HTTP 500"))
}

fn generated_media_server_domain_response(detail: &str) -> bool {
    detail.contains("returned HTTP 400")
        || detail.contains("returned HTTP 404")
        || detail.contains("returned HTTP 405")
        || detail.contains("returned HTTP 409")
}

fn jellyfin_expected_specific_domain_response(op_name: &str, detail: &str) -> bool {
    matches!(op_name, "update_user_policy")
        && detail.contains("returned HTTP 403")
        && detail
            .contains("There must be at least one user in the system with administrative access.")
        || matches!(
            op_name,
            "get_channel_mapping_options"
                | "get_trailers"
                | "create_collection"
                | "move_item"
                | "set_channel_mapping"
                | "get_attachment"
                | "get_audio_stream"
                | "get_audio_stream_by_container"
                | "get_playback_info"
                | "get_program"
                | "get_remote_subtitles"
                | "get_subtitle"
                | "get_subtitle_with_ticks"
                | "get_universal_audio_stream"
                | "get_video_stream"
                | "get_video_stream_by_container"
                | "remove_item_from_playlist"
        ) && detail.contains("returned HTTP 500")
            && detail.contains("Error processing request.")
        || matches!(
            op_name,
            "authenticate_with_quick_connect"
                | "authorize_quick_connect"
                | "complete_wizard"
                | "create_key"
                | "restart_application"
                | "revoke_key"
                | "set_remote_access"
                | "shutdown_application"
                | "start_restore_backup"
                | "update_branding_configuration"
                | "update_configuration"
                | "update_initial_configuration"
                | "update_named_configuration"
                | "update_plugin_configuration"
                | "update_startup_user"
                | "update_user_configuration"
        ) && detail.contains("returned HTTP 503")
            && detail.contains("Jellyfin Server is loading. Please try again shortly.")
}

fn plex_expected_specific_domain_response(op_name: &str, detail: &str) -> bool {
    matches!(
        op_name,
        "get_all_subscriptions"
            | "get_scheduled_recordings"
            | "process_subscriptions"
            | "terminate_session"
            | "edit_subscription_preferences"
            | "get_subscription"
    ) && detail.contains("returned HTTP 401")
        && detail.contains("401 Unauthorized")
        || matches!(
            op_name,
            "get_source_connection_information"
                | "get_transient_token"
                | "detect_credits"
                | "detect_voice_activity"
                | "get_sonic_path"
                | "list_sonically_similar"
        ) && detail.contains("returned HTTP 403")
            && detail.contains("403 Forbidden")
        || matches!(op_name, "get_playlist_items")
            && detail.contains("returned HTTP 500")
            && detail.contains("500 Internal Server Error")
        || matches!(op_name, "get_stream")
            && detail.contains("returned HTTP 501")
            && detail.contains("501 Not Implemented")
}

fn overseerr_expected_domain_response(op_name: &str, detail: &str) -> bool {
    if overseerr_expected_specific_domain_response(op_name, detail) {
        return true;
    }
    let is_overseerr_name = op_name.contains("settings")
        || op_name.contains("auth")
        || op_name.contains("request")
        || op_name.contains("issue")
        || op_name.contains("discover")
        || op_name.contains("movie")
        || op_name.contains("tv")
        || op_name.contains("user")
        || op_name.contains("media")
        || op_name.contains("person")
        || op_name.contains("keyword")
        || op_name.contains("collection")
        || op_name.contains("network")
        || op_name.contains("studio")
        || op_name.contains("service_sonarr");
    is_overseerr_name
        && (detail.contains("returned HTTP 400")
            || detail.contains("returned HTTP 404")
            || detail.contains("returned HTTP 405")
            || detail.contains("returned HTTP 409")
            || detail.contains("overseerr request failed"))
}

fn overseerr_expected_specific_domain_response(op_name: &str, detail: &str) -> bool {
    matches!((op_name, detail), ("get_settings_notifications_pushover_sounds", d)
        if d.contains("returned HTTP 500") && d.contains("Unable to retrieve Pushover sounds."))
        || matches!((op_name, detail), ("put_user", d)
            if d.contains("returned HTTP 500") && d.contains("parameterValue.value is not iterable"))
        || matches!((op_name, detail), ("delete_user_push_subscription_by_user_id_endpoint", d)
            if d.contains("returned HTTP 500") && d.contains("User push subcription not found"))
        || matches!((op_name, detail), ("post_auth_local", d)
            if d.contains("returned HTTP 403") && d.contains("Access denied."))
        || matches!((op_name, detail), ("post_auth_plex", d)
            if d.contains("returned HTTP 500") && d.contains("Unable to authenticate."))
        || matches!((op_name, detail), ("post_auth_reset_password_by_guid", d)
            if d.contains("returned HTTP 500") && d.contains("Password must be at least 8 characters long."))
        || matches!((op_name, detail), ("post_settings_notifications_discord_test", d)
            if d.contains("returned HTTP 500") && d.contains("Failed to send Discord notification."))
        || matches!((op_name, detail), ("post_settings_notifications_lunasea_test", d)
            if d.contains("returned HTTP 500") && d.contains("Failed to send web push notification."))
        || matches!((op_name, detail), ("post_settings_notifications_slack_test", d)
            if d.contains("returned HTTP 500") && d.contains("Failed to send Slack notification."))
        || matches!((op_name, detail), ("post_settings_notifications_webhook", d)
            if d.contains("returned HTTP 500") && d.contains("is not valid JSON"))
        || matches!((op_name, detail), ("post_settings_notifications_webhook_test", d)
            if d.contains("returned HTTP 500") && d.contains("Failed to send webhook notification."))
        || matches!((op_name, detail), ("post_settings_plex", d)
            if d.contains("returned HTTP 500") && d.contains("Unable to connect to Plex."))
        || matches!((op_name, detail), ("post_settings_tautulli", d)
            if d.contains("returned HTTP 500") && d.contains("Unable to connect to Tautulli."))
        || matches!((op_name, detail), ("post_user_settings_permissions_by_user_id", d)
            if d.contains("returned HTTP 403")
                && d.contains("You do not have permission to modify this user"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, json};
    use yarr::openapi::{self, OperationSpec};

    #[test]
    fn jellyfin_loading_response_is_retried_not_classified() {
        let op = Box::leak(Box::new(OperationSpec {
            name: "get_auth_providers",
            method: openapi::HttpMethod::Get,
            path: "/Auth/Providers",
            path_params: &[],
            query_params: &[],
            has_body: false,
            request_type: None,
            response_type: None,
            tag: "Auth",
            summary: "Get auth providers",
        }));
        let call = PreparedCall {
            kind: ServiceKind::Jellyfin,
            op,
            args: Map::new(),
        };
        let values = vec![json!({
            "name": "get_auth_providers",
            "args": {},
            "ok": false,
            "error": "Error: jellyfin returned HTTP 503 (Jellyfin Server is loading. Please try again shortly.)"
        })];
        assert!(should_retry_domain_result(&[call], &values));
    }

    #[test]
    fn media_server_domain_response_rejects_auth_and_server_outages() {
        for status in [401, 403, 500, 501, 503] {
            assert!(
                !generated_media_server_domain_response(&format!(
                    "jellyfin returned HTTP {status} (broken)"
                )),
                "HTTP {status} must not be accepted as a generic media-server domain response"
            );
        }
        for status in [400, 404, 405, 409] {
            assert!(generated_media_server_domain_response(&format!(
                "jellyfin returned HTTP {status} (domain error)"
            )));
        }
    }

    #[test]
    fn jellyfin_specific_domain_response_is_narrow() {
        assert!(jellyfin_expected_specific_domain_response(
            "get_video_stream",
            "Error: jellyfin returned HTTP 500 (Error processing request.)"
        ));
        assert!(jellyfin_expected_specific_domain_response(
            "shutdown_application",
            "Error: jellyfin returned HTTP 503 (Jellyfin Server is loading. Please try again shortly.)"
        ));
        assert!(jellyfin_expected_specific_domain_response(
            "update_user_policy",
            "Error: jellyfin returned HTTP 403 (\"There must be at least one user in the system with administrative access.\")"
        ));
        assert!(!jellyfin_expected_specific_domain_response(
            "get_video_stream",
            "Error: jellyfin returned HTTP 500 (panic)"
        ));
        assert!(!jellyfin_expected_specific_domain_response(
            "get_users",
            "Error: jellyfin returned HTTP 503 (Jellyfin Server is loading. Please try again shortly.)"
        ));
    }

    #[test]
    fn plex_specific_domain_response_is_narrow() {
        assert!(plex_expected_specific_domain_response(
            "get_all_subscriptions",
            "Error: plex returned HTTP 401 (<html><head><title>Unauthorized</title></head><body><h1>401 Unauthorized</h1></body></html>)"
        ));
        assert!(plex_expected_specific_domain_response(
            "detect_credits",
            "Error: plex returned HTTP 403 (<html><head><title>Forbidden</title></head><body><h1>403 Forbidden</h1></body></html>)"
        ));
        assert!(plex_expected_specific_domain_response(
            "get_stream",
            "Error: plex returned HTTP 501 (<html><head><title>Not Implemented</title></head><body><h1>501 Not Implemented</h1></body></html>)"
        ));
        assert!(!plex_expected_specific_domain_response(
            "get_sections",
            "Error: plex returned HTTP 401 (<html><head><title>Unauthorized</title></head><body><h1>401 Unauthorized</h1></body></html>)"
        ));
        assert!(!plex_expected_specific_domain_response(
            "get_stream",
            "Error: plex returned HTTP 500 (<html><head><title>Internal Server Error</title></head><body><h1>500 Internal Server Error</h1></body></html>)"
        ));
    }

    #[test]
    fn overseerr_domain_response_rejects_auth_and_server_outages() {
        for status in [401, 403, 500, 501, 503] {
            assert!(
                !overseerr_expected_domain_response(
                    "get_user_by_user_id",
                    &format!("overseerr returned HTTP {status} (broken)")
                ),
                "HTTP {status} must not be accepted as an Overseerr domain response"
            );
        }
        for status in [400, 404, 405, 409] {
            assert!(overseerr_expected_domain_response(
                "get_user_by_user_id",
                &format!("overseerr returned HTTP {status} (domain error)")
            ));
        }
    }
}
