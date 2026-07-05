use anyhow::Result;
use serde_json::{Value, json};
use yarr::ServiceKind;

use crate::live::process;

pub(in crate::live) fn seed_service_fixtures(
    yarr: &process::YarrProcess,
    svc: &str,
    kind: ServiceKind,
) -> Result<()> {
    match kind {
        ServiceKind::Sonarr => ensure_sonarr_download_client(yarr, svc),
        _ => Ok(()),
    }
}

fn ensure_sonarr_download_client(yarr: &process::YarrProcess, svc: &str) -> Result<()> {
    ensure_sonarr_qbittorrent_download_client(yarr, svc)?;
    ensure_sonarr_newznab_indexer(yarr, svc)?;
    ensure_sonarr_custom_script_notification(yarr, svc)?;
    ensure_sonarr_remote_path_mapping(yarr, svc)?;
    ensure_sonarr_autotagging(yarr, svc)
}

fn ensure_sonarr_qbittorrent_download_client(yarr: &process::YarrProcess, svc: &str) -> Result<()> {
    let existing = yarr.json(&[svc, "op", "get_downloadclient", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("yarr-live-qbit")
                && item.get("implementation").and_then(Value::as_str) == Some("QBittorrent")
        })
    }) {
        return Ok(());
    }
    let body = json!({
        "enable": false,
        "protocol": "torrent",
        "priority": 1,
        "removeCompletedDownloads": false,
        "removeFailedDownloads": false,
        "name": "yarr-live-qbit",
        "implementation": "QBittorrent",
        "implementationName": "qBittorrent",
        "configContract": "QBittorrentSettings",
        "fields": [
            {"name": "host", "value": "100.118.209.1"},
            {"name": "port", "value": 8080},
            {"name": "useSsl", "value": false},
            {"name": "urlBase", "value": ""},
            {"name": "apiKey", "value": ""},
            {"name": "username", "value": ""},
            {"name": "password", "value": ""},
            {"name": "tvCategory", "value": "tv-sonarr"},
            {"name": "tvImportedCategory", "value": ""},
            {"name": "recentTvPriority", "value": 0},
            {"name": "olderTvPriority", "value": 0},
            {"name": "initialState", "value": 0},
            {"name": "sequentialOrder", "value": false},
            {"name": "firstAndLast", "value": false},
            {"name": "contentLayout", "value": 0}
        ],
        "tags": []
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    yarr.json(&[svc, "op", "post_downloadclient", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_newznab_indexer(yarr: &process::YarrProcess, svc: &str) -> Result<()> {
    let existing = yarr.json(&[svc, "op", "get_indexer", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("yarr-live-newznab")
                && item.get("implementation").and_then(Value::as_str) == Some("Newznab")
        })
    }) {
        return Ok(());
    }
    let body = json!({
        "enableRss": false,
        "enableAutomaticSearch": false,
        "enableInteractiveSearch": false,
        "supportsRss": true,
        "supportsSearch": true,
        "protocol": "usenet",
        "priority": 1,
        "name": "yarr-live-newznab",
        "implementation": "Newznab",
        "implementationName": "Newznab",
        "configContract": "NewznabSettings",
        "fields": [
            {"name": "baseUrl", "value": "http://127.0.0.1:9"},
            {"name": "apiPath", "value": "/api"},
            {"name": "apiKey", "value": "yarr-live"},
            {"name": "categories", "value": [5030, 5040]},
            {"name": "animeCategories", "value": []},
            {"name": "animeStandardFormatSearch", "value": false},
            {"name": "additionalParameters", "value": ""},
            {"name": "multiLanguages", "value": []},
            {"name": "failDownloads", "value": []}
        ],
        "tags": []
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    yarr.json(&[svc, "op", "post_indexer", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_custom_script_notification(yarr: &process::YarrProcess, svc: &str) -> Result<()> {
    let existing = yarr.json(&[svc, "op", "get_notification", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("yarr-live-script")
                && item.get("implementation").and_then(Value::as_str) == Some("CustomScript")
        })
    }) {
        return Ok(());
    }
    let body = json!({
        "name": "yarr-live-script",
        "implementation": "CustomScript",
        "implementationName": "Custom Script",
        "configContract": "CustomScriptSettings",
        "fields": [
            {"name": "path", "value": "/bin/true"},
            {"name": "arguments", "value": ""}
        ],
        "onGrab": false,
        "onDownload": false,
        "onUpgrade": false,
        "onRename": false,
        "onSeriesAdd": false,
        "onSeriesDelete": false,
        "onEpisodeFileDelete": false,
        "onEpisodeFileDeleteForUpgrade": false,
        "onHealthIssue": false,
        "onHealthRestored": false,
        "onApplicationUpdate": false,
        "includeHealthWarnings": false,
        "tags": []
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    yarr.json(&[svc, "op", "post_notification", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_remote_path_mapping(yarr: &process::YarrProcess, svc: &str) -> Result<()> {
    let existing = yarr.json(&[svc, "op", "get_remotepathmapping", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("host").and_then(Value::as_str) == Some("yarr-live-host")
                && item.get("remotePath").and_then(Value::as_str) == Some("/downloads/")
                && item.get("localPath").and_then(Value::as_str) == Some("/data/media/tv/")
        })
    }) {
        return Ok(());
    }
    let body = json!({
        "host": "yarr-live-host",
        "remotePath": "/downloads/",
        "localPath": "/data/media/tv/"
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    yarr.json(&[svc, "op", "post_remotepathmapping", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_autotagging(yarr: &process::YarrProcess, svc: &str) -> Result<()> {
    let existing = yarr.json(&[svc, "op", "get_autotagging", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item.get("name").and_then(Value::as_str) == Some("yarr-live-autotag"))
    }) {
        return Ok(());
    }
    let body = json!({
        "name": "yarr-live-autotag",
        "removeTagsAutomatically": false,
        "tags": [40],
        "specifications": [{
            "name": "Monitored",
            "implementation": "MonitoredSpecification",
            "implementationName": "Monitored",
            "negate": false,
            "required": false,
            "fields": []
        }]
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    yarr.json(&[svc, "op", "post_autotagging", "--args", &args])?;
    Ok(())
}
