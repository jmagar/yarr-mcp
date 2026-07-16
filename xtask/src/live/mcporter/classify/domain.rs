use super::*;
pub(super) fn expected_redirect_response(kind: ServiceKind, op_name: &str, detail: &str) -> bool {
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

pub(super) fn sonarr_expected_domain_response(op_name: &str, detail: &str) -> bool {
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

pub(super) fn radarr_expected_domain_response(op_name: &str, detail: &str) -> bool {
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
