//! Exhaustive contract harness for the generated OpenAPI surface.
//!
//! shart is a disposable, dedicated test stack, so this drives **every generated
//! operation** of every spec-backed service (all methods — reads, writes, and
//! destructive deletes) via the `rustarr <service> op <name>` CLI verb, with inputs
//! synthesized from the vendored spec, and validates each 2xx response against the
//! operation's declared response schema. Output is a per-service summary plus a
//! per-operation breakdown written to `target/live-full/contract-<svc>.json`.
//!
//! Operations run GET -> POST -> PUT/PATCH -> DELETE so reads/updates see existing
//! resources before deletes remove them. Pass `--no-destructive` to skip DELETEs.

pub(super) mod synth;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::process::Command;

use rustarr::ServiceKind;
use rustarr::openapi::{self, HttpMethod, OperationSpec};

use super::reset;
use super::{process, report};
use synth::Spec;

/// (kind str, spec path) for the spec-backed services.
pub(super) const SPECS: &[(&str, &str)] = &[
    ("sonarr", "specs/sonarr.openapi.json"),
    ("radarr", "specs/radarr.openapi.json"),
    ("prowlarr", "specs/prowlarr.openapi.json"),
    ("overseerr", "specs/overseerr.openapi.yml"),
    ("jellyfin", "specs/jellyfin.openapi.json"),
    ("plex", "specs/plex.openapi.yml"),
];

pub(super) fn kind_of(name: &str) -> Option<ServiceKind> {
    Some(match name {
        "sonarr" => ServiceKind::Sonarr,
        "radarr" => ServiceKind::Radarr,
        "prowlarr" => ServiceKind::Prowlarr,
        "overseerr" => ServiceKind::Overseerr,
        "jellyfin" => ServiceKind::Jellyfin,
        "plex" => ServiceKind::Plex,
        _ => return None,
    })
}

#[derive(Serialize)]
pub(super) struct OpResult {
    pub(super) name: &'static str,
    pub(super) method: &'static str,
    pub(super) path: &'static str,
    pub(super) outcome: &'static str, // ok | schema_mismatch | rejected
    pub(super) detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) args: Option<Value>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ContractStatus {
    pub(super) ok: usize,
    pub(super) schema_mismatch: usize,
    pub(super) rejected: usize,
    pub(super) skipped: usize,
    pub(super) total: usize,
    pub(super) passed: bool,
    pub(super) detail: String,
}

pub fn run(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
    matrix: &super::matrix::Matrix,
    no_destructive: bool,
    only_service: Option<&str>,
) -> Result<()> {
    let configured: std::collections::BTreeSet<&str> =
        matrix.services.iter().map(|s| s.kind.as_str()).collect();

    for (svc, spec_path) in SPECS {
        if only_service.is_some_and(|only| only != *svc) {
            continue;
        }
        if !configured.contains(svc) {
            continue;
        }
        let kind = kind_of(svc).expect("spec-backed kind");
        if reset::target_for(svc).is_some() {
            reset::reset_service(svc)?;
            if let Some(url) = reset::service_url(&rustarr.env, svc) {
                reset::wait_service_url(&url)?;
            }
        }
        seed_service_fixtures(rustarr, svc, kind)
            .with_context(|| format!("seed live fixtures for {svc}"))?;
        let spec = Spec::load(spec_path).with_context(|| format!("load {spec_path}"))?;
        let ops: Vec<&'static OperationSpec> = openapi::operations_for_kind(kind).iter().collect();

        // Create-first seeding: run phases in order, harvesting ids between them so
        // later phases can hit real resources:
        //   0  base collection reads (GET, no path/query fixture dependency)
        //   1  query collection reads (GETs needing seeded query ids)
        //   2  creates (POST)                          -> seed ids from created objects
        //   3  resource reads/updates (GET/PUT/PATCH)  -> use seeded ids
        //   4  deletes (DELETE)                        -> use seeded ids; also cleanup
        let mut fixtures = FixtureStore::default();
        let mut results: Vec<OpResult> = Vec::with_capacity(ops.len());
        for phase in 0u8..=4 {
            let phase_ops: Vec<&'static OperationSpec> = ops
                .iter()
                .copied()
                .filter(|o| seed_phase(o) == phase)
                .collect();
            let outs = parallel_run(
                rustarr,
                svc,
                kind,
                &spec,
                &fixtures,
                &phase_ops,
                no_destructive,
            );
            harvest_into(&mut fixtures, &outs);
            results.extend(outs.into_iter().map(|(_, r, _)| r));
        }

        write_detail(svc, &results)?;
        let status = contract_status(&results);
        if status.passed {
            report.pass(format!("contract {svc}"), status.detail);
        } else {
            report.fail(format!("contract {svc}"), status.detail);
        }
    }
    Ok(())
}

pub(super) fn seed_service_fixtures(
    rustarr: &process::RustarrProcess,
    svc: &str,
    kind: ServiceKind,
) -> Result<()> {
    match kind {
        ServiceKind::Sonarr => ensure_sonarr_download_client(rustarr, svc),
        ServiceKind::Prowlarr => ensure_prowlarr_fixtures(rustarr, svc),
        ServiceKind::Radarr => ensure_radarr_fixtures(rustarr, svc),
        _ => Ok(()),
    }
}

fn ensure_prowlarr_fixtures(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    ensure_prowlarr_filesystem_prereqs()?;
    ensure_prowlarr_application(rustarr, svc)?;
    ensure_prowlarr_app_profile(rustarr, svc)?;
    ensure_prowlarr_download_client(rustarr, svc)?;
    if let Err(err) = ensure_prowlarr_indexer(rustarr, svc) {
        eprintln!("warning: failed to seed Prowlarr indexer fixture: {err:#}");
    }
    ensure_prowlarr_indexer_proxy(rustarr, svc)?;
    ensure_prowlarr_backup(rustarr, svc)?;
    ensure_prowlarr_custom_script_notification(rustarr, svc)
}

fn ensure_prowlarr_filesystem_prereqs() -> Result<()> {
    let status = Command::new("ssh")
        .arg("shart")
        .arg(
            r##"docker exec prowlarr sh -lc 'mkdir -p /tmp/rustarr-prowlarr-torrents && chmod 777 /tmp/rustarr-prowlarr-torrents && printf "#!/bin/sh\nexit 0\n" > /config/rustarr-live-notify.sh && chmod +x /config/rustarr-live-notify.sh'
cat >/tmp/rustarr-newznab.py <<'PY'
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs
class H(BaseHTTPRequestHandler):
    def do_GET(self):
        open('/tmp/rustarr-newznab-requests.log','a').write(self.path+'\n')
        q=parse_qs(urlparse(self.path).query)
        t=(q.get('t') or [''])[0]
        if self.path.startswith('/download'):
            self.send_response(200); self.send_header('Content-Type','application/x-bittorrent'); self.end_headers(); self.wfile.write(b'd8:announce13:http://x/e'); return
        if t=='caps':
            body='''<?xml version="1.0" encoding="UTF-8"?><caps><server title="rustarr"/><limits max="100" default="100"/><searching><search available="yes" supportedParams="q"/><tv-search available="yes" supportedParams="q"/><movie-search available="yes" supportedParams="q"/></searching><categories><category id="2000" name="Movies"/><category id="5000" name="TV"/></categories></caps>'''
        else:
            body='''<?xml version="1.0" encoding="UTF-8"?><rss version="2.0" xmlns:newznab="http://www.newznab.com/DTD/2010/feeds/attributes/"><channel><title>rustarr</title><item><title>test Ubuntu Rustarr Fixture 2026 1080p</title><guid isPermaLink="false">test-rustarr-fixture-1</guid><link>http://100.118.209.1:18080/download/rustarr.torrent</link><comments>http://100.118.209.1:18080/details/1</comments><pubDate>Sat, 04 Jul 2026 00:00:00 GMT</pubDate><category>Movies</category><size>1048576</size><enclosure url="http://100.118.209.1:18080/download/rustarr.torrent" length="1048576" type="application/x-bittorrent"/><newznab:attr name="category" value="2000"/><newznab:attr name="size" value="1048576"/><newznab:attr name="guid" value="test-rustarr-fixture-1"/></item></channel></rss>'''
        self.send_response(200); self.send_header('Content-Type','application/xml'); self.end_headers(); self.wfile.write(body.encode())
    def log_message(self,*a): pass
HTTPServer(('0.0.0.0',18080), H).serve_forever()
PY
fuser -k 18080/tcp >/dev/null 2>&1 || true
nohup python3 /tmp/rustarr-newznab.py >/tmp/rustarr-newznab.log 2>&1 &
docker exec prowlarr sh -lc 'cat >/tmp/rustarr-flaresolverr.py <<'"'"'PY'"'"'
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
class H(BaseHTTPRequestHandler):
    def _send(self, obj):
        body=json.dumps(obj).encode()
        self.send_response(200)
        self.send_header("Content-Type","application/json")
        self.send_header("Content-Length",str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def do_GET(self):
        self._send({"msg":"FlareSolverr is ready!","version":"3.3.21","userAgent":"rustarr-live"})
    def do_POST(self):
        length=int(self.headers.get("Content-Length","0") or 0)
        self.rfile.read(length)
        self._send({"status":"ok","message":"","solution":{"url":"http://rustarr.local/","status":200,"headers":{},"response":"<html></html>","cookies":[],"userAgent":"rustarr-live"},"startTimestamp":0,"endTimestamp":1,"version":"3.3.21"})
    def log_message(self,*a): pass
HTTPServer(("127.0.0.1",8191), H).serve_forever()
PY
fuser -k 8191/tcp >/dev/null 2>&1 || true
nohup python3 /tmp/rustarr-flaresolverr.py >/tmp/rustarr-flaresolverr.log 2>&1 &'
sleep 0.5"##,
        )
        .status()
        .context("prepare Prowlarr live filesystem fixtures on shart")?;
    anyhow::ensure!(
        status.success(),
        "prepare Prowlarr live filesystem fixtures on shart failed with {status}"
    );
    Ok(())
}

fn ensure_prowlarr_application(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_applications", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-sonarr-app")
                && item.get("implementation").and_then(Value::as_str) == Some("Sonarr")
        })
    }) {
        return Ok(());
    }
    let body = prowlarr_sonarr_application_body("rustarr-live-sonarr-app".into());
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_applications", "--args", &args])?;
    Ok(())
}

fn ensure_prowlarr_download_client(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_downloadclient", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-blackhole")
                && item.get("implementation").and_then(Value::as_str) == Some("TorrentBlackhole")
        })
    }) {
        return Ok(());
    }
    let body = prowlarr_download_client_body("rustarr-live-blackhole".into());
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_downloadclient", "--args", &args])?;
    Ok(())
}

fn ensure_prowlarr_app_profile(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_appprofile", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item.get("name").and_then(Value::as_str) == Some("rustarr-live-profile"))
    }) {
        return Ok(());
    }
    let body = json!({
        "name": "rustarr-live-profile",
        "enableRss": true,
        "enableAutomaticSearch": true,
        "enableInteractiveSearch": true,
        "minimumSeeders": 1
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_appprofile", "--args", &args])?;
    Ok(())
}

fn ensure_prowlarr_indexer(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_indexer", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-torznab")
                && item.get("implementation").and_then(Value::as_str) == Some("Torznab")
        })
    }) {
        return Ok(());
    }
    let body = prowlarr_indexer_body("rustarr-live-torznab".into());
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_indexer", "--args", &args])?;
    Ok(())
}

fn ensure_prowlarr_backup(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let mut backups = rustarr.json(&[svc, "op", "get_system_backup", "--args", "{}"])?;
    while backups.as_array().map_or(0, Vec::len) < 2 {
        let before = backups.as_array().map_or(0, Vec::len);
        let args = serde_json::to_string(&json!({ "body": { "name": "Backup" } }))?;
        rustarr.json(&[svc, "op", "post_command", "--args", &args])?;
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            backups = rustarr.json(&[svc, "op", "get_system_backup", "--args", "{}"])?;
            if backups.as_array().map_or(0, Vec::len) > before {
                break;
            }
        }
        anyhow::ensure!(
            backups.as_array().map_or(0, Vec::len) > before,
            "Prowlarr backup fixture count did not increase after Backup command"
        );
    }
    copy_prowlarr_backup_fixture(&backups)
}

fn copy_prowlarr_backup_fixture(backups: &Value) -> Result<()> {
    let path = backups
        .as_array()
        .and_then(|items| items.first())
        .and_then(|backup| backup.get("path"))
        .and_then(Value::as_str)
        .context("Prowlarr backup fixture has no path")?;
    let file_name = path
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .context("Prowlarr backup fixture path has no filename")?;
    let local = prowlarr_backup_upload_fixture_path();
    if let Some(parent) = std::path::Path::new(&local).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let remote_path = format!("/config/prowlarr/Backups/manual/{file_name}");
    let output = Command::new("ssh")
        .arg("shart")
        .arg(format!(
            "docker exec prowlarr cat {}",
            shell_quote(&remote_path)
        ))
        .output()
        .context("copy Prowlarr backup fixture from shart")?;
    anyhow::ensure!(
        output.status.success(),
        "copy Prowlarr backup fixture from shart failed with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::write(local, output.stdout)?;
    Ok(())
}

fn prowlarr_backup_upload_fixture_path() -> String {
    "target/live-full/tmp/prowlarr-backup-upload.zip".into()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn ensure_prowlarr_indexer_proxy(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_indexerproxy", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-flaresolverr")
                && item.get("implementation").and_then(Value::as_str) == Some("FlareSolverr")
        })
    }) {
        return Ok(());
    }
    let body = prowlarr_indexer_proxy_body("rustarr-live-flaresolverr".into());
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_indexerproxy", "--args", &args])?;
    Ok(())
}

fn ensure_prowlarr_custom_script_notification(
    rustarr: &process::RustarrProcess,
    svc: &str,
) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_notification", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-script")
                && item.get("implementation").and_then(Value::as_str) == Some("CustomScript")
        })
    }) {
        return Ok(());
    }
    let body = prowlarr_notification_body("rustarr-live-script".into());
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_notification", "--args", &args])?;
    Ok(())
}

fn ensure_radarr_fixtures(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    ensure_radarr_filesystem_prereqs()?;
    ensure_radarr_qbittorrent_download_client(rustarr, svc)?;
    ensure_radarr_newznab_indexer(rustarr, svc)?;
    ensure_radarr_custom_script_notification(rustarr, svc)?;
    ensure_radarr_remote_path_mapping(rustarr, svc)?;
    ensure_radarr_autotagging(rustarr, svc)
}

fn ensure_radarr_filesystem_prereqs() -> Result<()> {
    let status = Command::new("ssh")
        .arg("shart")
        .arg("docker exec radarr sh -lc 'mkdir -p \"/data/media/movies/The Matrix (1999)\"'")
        .status()
        .context("prepare Radarr live filesystem fixtures on shart")?;
    anyhow::ensure!(
        status.success(),
        "prepare Radarr live filesystem fixtures on shart failed with {status}"
    );
    Ok(())
}

fn ensure_radarr_qbittorrent_download_client(
    rustarr: &process::RustarrProcess,
    svc: &str,
) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_downloadclient", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-qbit")
                && item.get("implementation").and_then(Value::as_str) == Some("QBittorrent")
        })
    }) {
        return Ok(());
    }
    let body = json!({
        "enable": true,
        "protocol": "torrent",
        "priority": 1,
        "removeCompletedDownloads": false,
        "removeFailedDownloads": false,
        "name": "rustarr-live-qbit",
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
            {"name": "movieCategory", "value": "radarr"},
            {"name": "movieImportedCategory", "value": ""},
            {"name": "recentMoviePriority", "value": 0},
            {"name": "olderMoviePriority", "value": 0},
            {"name": "initialState", "value": 0},
            {"name": "sequentialOrder", "value": false},
            {"name": "firstAndLast", "value": false},
            {"name": "contentLayout", "value": 0}
        ],
        "tags": []
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_downloadclient", "--args", &args])?;
    Ok(())
}

fn ensure_radarr_newznab_indexer(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_indexer", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-newznab")
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
        "protocol": "torrent",
        "priority": 1,
        "name": "rustarr-live-newznab",
        "implementation": "Torznab",
        "implementationName": "Torznab",
        "configContract": "TorznabSettings",
        "fields": [
            {"name": "baseUrl", "value": "http://127.0.0.1:9"},
            {"name": "apiPath", "value": "/api"},
            {"name": "apiKey", "value": "rustarr-live"},
            {"name": "categories", "value": [2000, 2010]},
            {"name": "animeCategories", "value": []},
            {"name": "additionalParameters", "value": ""},
            {"name": "multiLanguages", "value": []}
        ],
        "tags": []
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_indexer", "--args", &args])?;
    Ok(())
}

fn ensure_radarr_custom_script_notification(
    rustarr: &process::RustarrProcess,
    svc: &str,
) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_notification", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-script")
                && item.get("implementation").and_then(Value::as_str) == Some("CustomScript")
        })
    }) {
        return Ok(());
    }
    let body = json!({
        "name": "rustarr-live-script",
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
        "onMovieAdded": false,
        "onMovieDelete": false,
        "onMovieFileDelete": false,
        "onMovieFileDeleteForUpgrade": false,
        "onHealthIssue": false,
        "onHealthRestored": false,
        "onApplicationUpdate": false,
        "onManualInteractionRequired": false,
        "includeHealthWarnings": false,
        "tags": []
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_notification", "--args", &args])?;
    Ok(())
}

fn ensure_radarr_remote_path_mapping(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_remotepathmapping", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("host").and_then(Value::as_str) == Some("rustarr-live-host")
                && item.get("remotePath").and_then(Value::as_str) == Some("/downloads/")
                && item.get("localPath").and_then(Value::as_str) == Some("/data/media/movies/")
        })
    }) {
        return Ok(());
    }
    let body = json!({
        "host": "rustarr-live-host",
        "remotePath": "/downloads/",
        "localPath": "/data/media/movies/"
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_remotepathmapping", "--args", &args])?;
    Ok(())
}

fn ensure_radarr_autotagging(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_autotagging", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item.get("name").and_then(Value::as_str) == Some("rustarr-live-autotag"))
    }) {
        return Ok(());
    }
    let body = radarr_autotagging_body("rustarr-live-autotag".into());
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_autotagging", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_download_client(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    ensure_sonarr_filesystem_prereqs()?;
    ensure_sonarr_tag(rustarr, svc)?;
    ensure_sonarr_root_folder(rustarr, svc)?;
    ensure_sonarr_series(rustarr, svc)?;
    ensure_sonarr_qbittorrent_download_client(rustarr, svc)?;
    ensure_sonarr_newznab_indexer(rustarr, svc)?;
    ensure_sonarr_custom_script_notification(rustarr, svc)?;
    ensure_sonarr_remote_path_mapping(rustarr, svc)?;
    ensure_sonarr_autotagging(rustarr, svc)?;
    ensure_sonarr_import_list(rustarr, svc)?;
    ensure_sonarr_backup(rustarr, svc)
}

fn ensure_sonarr_tag(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_tag", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item.get("label").and_then(Value::as_str) == Some("rustarr-live"))
    }) {
        return Ok(());
    }
    let args = serde_json::to_string(&json!({ "body": { "label": "rustarr-live" } }))?;
    rustarr.json(&[svc, "op", "post_tag", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_root_folder(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_rootfolder", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item.get("path").and_then(Value::as_str) == Some("/data/media/tv"))
    }) {
        return Ok(());
    }
    let args = serde_json::to_string(&json!({ "body": { "path": "/data/media/tv" } }))?;
    rustarr.json(&[svc, "op", "post_rootfolder", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_series(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_series", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item.get("tvdbId").and_then(Value::as_i64) == Some(403245))
    }) {
        return Ok(());
    }
    let lookup = rustarr.json(&[
        svc,
        "op",
        "get_series_lookup",
        "--args",
        r#"{"term":"tvdb:403245"}"#,
    ])?;
    let mut series = lookup
        .as_array()
        .and_then(|items| items.first())
        .cloned()
        .context("Sonarr lookup returned no Silo fixture candidate")?;
    let quality_profile_id = sonarr_quality_profile_id(rustarr, svc)?;
    let language_profile_id = sonarr_language_profile_id(rustarr, svc)?;
    if let Some(obj) = series.as_object_mut() {
        obj.remove("id");
        obj.insert("qualityProfileId".into(), json!(quality_profile_id));
        obj.insert("languageProfileId".into(), json!(language_profile_id));
        obj.insert("rootFolderPath".into(), json!("/data/media/tv"));
        obj.insert("path".into(), json!("/data/media/tv/Silo"));
        obj.insert("monitored".into(), json!(false));
        obj.insert("seasonFolder".into(), json!(true));
        obj.insert("seriesType".into(), json!("standard"));
        obj.insert("tags".into(), json!([]));
        obj.insert(
            "addOptions".into(),
            json!({
                "monitor": "none",
                "searchForMissingEpisodes": false,
                "searchForCutoffUnmetEpisodes": false
            }),
        );
    }
    let args = serde_json::to_string(&json!({ "body": series }))?;
    rustarr.json(&[svc, "op", "post_series", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_qbittorrent_download_client(
    rustarr: &process::RustarrProcess,
    svc: &str,
) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_downloadclient", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-qbit")
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
        "name": "rustarr-live-qbit",
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
    rustarr.json(&[svc, "op", "post_downloadclient", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_filesystem_prereqs() -> Result<()> {
    let status = Command::new("ssh")
        .arg("shart")
        .arg(
            r##"docker exec sonarr sh -lc 'mkdir -p /data/media/tv/Silo \
             && printf "#!/bin/sh\nexit 0\n" > /data/media/tv/rustarr-live-notify.sh \
             && chmod +x /data/media/tv/rustarr-live-notify.sh'
cat >/tmp/rustarr-sonarr-newznab.py <<'PY'
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs
class H(BaseHTTPRequestHandler):
    def do_GET(self):
        q=parse_qs(urlparse(self.path).query)
        t=(q.get('t') or [''])[0]
        if self.path.startswith('/download'):
            self.send_response(200); self.send_header('Content-Type','application/x-nzb'); self.end_headers(); self.wfile.write(b'<?xml version="1.0"?><nzb xmlns="http://www.newzbin.com/DTD/2003/nzb"></nzb>'); return
        if t=='caps':
            body='''<?xml version="1.0" encoding="UTF-8"?><caps><server title="rustarr-sonarr"/><limits max="100" default="100"/><searching><search available="yes" supportedParams="q"/><tv-search available="yes" supportedParams="q,season,ep"/><movie-search available="yes" supportedParams="q"/></searching><categories><category id="5000" name="TV"><subcat id="5030" name="TV/SD"/><subcat id="5040" name="TV/HD"/></category></categories></caps>'''
        else:
            body='''<?xml version="1.0" encoding="UTF-8"?><rss version="2.0" xmlns:newznab="http://www.newznab.com/DTD/2010/feeds/attributes/"><channel><title>rustarr-sonarr</title><item><title>Silo S01E01 RustarrLive 1080p</title><guid isPermaLink="false">sonarr-rustarr-fixture-1</guid><link>http://100.118.209.1:18081/download/rustarr.nzb</link><comments>http://100.118.209.1:18081/details/1</comments><pubDate>Sat, 04 Jul 2026 00:00:00 GMT</pubDate><category>TV</category><size>1048576</size><enclosure url="http://100.118.209.1:18081/download/rustarr.nzb" length="1048576" type="application/x-nzb"/><newznab:attr name="category" value="5040"/><newznab:attr name="size" value="1048576"/><newznab:attr name="guid" value="sonarr-rustarr-fixture-1"/></item></channel></rss>'''
        self.send_response(200); self.send_header('Content-Type','application/xml'); self.end_headers(); self.wfile.write(body.encode())
    def log_message(self,*a): pass
HTTPServer(('0.0.0.0',18081), H).serve_forever()
PY
fuser -k 18081/tcp >/dev/null 2>&1 || true
nohup python3 /tmp/rustarr-sonarr-newznab.py >/tmp/rustarr-sonarr-newznab.log 2>&1 &
sleep 0.5"##,
        )
        .status()
        .context("prepare Sonarr live filesystem fixtures on shart")?;
    anyhow::ensure!(
        status.success(),
        "prepare Sonarr live filesystem fixtures on shart failed with {status}"
    );
    Ok(())
}

fn ensure_sonarr_newznab_indexer(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_indexer", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-newznab")
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
        "name": "rustarr-live-newznab",
        "implementation": "Torznab",
        "implementationName": "Torznab",
        "configContract": "TorznabSettings",
        "fields": [
            {"name": "baseUrl", "value": "http://100.118.209.1:18081"},
            {"name": "apiPath", "value": "/api"},
            {"name": "apiKey", "value": "rustarr-live"},
            {"name": "categories", "value": [5000, 5020, 5030, 5040, 5045, 5050]},
            {"name": "animeCategories", "value": [5070]},
            {"name": "animeStandardFormatSearch", "value": false},
            {"name": "additionalParameters", "value": ""},
            {"name": "multiLanguages", "value": []},
            {"name": "failDownloads", "value": []}
        ],
        "tags": []
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_indexer", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_custom_script_notification(
    rustarr: &process::RustarrProcess,
    svc: &str,
) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_notification", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("name").and_then(Value::as_str) == Some("rustarr-live-script")
                && item.get("implementation").and_then(Value::as_str) == Some("CustomScript")
        })
    }) {
        return Ok(());
    }
    let body = json!({
        "name": "rustarr-live-script",
        "implementation": "CustomScript",
        "implementationName": "Custom Script",
        "configContract": "CustomScriptSettings",
        "fields": [
            {"name": "path", "value": "/data/media/tv/rustarr-live-notify.sh"},
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
    rustarr.json(&[svc, "op", "post_notification", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_remote_path_mapping(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_remotepathmapping", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.get("host").and_then(Value::as_str) == Some("rustarr-live-host")
                && item.get("remotePath").and_then(Value::as_str) == Some("/downloads/")
                && item.get("localPath").and_then(Value::as_str) == Some("/data/media/tv/")
        })
    }) {
        return Ok(());
    }
    let body = json!({
        "host": "rustarr-live-host",
        "remotePath": "/downloads/",
        "localPath": "/data/media/tv/"
    });
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_remotepathmapping", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_autotagging(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_autotagging", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item.get("name").and_then(Value::as_str) == Some("rustarr-live-autotag"))
    }) {
        return Ok(());
    }
    let body = json!({
        "name": "rustarr-live-autotag",
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
    rustarr.json(&[svc, "op", "post_autotagging", "--args", &args])?;
    Ok(())
}

fn ensure_sonarr_import_list(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    let existing = rustarr.json(&[svc, "op", "get_importlist", "--args", "{}"])?;
    if existing.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item.get("name").and_then(Value::as_str) == Some("rustarr-live-custom"))
    }) {
        return Ok(());
    }
    let mut body = sonarr_import_list_body("rustarr-live-custom".into());
    if let Some(obj) = body.as_object_mut() {
        obj.insert(
            "qualityProfileId".into(),
            json!(sonarr_quality_profile_id(rustarr, svc)?),
        );
        obj.insert(
            "languageProfileId".into(),
            json!(sonarr_language_profile_id(rustarr, svc)?),
        );
    }
    let args = serde_json::to_string(&json!({ "body": body }))?;
    rustarr.json(&[svc, "op", "post_importlist", "--args", &args])?;
    Ok(())
}

fn sonarr_quality_profile_id(rustarr: &process::RustarrProcess, svc: &str) -> Result<i64> {
    let profiles = rustarr.json(&[svc, "op", "get_qualityprofile", "--args", "{}"])?;
    profiles
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("id"))
        .and_then(Value::as_i64)
        .context("Sonarr has no quality profile fixture")
}

fn sonarr_language_profile_id(rustarr: &process::RustarrProcess, svc: &str) -> Result<i64> {
    let profiles = rustarr.json(&[svc, "op", "get_languageprofile", "--args", "{}"])?;
    Ok(profiles
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("id"))
        .and_then(Value::as_i64)
        .unwrap_or(1))
}

fn ensure_sonarr_backup(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    for _ in 0..2 {
        rustarr.json(&[
            svc,
            "op",
            "post_command",
            "--args",
            r#"{"body":{"name":"Backup","type":"manual"}}"#,
        ])?;
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    let status = Command::new("ssh")
        .arg("shart")
        .arg(
            "docker exec sonarr sh -lc 'latest=$(ls -t /config/Backups/manual/*.zip 2>/dev/null | head -1); test -n \"$latest\" && cat \"$latest\"' > /tmp/rustarr-sonarr-backup-upload.zip",
        )
        .status()
        .context("copy Sonarr backup fixture on shart")?;
    anyhow::ensure!(
        status.success(),
        "copy Sonarr backup fixture on shart failed with {status}"
    );
    let local_path = sonarr_backup_upload_fixture_path();
    if let Some(parent) = std::path::Path::new(&local_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = Command::new("scp")
        .arg("shart:/tmp/rustarr-sonarr-backup-upload.zip")
        .arg(&local_path)
        .status()
        .context("copy Sonarr backup fixture locally")?;
    anyhow::ensure!(
        status.success(),
        "copy Sonarr backup fixture locally failed with {status}"
    );
    Ok(())
}

/// Bounded thread pool: run `ops` through `run_op` concurrently. Returns, per op,
/// `(op, result, success-body)` so the caller can harvest seeded ids between phases.
pub(super) type RunOut = (&'static OperationSpec, OpResult, Option<Value>);

fn parallel_run(
    rustarr: &process::RustarrProcess,
    svc: &str,
    kind: ServiceKind,
    spec: &Spec,
    fixtures: &FixtureStore,
    ops: &[&'static OperationSpec],
    no_destructive: bool,
) -> Vec<RunOut> {
    // Keep contract execution serial. This suite is the authoritative endpoint
    // coverage gate; concurrent generated writes made the shart services drop
    // connections and produced false "coverage" failures before the endpoint
    // itself could be evaluated.
    const WORKERS: usize = 1;
    if ops.is_empty() {
        return Vec::new();
    }
    let chunk = ops.len().div_ceil(WORKERS).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = ops
            .chunks(chunk)
            .map(|c| {
                s.spawn(move || {
                    c.iter()
                        .map(|op| {
                            let (r, v) =
                                run_op(rustarr, svc, kind, spec, op, fixtures, no_destructive);
                            (*op, r, v)
                        })
                        .collect::<Vec<RunOut>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("contract worker panicked"))
            .collect()
    })
}

#[derive(Debug, Default)]
pub(super) struct FixtureStore {
    ids: BTreeMap<String, Vec<Value>>,
    bodies: BTreeMap<String, Vec<Value>>,
}

impl FixtureStore {
    fn values_for(&self, path: &str) -> Option<&[Value]> {
        self.ids.get(path).map(Vec::as_slice)
    }

    fn body_for(&self, path: &str) -> Option<&Value> {
        self.bodies.get(path).and_then(|v| v.first())
    }
}

/// Merge resource fixtures a phase's responses expose into the fixture pool, keyed
/// by the op's path. Array/object GETs contribute real `id`/`Id`/`ratingKey`
/// values plus reusable object bodies. Later path-param/body ops must resolve
/// against this pool; the harness no longer fabricates synthetic IDs.
pub(super) fn harvest_into(fixtures: &mut FixtureStore, outs: &[RunOut]) {
    for (op, _result, value) in outs {
        let Some(value) = value else { continue };
        let bodies = harvest_objects_for_op(op, value);
        let mut found = bodies
            .iter()
            .flat_map(harvest_id_values)
            .collect::<Vec<_>>();
        if let Some(id) = first_id_value(value) {
            found.push(id);
        }
        if !found.is_empty() {
            let pool = fixtures.ids.entry(op.path.to_string()).or_default();
            pool.extend(found);
            dedupe_values(pool);
            pool.truncate(8);
        }
        if !bodies.is_empty() {
            let pool = fixtures.bodies.entry(op.path.to_string()).or_default();
            pool.extend(bodies);
            pool.truncate(8);
        }
    }
}

/// Seeding phase for an op: collection reads first (0) to discover ids, then creates
/// (1) to seed more, then resource reads/updates (2) that consume ids, then deletes
/// (3) last so reads/updates precede cleanup.
pub(super) fn seed_phase(op: &OperationSpec) -> u8 {
    if matches!(op.name, "get_moviefile" | "get_rename") {
        return 3;
    }
    match op.method {
        HttpMethod::Get
            if op.path_params.is_empty()
                && !op
                    .query_params
                    .iter()
                    .any(|param| should_seed_optional_query(param)) =>
        {
            0
        }
        HttpMethod::Get if op.path_params.is_empty() => 1,
        HttpMethod::Post => 2,
        HttpMethod::Delete => 4,
        _ => 3, // GET-with-id, PUT, PATCH
    }
}

fn tally(results: &[OpResult]) -> (usize, usize, usize, usize) {
    let mut t = (0, 0, 0, 0);
    for r in results {
        match r.outcome {
            "ok" => t.0 += 1,
            "schema_mismatch" => t.1 += 1,
            "rejected" => t.2 += 1,
            _ => t.3 += 1,
        }
    }
    t
}

pub(super) fn contract_status(results: &[OpResult]) -> ContractStatus {
    let (ok, schema_mismatch, rejected, skipped) = tally(results);
    let total = results.len();
    let passed = rejected == 0 && schema_mismatch == 0 && skipped == 0 && ok > 0;
    let detail = format!(
        "{ok} contract-ok, {schema_mismatch} schema-mismatch, {rejected} contract-rejected (fails coverage), {skipped} skipped of {total} ops"
    );
    ContractStatus {
        ok,
        schema_mismatch,
        rejected,
        skipped,
        total,
        passed,
        detail,
    }
}

fn harvest_objects(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter(|v| v.is_object())
            .cloned()
            .collect::<Vec<_>>(),
        Value::Object(map) => {
            if let Some(items) = map.values().find_map(Value::as_array) {
                items
                    .iter()
                    .filter(|v| v.is_object())
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                vec![value.clone()]
            }
        }
        _ => Vec::new(),
    }
}

fn harvest_objects_for_op(op: &OperationSpec, value: &Value) -> Vec<Value> {
    let mut bodies = harvest_objects(value);
    if op.path.starts_with("/library/")
        && let Some(media_container) = value.get("MediaContainer")
    {
        for key in ["Directory", "Metadata", "Playlist"] {
            if let Some(items) = media_container.get(key).and_then(Value::as_array) {
                bodies.extend(items.iter().filter(|v| v.is_object()).cloned());
            }
        }
    }
    bodies
}

fn harvest_id_values(value: &Value) -> Vec<Value> {
    match value {
        Value::Object(_) => first_id_value(value).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn first_id_value(value: &Value) -> Option<Value> {
    let obj = value.as_object()?;
    for key in ["id", "Id", "ID", "ratingKey", "key", "Guid", "guid"] {
        if let Some(v) = obj.get(key).filter(|v| is_scalar(v)) {
            return Some(v.clone());
        }
    }
    None
}

fn dedupe_values(values: &mut Vec<Value>) {
    let mut seen = std::collections::BTreeSet::new();
    values.retain(|value| seen.insert(value.to_string()));
}

fn is_scalar(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_))
}

/// Run one op. Returns its classified result plus the successful response body (so
/// the caller can harvest resource ids for create-first seeding).
fn run_op(
    rustarr: &process::RustarrProcess,
    svc: &str,
    kind: ServiceKind,
    spec: &Spec,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    no_destructive: bool,
) -> (OpResult, Option<Value>) {
    let mk = |outcome, detail: String| OpResult {
        name: op.name,
        method: op.method.as_str(),
        path: op.path,
        outcome,
        detail,
        args: None,
    };
    let args = match prepare_op_args(kind, spec, op, fixtures, no_destructive, false) {
        PreparedOp::Call(args) => args,
        PreparedOp::Skip(detail) => {
            return (
                mk("rejected", format!("missing executable fixture: {detail}")),
                None,
            );
        }
    };
    // DELETE confirmation is handled by RUSTARR_ALLOW_DESTRUCTIVE on the test stack;
    // pass --confirm too so it works whether or not the env is set.
    match invoke(rustarr, svc, op.name, &args, op.method.is_delete()) {
        Ok(Some(value)) => {
            let result = match op.response_type {
                Some(ty) => match spec.validate_response(ty, &value) {
                    Ok(()) => mk("ok", format!("2xx + matches {ty}")),
                    Err(e) => mk(
                        "schema_mismatch",
                        format!("{e}").chars().take(180).collect(),
                    ),
                },
                None => mk("ok", "2xx (no declared response type to validate)".into()),
            };
            (result, Some(value))
        }
        Ok(None) => (mk("ok", "2xx (empty/non-JSON body)".into()), None),
        Err(e) => (
            mk("rejected", format!("{e}").chars().take(180).collect()),
            None,
        ),
    }
}

pub(super) enum PreparedOp {
    Call(Map<String, Value>),
    Skip(String),
}

pub(super) fn prepare_op_args(
    kind: ServiceKind,
    spec: &Spec,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    no_destructive: bool,
    allow_reset_required: bool,
) -> PreparedOp {
    if no_destructive && op.method.is_delete() {
        return PreparedOp::Skip("destructive (DELETE) skipped via --no-destructive".into());
    }
    if op_is_self_destructive_control(op) && !allow_reset_required {
        return PreparedOp::Skip("self-destructive control endpoint skipped".into());
    }
    // Reset-required endpoints are run by the reset phase, not the normal phase,
    // so self-destructive controls and config/auth mutations cannot poison later
    // operations in the same service run.
    if op_requires_stack_reset(op) && !allow_reset_required {
        return PreparedOp::Skip(
            "requires stack reset/reseed (control endpoint or config/auth mutation)".into(),
        );
    }
    // Generated OpenAPI callables must be invoked even when the response is HTML,
    // XML, empty, or otherwise non-JSON. The validator handles non-JSON success
    // bodies as an exercised contract; no endpoint is skipped for content type.
    // Satisfy path params from discovered/seeded fixtures (parent collection =
    // path before `{`). No fallback IDs: a contract call that needs a resource but
    // has no resource fixture is not a meaningful live test.
    let mut path_args = Map::new();
    if !op.path_params.is_empty() {
        let parent = fixture_parent_path(op.path);
        for p in op.path_params {
            let Some(value) = fixture_path_param_value(kind, fixtures, parent, p) else {
                return PreparedOp::Skip(format!(
                    "no live fixture for path param `{p}` under `{parent}`"
                ));
            };
            path_args.insert((*p).to_string(), value);
        }
    }
    let Some(mut args) = spec.build_args(op.method.as_str(), op.path, &path_args) else {
        return PreparedOp::Call(path_args);
    };
    if op.path.contains("{ids}") {
        args.insert("ids".into(), json!(1));
    }
    apply_fixture_args(kind, op, fixtures, &mut args);
    if op.path.contains("{ids}") {
        args.insert("ids".into(), json!(1));
    }
    if op.has_body
        && let Some(body) = live_fixture_body_for_op(kind, op)
    {
        args.insert("body".into(), body);
        apply_fixture_body_args(kind, op, fixtures, &mut args);
        return PreparedOp::Call(args);
    }
    if op.has_body && can_reuse_fixture_body(op) {
        if let Some(body) = fixture_body_for_op(fixtures, op) {
            args.insert("body".into(), body.clone());
        }
    }
    apply_fixture_body_args(kind, op, fixtures, &mut args);
    PreparedOp::Call(args)
}

fn can_reuse_fixture_body(op: &OperationSpec) -> bool {
    matches!(op.method, HttpMethod::Put | HttpMethod::Patch)
        || op.path.ends_with("/test")
        || op.path.contains("/test/")
        || op.path.contains("/action/")
}

fn live_fixture_body_for_op(kind: ServiceKind, op: &OperationSpec) -> Option<Value> {
    match (kind, op.name) {
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_command") => {
            Some(json!({ "name": "RefreshMonitoredDownloads" }))
        }
        (ServiceKind::Sonarr, "post_autotagging") => {
            Some(sonarr_autotagging_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Sonarr, "post_customformat") => {
            Some(sonarr_custom_format_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Sonarr, "post_delayprofile") => Some(sonarr_delay_profile_body()),
        (ServiceKind::Sonarr, "post_downloadclient" | "post_downloadclient_test") => Some(
            sonarr_download_client_body(unique_live_label(kind, op.name)),
        ),
        (ServiceKind::Sonarr, "post_importlist" | "post_importlist_test") => {
            Some(sonarr_import_list_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Sonarr, "post_indexer" | "post_indexer_test") => {
            Some(sonarr_indexer_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Sonarr, "post_metadata") => {
            Some(sonarr_metadata_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Sonarr, "post_notification" | "post_notification_test") => {
            Some(sonarr_notification_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Sonarr, "post_releaseprofile") => Some(sonarr_release_profile_body(
            unique_live_label(kind, op.name),
        )),
        (ServiceKind::Sonarr, "post_remotepathmapping") => Some(json!({
            "host": unique_live_label(kind, op.name),
            "remotePath": "/downloads/",
            "localPath": "/data/media/tv/"
        })),
        (ServiceKind::Sonarr, "post_rootfolder") => Some(json!({ "path": "/data/media/tv" })),
        (ServiceKind::Sonarr, "post_series") => Some(sonarr_series_body()),
        (ServiceKind::Sonarr, "post_series_import") => Some(json!([sonarr_series_body()])),
        (ServiceKind::Sonarr, "post_seasonpass") => Some(sonarr_seasonpass_body()),
        (ServiceKind::Radarr, "post_autotagging") => {
            Some(radarr_autotagging_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Radarr, "post_downloadclient" | "post_downloadclient_test") => Some(
            radarr_download_client_body(unique_live_label(kind, op.name)),
        ),
        (ServiceKind::Radarr, "post_indexer" | "post_indexer_test") => {
            Some(radarr_indexer_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Radarr, "post_notification" | "post_notification_test") => {
            Some(radarr_notification_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Radarr, "post_remotepathmapping") => Some(json!({
            "host": unique_live_label(kind, op.name),
            "remotePath": "/downloads/",
            "localPath": "/data/media/movies/"
        })),
        (ServiceKind::Radarr, "post_rootfolder") => Some(json!({ "path": "/data/media/movies" })),
        (ServiceKind::Radarr, "post_tag") => {
            Some(json!({ "label": unique_live_label(kind, op.name) }))
        }
        (ServiceKind::Sonarr | ServiceKind::Prowlarr, "post_tag") => {
            Some(json!({ "label": unique_live_label(kind, op.name) }))
        }
        (ServiceKind::Prowlarr, "post_applications" | "post_applications_test") => Some(
            prowlarr_sonarr_application_body(unique_live_label(kind, op.name)),
        ),
        (ServiceKind::Prowlarr, "post_appprofile") => {
            Some(prowlarr_app_profile_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Prowlarr, "post_command") => Some(json!({ "name": "CheckHealth" })),
        (ServiceKind::Prowlarr, "post_downloadclient" | "post_downloadclient_test") => Some(
            prowlarr_download_client_body(unique_live_label(kind, op.name)),
        ),
        (ServiceKind::Prowlarr, "post_indexer" | "post_indexer_test") => {
            Some(prowlarr_indexer_body(unique_live_label(kind, op.name)))
        }
        (ServiceKind::Prowlarr, "post_indexerproxy" | "post_indexerproxy_test") => Some(
            prowlarr_indexer_proxy_body(unique_live_label(kind, op.name)),
        ),
        (ServiceKind::Prowlarr, "post_notification" | "post_notification_test") => {
            Some(prowlarr_notification_body(unique_live_label(kind, op.name)))
        }
        _ => None,
    }
}

fn live_env_value(key: &str) -> Option<String> {
    if let Ok(value) = std::env::var(key) {
        return Some(value);
    }
    let file = std::fs::read_to_string(super::guard::DEFAULT_ENV_FILE).ok()?;
    file.lines().find_map(|line| {
        let (candidate, value) = line.split_once('=')?;
        (candidate == key).then(|| value.trim_matches('"').to_string())
    })
}

fn prowlarr_sonarr_application_body(name: String) -> Value {
    let sonarr_url =
        live_env_value("RUSTARR_SONARR_URL").unwrap_or_else(|| "http://100.118.209.1:8989".into());
    let sonarr_api_key =
        live_env_value("RUSTARR_SONARR_API_KEY").unwrap_or_else(|| "rustarr-live".into());
    let prowlarr_url = live_env_value("RUSTARR_PROWLARR_URL")
        .unwrap_or_else(|| "http://100.118.209.1:9696".into());
    json!({
        "name": name,
        "implementation": "Sonarr",
        "implementationName": "Sonarr",
        "configContract": "SonarrSettings",
        "enable": false,
        "syncLevel": "disabled",
        "fields": [
            {"name": "prowlarrUrl", "value": prowlarr_url},
            {"name": "baseUrl", "value": sonarr_url},
            {"name": "apiKey", "value": sonarr_api_key},
            {"name": "syncCategories", "value": [5000]},
            {"name": "animeSyncCategories", "value": [5070]},
            {"name": "syncAnimeStandardFormatSearch", "value": true},
            {"name": "syncRejectBlocklistedTorrentHashesWhileGrabbing", "value": false}
        ],
        "tags": []
    })
}

fn prowlarr_download_client_body(name: String) -> Value {
    json!({
        "name": name,
        "implementation": "TorrentBlackhole",
        "implementationName": "Torrent Blackhole",
        "configContract": "TorrentBlackholeSettings",
        "enable": true,
        "priority": 1,
        "protocol": "torrent",
        "supportsCategories": false,
        "categories": [],
        "fields": [
            {"name": "torrentFolder", "value": "/tmp/rustarr-prowlarr-torrents"},
            {"name": "saveMagnetFiles", "value": false},
            {"name": "magnetFileExtension", "value": ".magnet"}
        ],
        "tags": []
    })
}

fn prowlarr_app_profile_body(name: String) -> Value {
    json!({
        "name": name,
        "enableRss": true,
        "enableAutomaticSearch": true,
        "enableInteractiveSearch": true,
        "minimumSeeders": 1
    })
}

fn prowlarr_indexer_body(name: String) -> Value {
    json!({
        "name": name,
        "implementation": "Torznab",
        "implementationName": "Torznab",
        "configContract": "TorznabSettings",
        "enable": true,
        "appProfileId": 1,
        "downloadClientId": 0,
        "protocol": "torrent",
        "privacy": "public",
        "priority": 25,
        "redirect": true,
        "supportsPagination": false,
        "supportsRedirect": false,
        "supportsRss": true,
        "supportsSearch": true,
        "fields": [
            {"name": "baseUrl", "value": "http://100.118.209.1:18080/"},
            {"name": "apiPath", "value": "/api"},
            {"name": "apiKey", "value": "rustarr-live"},
            {"name": "categories", "value": [2000]},
            {"name": "animeCategories", "value": []},
            {"name": "additionalParameters", "value": ""},
            {"name": "multiLanguages", "value": []},
            {"name": "baseSettings.limitsUnit", "value": 0},
            {"name": "torrentBaseSettings.preferMagnetUrl", "value": false}
        ],
        "tags": []
    })
}

fn prowlarr_indexer_proxy_body(name: String) -> Value {
    json!({
        "name": name,
        "implementation": "FlareSolverr",
        "implementationName": "FlareSolverr",
        "configContract": "FlareSolverrSettings",
        "fields": [
            {"name": "host", "value": "http://127.0.0.1:8191/"},
            {"name": "requestTimeout", "value": 1}
        ],
        "tags": []
    })
}

fn prowlarr_notification_body(name: String) -> Value {
    json!({
        "name": name,
        "implementation": "CustomScript",
        "implementationName": "Custom Script",
        "configContract": "CustomScriptSettings",
        "fields": [
            {"name": "path", "value": "/config/rustarr-live-notify.sh"},
            {"name": "arguments", "value": ""}
        ],
        "onGrab": false,
        "onHealthIssue": false,
        "onHealthRestored": false,
        "onApplicationUpdate": false,
        "includeHealthWarnings": false,
        "includeManualGrabs": false,
        "tags": []
    })
}

fn sonarr_autotagging_body(name: String) -> Value {
    json!({
        "name": name,
        "removeTagsAutomatically": false,
        "tags": [2],
        "specifications": [{
            "name": "Monitored",
            "implementation": "MonitoredSpecification",
            "implementationName": "Monitored",
            "negate": false,
            "required": false,
            "fields": []
        }]
    })
}

fn sonarr_custom_format_body(name: String) -> Value {
    json!({
        "name": name,
        "includeCustomFormatWhenRenaming": false,
        "specifications": [{
            "name": "RustarrLive",
            "implementation": "ReleaseTitleSpecification",
            "implementationName": "Release Title",
            "negate": false,
            "required": true,
            "fields": [{"name": "value", "value": "RustarrLive"}]
        }]
    })
}

fn sonarr_delay_profile_body() -> Value {
    json!({
        "enableUsenet": true,
        "enableTorrent": true,
        "preferredProtocol": "usenet",
        "usenetDelay": 0,
        "torrentDelay": 0,
        "bypassIfHighestQuality": true,
        "bypassIfAboveCustomFormatScore": false,
        "minimumCustomFormatScore": 0,
        "tags": [2]
    })
}

fn sonarr_download_client_body(name: String) -> Value {
    json!({
        "enable": false,
        "protocol": "torrent",
        "priority": 1,
        "removeCompletedDownloads": false,
        "removeFailedDownloads": false,
        "name": name,
        "implementation": "QBittorrent",
        "implementationName": "qBittorrent",
        "configContract": "QBittorrentSettings",
        "fields": [
            {"name": "host", "value": "100.120.242.29"},
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
    })
}

fn sonarr_indexer_body(name: String) -> Value {
    json!({
        "enableRss": false,
        "enableAutomaticSearch": false,
        "enableInteractiveSearch": false,
        "supportsRss": true,
        "supportsSearch": true,
        "protocol": "usenet",
        "priority": 1,
        "name": name,
        "implementation": "Newznab",
        "implementationName": "Newznab",
        "configContract": "NewznabSettings",
        "fields": [
            {"name": "baseUrl", "value": "http://100.118.209.1:18081"},
            {"name": "apiPath", "value": "/api"},
            {"name": "apiKey", "value": "rustarr-live"},
            {"name": "categories", "value": [5000, 5020, 5030, 5040, 5045, 5050]},
            {"name": "animeCategories", "value": [5070]},
            {"name": "animeStandardFormatSearch", "value": false},
            {"name": "additionalParameters", "value": ""},
            {"name": "multiLanguages", "value": []},
            {"name": "failDownloads", "value": []}
        ],
        "tags": []
    })
}

fn sonarr_metadata_body(name: String) -> Value {
    json!({
        "name": name,
        "enable": false,
        "implementation": "XbmcMetadata",
        "implementationName": "Kodi (XBMC) / Emby",
        "configContract": "XbmcMetadataSettings",
        "fields": [
            {"name": "seriesMetadata", "value": true},
            {"name": "seriesMetadataEpisodeGuide", "value": false},
            {"name": "seriesMetadataUrl", "value": false},
            {"name": "episodeMetadata", "value": true},
            {"name": "episodeImageThumb", "value": false},
            {"name": "seriesImages", "value": true},
            {"name": "seasonImages", "value": true},
            {"name": "episodeImages", "value": true}
        ],
        "tags": []
    })
}

fn sonarr_notification_body(name: String) -> Value {
    json!({
        "name": name,
        "implementation": "CustomScript",
        "implementationName": "Custom Script",
        "configContract": "CustomScriptSettings",
        "fields": [
            {"name": "path", "value": "/data/media/tv/rustarr-live-notify.sh"},
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
    })
}

fn sonarr_release_profile_body(name: String) -> Value {
    json!({
        "name": name,
        "enabled": true,
        "required": ["RustarrLive"],
        "ignored": [],
        "preferred": [],
        "includePreferredWhenRenaming": false,
        "indexerId": 0,
        "tags": []
    })
}

fn sonarr_import_list_body(name: String) -> Value {
    json!({
        "name": name,
        "enableAutomaticAdd": false,
        "shouldMonitor": "none",
        "monitorNewItems": "none",
        "rootFolderPath": "/data/media/tv",
        "qualityProfileId": 1,
        "languageProfileId": 1,
        "seriesType": "standard",
        "seasonFolder": true,
        "listType": "program",
        "listOrder": 0,
        "searchForMissingEpisodes": false,
        "implementation": "CustomImport",
        "implementationName": "Custom List",
        "configContract": "CustomSettings",
        "fields": [
            {"name": "baseUrl", "value": "http://100.118.209.1:18080/sonarr-importlist"}
        ],
        "tags": []
    })
}

fn sonarr_series_body() -> Value {
    json!({
        "title": "Silo",
        "titleSlug": "silo",
        "tvdbId": 403245,
        "qualityProfileId": 1,
        "languageProfileId": 1,
        "rootFolderPath": "/data/media/tv",
        "path": "/data/media/tv/Silo",
        "monitored": false,
        "monitorNewItems": "none",
        "seasonFolder": true,
        "seriesType": "standard",
        "tags": [],
        "addOptions": {
            "monitor": "none",
            "searchForMissingEpisodes": false,
            "searchForCutoffUnmetEpisodes": false
        }
    })
}

fn sonarr_seasonpass_body() -> Value {
    json!({
        "monitoringOptions": {
            "monitor": "none",
            "episodesToMonitor": [],
            "monitorNewItems": "none"
        },
        "series": [sonarr_series_body()]
    })
}

fn sonarr_backup_upload_fixture_path() -> String {
    "target/live-full/tmp/sonarr-backup-upload.zip".into()
}

fn radarr_autotagging_body(name: String) -> Value {
    json!({
        "name": name,
        "removeTagsAutomatically": false,
        "tags": [1],
        "specifications": [{
            "name": "Monitored",
            "implementation": "MonitoredSpecification",
            "implementationName": "Monitored",
            "negate": false,
            "required": false,
            "fields": []
        }]
    })
}

fn radarr_download_client_body(name: String) -> Value {
    json!({
        "enable": false,
        "protocol": "torrent",
        "priority": 1,
        "removeCompletedDownloads": false,
        "removeFailedDownloads": false,
        "name": name,
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
            {"name": "movieCategory", "value": "radarr"},
            {"name": "movieImportedCategory", "value": ""},
            {"name": "recentMoviePriority", "value": 0},
            {"name": "olderMoviePriority", "value": 0},
            {"name": "initialState", "value": 0},
            {"name": "sequentialOrder", "value": false},
            {"name": "firstAndLast", "value": false},
            {"name": "contentLayout", "value": 0}
        ],
        "tags": []
    })
}

fn radarr_indexer_body(name: String) -> Value {
    json!({
        "enableRss": false,
        "enableAutomaticSearch": false,
        "enableInteractiveSearch": false,
        "supportsRss": true,
        "supportsSearch": true,
        "protocol": "usenet",
        "priority": 1,
        "name": name,
        "implementation": "Newznab",
        "implementationName": "Newznab",
        "configContract": "NewznabSettings",
        "fields": [
            {"name": "baseUrl", "value": "http://127.0.0.1:9"},
            {"name": "apiPath", "value": "/api"},
            {"name": "apiKey", "value": "rustarr-live"},
            {"name": "categories", "value": [2000, 2010]},
            {"name": "animeCategories", "value": []},
            {"name": "additionalParameters", "value": ""},
            {"name": "multiLanguages", "value": []}
        ],
        "tags": []
    })
}

fn radarr_notification_body(name: String) -> Value {
    json!({
        "name": name,
        "implementation": "CustomScript",
        "implementationName": "Custom Script",
        "configContract": "CustomScriptSettings",
        "fields": [
            {"name": "path", "value": "/usr/bin/true"},
            {"name": "arguments", "value": ""}
        ],
        "onGrab": false,
        "onDownload": false,
        "onUpgrade": false,
        "onRename": false,
        "onMovieAdded": false,
        "onMovieDelete": false,
        "onMovieFileDelete": false,
        "onMovieFileDeleteForUpgrade": false,
        "onHealthIssue": false,
        "onHealthRestored": false,
        "onApplicationUpdate": false,
        "onManualInteractionRequired": false,
        "includeHealthWarnings": false,
        "tags": []
    })
}

fn unique_live_label(kind: ServiceKind, op_name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let op_name = op_name.replace('_', "-");
    format!("rustarr-live-{}-{op_name}-{nanos}", kind.as_str())
}

fn apply_fixture_args(
    kind: ServiceKind,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    args: &mut Map<String, Value>,
) {
    if kind == ServiceKind::Prowlarr
        && matches!(op.name, "get_download_by_id" | "get_indexer_download_by_id")
        && let Some(release) = fixtures.body_for("/api/v1/search")
        && let Some(download_url) = release.get("downloadUrl").and_then(Value::as_str)
    {
        for key in ["link", "file"] {
            if let Some(value) = query_value_from_url(download_url, key) {
                args.insert(key.into(), Value::String(value));
            }
        }
    }
    if kind == ServiceKind::Prowlarr && matches!(op.name, "get_by_id" | "get_indexer_newznab_by_id")
    {
        args.insert("t".into(), json!("search"));
    }
    if kind == ServiceKind::Prowlarr && op.name == "post_system_backup_restore_upload" {
        args.insert(
            "filePath".into(),
            json!(prowlarr_backup_upload_fixture_path()),
        );
        args.insert(
            "fileName".into(),
            json!(unique_live_label(kind, op.name) + ".zip"),
        );
    }
    if kind == ServiceKind::Sonarr && op.name == "post_system_backup_restore_upload" {
        args.insert(
            "filePath".into(),
            json!(sonarr_backup_upload_fixture_path()),
        );
        args.insert(
            "fileName".into(),
            json!(unique_live_label(kind, op.name) + ".zip"),
        );
    }
    for param in op.query_params {
        if let Some(value) = fixture_arg_value(kind, op, fixtures, param) {
            if args.contains_key(*param) || should_seed_optional_query(param) {
                args.insert((*param).to_string(), value);
            }
        }
    }
    if kind == ServiceKind::Radarr {
        match op.name {
            "get_manualimport" => {
                args.entry("path").or_insert(json!(live_root_path(kind)));
            }
            "get_moviefile" | "get_rename" => {
                if let Some(movie_id) =
                    fixture_first_id(fixtures, &["/api/v3/movie", "/api/v3/movie/import"])
                {
                    args.entry("movieId").or_insert(movie_id);
                }
            }
            _ => {}
        }
    }
}

fn apply_fixture_body_args(
    kind: ServiceKind,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    args: &mut Map<String, Value>,
) {
    if !matches!(kind, ServiceKind::Prowlarr | ServiceKind::Sonarr) {
        return;
    }
    let Some(body) = args.get_mut("body").and_then(Value::as_object_mut) else {
        if kind == ServiceKind::Prowlarr
            && op.name == "post_search_bulk"
            && let Some(release) = fixtures.body_for("/api/v1/search")
        {
            args.insert("body".into(), Value::Array(vec![release.clone()]));
        }
        return;
    };
    if kind == ServiceKind::Sonarr
        && matches!(op.name, "post_release" | "post_release_push")
        && let Some(release) = fixtures
            .body_for("/api/v3/release")
            .and_then(Value::as_object)
    {
        body.clear();
        body.extend(release.clone());
        return;
    }
    if kind == ServiceKind::Sonarr {
        if body.contains_key("qualityProfileId")
            && let Some(id) = fixture_first_id(fixtures, &["/api/v3/qualityprofile"])
        {
            body.insert("qualityProfileId".into(), id);
        }
        if body.contains_key("languageProfileId")
            && let Some(id) = fixture_first_id(fixtures, &["/api/v3/languageprofile"])
        {
            body.insert("languageProfileId".into(), id);
        }
    }
    if kind == ServiceKind::Prowlarr
        && op.name == "post_search"
        && let Some(release) = fixtures
            .body_for("/api/v1/search")
            .and_then(Value::as_object)
    {
        body.clear();
        body.extend(release.clone());
        return;
    }
    if body.get("ids").is_some_and(|value| {
        value.as_array().is_some_and(|items| items.is_empty()) || value.is_null()
    }) || (matches!(kind, ServiceKind::Prowlarr | ServiceKind::Sonarr)
        && op.path.ends_with("/bulk")
        && !body.contains_key("ids"))
    {
        if let Some(Value::Array(ids)) = fixture_arg_value(kind, op, fixtures, "ids") {
            body.insert("ids".into(), Value::Array(ids));
        }
    }
}

fn should_seed_optional_query(param: &str) -> bool {
    let lower = param.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "seriesid"
            | "movieid"
            | "episodeids"
            | "episodefileids"
            | "itemid"
            | "userid"
            | "parentid"
            | "sectionid"
            | "librarysectionid"
            | "ratingkey"
            | "metadataitemid"
            | "path"
            | "term"
            | "query"
            | "imdbid"
            | "tmdbid"
    )
}

fn fixture_arg_value(
    kind: ServiceKind,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    param: &str,
) -> Option<Value> {
    let lower = param.to_ascii_lowercase();
    if lower == "path" {
        return Some(json!(live_root_path(kind)));
    }
    if lower == "term" || lower == "query" || lower == "searchterm" {
        return Some(json!(live_search_term(kind)));
    }
    if kind == ServiceKind::Prowlarr && lower == "t" {
        return Some(json!("search"));
    }
    if lower == "prefs" {
        return Some(json!(["FriendlyName=Rustarr Live Plex"]));
    }
    if lower == "imagetype" {
        return Some(json!("Primary"));
    }
    if lower == "imageindex" || lower == "index" || lower == "routeindex" {
        return Some(json!(0));
    }
    if lower == "container" || lower == "format" || lower == "routeformat" {
        return Some(json!("mp4"));
    }
    if lower == "language" {
        return Some(json!("eng"));
    }
    if lower == "width" || lower == "maxwidth" {
        return Some(json!(320));
    }
    if lower == "height" || lower == "maxheight" {
        return Some(json!(180));
    }
    if lower == "year" {
        return Some(json!(2026));
    }
    if kind == ServiceKind::Radarr && lower == "tmdbid" {
        return Some(json!(603));
    }
    if kind == ServiceKind::Radarr && lower == "imdbid" {
        return Some(json!("tt0133093"));
    }
    if lower == "percentplayed" || lower == "unplayedcount" || lower.ends_with("ticks") {
        return Some(json!(0));
    }
    if lower == "seriesid" {
        return fixture_first_id(fixtures, &["/api/v3/series"]);
    }
    if lower == "movieid" {
        return fixture_first_id(fixtures, &["/api/v3/movie"]);
    }
    if lower == "episodeids" {
        return fixture_first_id(fixtures, &["/api/v3/episode"]).map(|id| json!([id]));
    }
    if lower == "episodefileids" {
        return fixture_first_id(fixtures, &["/api/v3/episodefile"]).map(|id| json!([id]));
    }
    if lower == "userid" {
        return fixture_first_id(fixtures, &["/Users", "/users"]);
    }
    if lower == "itemid"
        || lower == "videoid"
        || lower == "routeitemid"
        || lower == "parentid"
        || lower == "artistid"
        || lower == "albumid"
    {
        return fixture_first_id(fixtures, &["/Items", "/library/metadata"]);
    }
    if lower == "mediasourceid" || lower == "routemediasourceid" {
        return fixture_first_media_source_id(fixtures)
            .or_else(|| fixture_first_id(fixtures, &["/Items", "/library/metadata"]));
    }
    if lower == "sectionid" || lower == "librarysectionid" {
        return fixture_first_id(fixtures, &["/library/sections/all"]);
    }
    if lower == "ratingkey" || lower == "metadataitemid" {
        return fixture_first_id(fixtures, &["/library/metadata", "/library/sections/all"]);
    }
    if lower == "id" || lower.ends_with("id") || lower == "ids" {
        let parent = fixture_parent_path(op.path);
        let id = fixture_path_value(fixtures, parent, param)
            .or_else(|| fixture_first_id(fixtures, &[parent]));
        return if lower == "ids" {
            id.map(|value| json!([value]))
        } else {
            id
        };
    }
    if lower.contains("name") {
        let parent = fixture_parent_path(op.path);
        return fixture_path_value(fixtures, parent, param).or_else(|| Some(json!("rustarr-live")));
    }
    None
}

fn fixture_path_param_value(
    kind: ServiceKind,
    fixtures: &FixtureStore,
    parent: &str,
    param: &str,
) -> Option<Value> {
    if param.eq_ignore_ascii_case("ids") {
        return Some(json!(1));
    }
    if kind == ServiceKind::Prowlarr {
        let lower = param.to_ascii_lowercase();
        match (parent, lower.as_str()) {
            ("", "id") => return fixture_first_id(fixtures, &["/api/v1/indexer"]),
            ("", "path") => return Some(json!("api/v1/system/status")),
            ("/content", "path") => return Some(json!("styles.css")),
            ("/api/v1/log/file", "filename") | ("/api/v1/log/file/update", "filename") => {
                return Some(json!("prowlarr.txt"));
            }
            ("/api/v1/appprofile", "id") => {
                return fixture_first_non_id(fixtures, "/api/v1/appprofile", 1);
            }
            ("/api/v1/indexerproxy", "name") => return Some(json!("rustarr-live-flaresolverr")),
            ("/api/v1/system/backup", "id") => {
                return fixture_first_id(fixtures, &["/api/v1/system/backup"]);
            }
            ("/api/v1/system/backup/restore", "id") => {
                return fixture_nth_id(fixtures, "/api/v1/system/backup", 1)
                    .or_else(|| fixture_first_id(fixtures, &["/api/v1/system/backup"]));
            }
            _ => {}
        }
    }
    if kind == ServiceKind::Sonarr {
        let lower = param.to_ascii_lowercase();
        match (parent, lower.as_str()) {
            ("", "path") => return Some(json!("api/v3/system/status")),
            ("/content", "path") => return Some(json!("styles.css")),
            ("/api/v3/log/file", "filename") | ("/api/v3/log/file/update", "filename") => {
                return Some(json!("sonarr.txt"));
            }
            ("/api/v3/calendar", "id") => return fixture_first_id(fixtures, &["/api/v3/episode"]),
            ("/api/v3/wanted/cutoff", "id") => {
                return fixture_first_id(fixtures, &["/api/v3/episode"]);
            }
            ("/api/v3/mediacover", "seriesid") => {
                return fixture_first_id(fixtures, &["/api/v3/series"]);
            }
            ("/api/v3/mediacover", "filename") => return Some(json!("poster.jpg")),
            ("/api/v3/blocklist", "id")
            | ("/api/v3/episodefile", "id")
            | ("/api/v3/history", "id")
            | ("/api/v3/localization", "id")
            | ("/api/v3/queue", "id") => return Some(json!(1)),
            ("/api/v3/system/backup/restore", "id") => {
                return fixture_nth_id(fixtures, "/api/v3/system/backup", 1)
                    .or_else(|| fixture_first_id(fixtures, &["/api/v3/system/backup"]));
            }
            _ => {}
        }
    }
    if kind == ServiceKind::Radarr {
        let lower = param.to_ascii_lowercase();
        match (parent, lower.as_str()) {
            ("", "path") => return Some(json!("api/v3/system/status")),
            ("/content", "path") => return Some(json!("styles.css")),
            ("/api/v3/log/file", "filename") | ("/api/v3/log/file/update", "filename") => {
                return Some(json!("radarr.txt"));
            }
            ("/api/v3/mediacover", "movieid") => {
                return fixture_first_id(fixtures, &["/api/v3/movie"]);
            }
            ("/api/v3/mediacover", "filename") => return Some(json!("poster.jpg")),
            ("/api/v3/blocklist", "id")
            | ("/api/v3/collection", "id")
            | ("/api/v3/customformat", "id")
            | ("/api/v3/history", "id")
            | ("/api/v3/importlist", "id")
            | ("/api/v3/moviefile", "id")
            | ("/api/v3/queue", "id")
            | ("/api/v3/releaseprofile", "id") => return Some(json!(1)),
            ("/api/v3/system/backup/restore", "id") => {
                return fixture_nth_id(fixtures, "/api/v3/system/backup", 1)
                    .or_else(|| fixture_first_id(fixtures, &["/api/v3/system/backup"]));
            }
            ("/api/v3/importlist", "name") => return Some(json!("test")),
            _ => {}
        }
    }
    if kind == ServiceKind::Overseerr && param.eq_ignore_ascii_case("status") {
        if parent == "/api/v1/request" {
            return Some(json!("approve"));
        }
        if parent == "/api/v1/media" {
            return Some(json!("available"));
        }
    }
    if kind == ServiceKind::Overseerr {
        let lower = param.to_ascii_lowercase();
        match lower.as_str() {
            "status" if parent == "/api/v1/issue" => return Some(json!("resolved")),
            "movieid" | "tmdbid" => return Some(json!(603)),
            "tvid" => return Some(json!(1399)),
            "seasonid" => return Some(json!(1)),
            "personid" => return Some(json!(6384)),
            "collectionid" => return Some(json!(2344)),
            "keywordid" => return Some(json!(9715)),
            "genreid" => return Some(json!(28)),
            "studioid" => return Some(json!(2)),
            "networkid" => return Some(json!(49)),
            "language" => return Some(json!("en")),
            "userid" => return fixture_first_id(fixtures, &["/api/v1/user"]).or(Some(json!(1))),
            "requestid" => {
                return fixture_first_id(fixtures, &["/api/v1/request"]).or(Some(json!(1)));
            }
            "mediaid" => return fixture_first_id(fixtures, &["/api/v1/media"]).or(Some(json!(1))),
            "issueid" => return fixture_first_id(fixtures, &["/api/v1/issue"]).or(Some(json!(1))),
            "commentid" => {
                return fixture_first_id(fixtures, &["/api/v1/issueComment"]).or(Some(json!(1)));
            }
            "endpoint" => return Some(json!("rustarr-live")),
            "sliderid" | "radarrid" | "sonarrid" | "cacheid" | "jobid" => return Some(json!(1)),
            "guid" => return Some(json!("00000000-0000-0000-0000-000000000000")),
            _ => {}
        }
    }
    if kind == ServiceKind::Jellyfin {
        let lower = param.to_ascii_lowercase();
        if lower == "format" && parent.contains("/Images") {
            return Some(json!("jpg"));
        }
        match lower.as_str() {
            "imagetype" => return Some(json!("Primary")),
            "imageindex" | "index" | "routeindex" | "newindex" => return Some(json!(0)),
            "container" | "format" | "routeformat" => return Some(json!("mp4")),
            "language" => return Some(json!("eng")),
            "year" => return Some(json!(2026)),
            "maxwidth" | "width" => return Some(json!(320)),
            "maxheight" | "height" => return Some(json!(180)),
            "percentplayed" | "unplayedcount" | "routestartpositionticks" => {
                return Some(json!(0));
            }
            "tag" => return Some(json!("0")),
            "command" => return Some(json!("DisplayContent")),
            "version" => {
                return fixture_field_value_case_insensitive(fixtures, parent, "version");
            }
            _ => {}
        }
    }
    fixture_path_value(fixtures, parent, param).or_else(|| default_path_param_value(param))
}

fn default_path_param_value(param: &str) -> Option<Value> {
    let lower = param.to_ascii_lowercase();
    if lower == "transcodetype" {
        return Some(json!("video"));
    }
    if lower == "ext" || lower == "extension" {
        return Some(json!("mp4"));
    }
    if lower == "butlertask" {
        return Some(json!("CleanOldBundles"));
    }
    if lower == "country" {
        return Some(json!("USA"));
    }
    if lower == "epgid" {
        return Some(json!("1"));
    }
    if lower == "region" {
        return Some(json!("CA"));
    }
    if lower == "channel" || lower == "chapter" || lower == "offset" || lower == "version" {
        return Some(json!(0));
    }
    if lower == "updatedat" || lower == "changestamp" {
        return Some(json!(1));
    }
    if lower == "action" {
        return Some(json!("add"));
    }
    if lower.contains("id") || lower.ends_with("key") || lower == "ids" {
        return Some(json!(1));
    }
    if lower.contains("guid") {
        return Some(json!("00000000-0000-0000-0000-000000000000"));
    }
    if lower.contains("name") || lower.contains("key") || lower.contains("token") {
        return Some(json!("rustarr-live"));
    }
    if lower.contains("path") || lower.contains("file") {
        return Some(json!("rustarr-live"));
    }
    None
}

fn fixture_first_id(fixtures: &FixtureStore, paths: &[&str]) -> Option<Value> {
    paths.iter().find_map(|path| {
        fixtures
            .values_for(path)
            .and_then(|values| values.first().cloned())
    })
}

fn fixture_first_non_id(fixtures: &FixtureStore, path: &str, excluded: i64) -> Option<Value> {
    fixtures.values_for(path).and_then(|values| {
        values
            .iter()
            .find(|value| value.as_i64() != Some(excluded))
            .cloned()
    })
}

fn fixture_nth_id(fixtures: &FixtureStore, path: &str, index: usize) -> Option<Value> {
    fixtures
        .values_for(path)
        .and_then(|values| values.get(index).cloned())
}

fn query_value_from_url(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (candidate, value) = pair.split_once('=')?;
        (candidate == key).then(|| value.replace('+', " "))
    })
}

fn fixture_first_media_source_id(fixtures: &FixtureStore) -> Option<Value> {
    fixtures.bodies.values().flatten().find_map(|body| {
        body.pointer("/MediaSources/0/Id")
            .or_else(|| body.pointer("/media/0/id"))
            .filter(|value| is_scalar(value))
            .cloned()
    })
}

fn live_root_path(kind: ServiceKind) -> &'static str {
    match kind {
        ServiceKind::Sonarr => "/data/media/tv",
        ServiceKind::Radarr => "/data/media/movies",
        ServiceKind::Jellyfin | ServiceKind::Plex => "/data/rustarr-live-plex-movies",
        _ => "/tmp",
    }
}

fn live_search_term(kind: ServiceKind) -> &'static str {
    match kind {
        ServiceKind::Sonarr => "silo",
        ServiceKind::Radarr => "the matrix",
        ServiceKind::Prowlarr => "ubuntu",
        ServiceKind::Jellyfin | ServiceKind::Plex => "rustarr",
        _ => "rustarr",
    }
}

fn fixture_parent_path(path: &str) -> &str {
    let parent = path.split_once("/{").map(|(a, _)| a).unwrap_or(path);
    for suffix in [
        "/action", "/test", "/testall", "/failed", "/grab", "/reorder", "/refresh", "/bulk",
    ] {
        if let Some(stripped) = parent.strip_suffix(suffix) {
            return stripped;
        }
    }
    parent
}

fn fixture_path_value(fixtures: &FixtureStore, parent: &str, param: &str) -> Option<Value> {
    let lower = param.to_ascii_lowercase();
    if lower == "index" || lower == "newindex" {
        return Some(json!(0));
    }
    let body = fixtures.body_for(parent);
    if lower.contains("name")
        && let Some(value) = body.and_then(|b| field_value(b, &["name", "Name", "title", "Title"]))
    {
        return Some(value);
    }
    body.and_then(|b| {
        field_value(
            b,
            &[param, "id", "Id", "ID", "ratingKey", "key", "Guid", "guid"],
        )
    })
    .or_else(|| {
        fixtures
            .values_for(parent)
            .and_then(|values| values.first().cloned())
    })
    .or_else(|| {
        fixture_parent_aliases(parent).iter().find_map(|alias| {
            fixtures
                .body_for(alias)
                .and_then(|b| {
                    field_value(
                        b,
                        &[param, "id", "Id", "ID", "ratingKey", "key", "Guid", "guid"],
                    )
                })
                .or_else(|| {
                    fixtures
                        .values_for(alias)
                        .and_then(|values| values.first().cloned())
                })
        })
    })
}

fn fixture_field_value_case_insensitive(
    fixtures: &FixtureStore,
    parent: &str,
    field: &str,
) -> Option<Value> {
    fixtures
        .body_for(parent)
        .and_then(|body| object_field_case_insensitive(body, field))
        .or_else(|| {
            fixture_parent_aliases(parent).iter().find_map(|alias| {
                fixtures
                    .body_for(alias)
                    .and_then(|body| object_field_case_insensitive(body, field))
            })
        })
}

fn object_field_case_insensitive(value: &Value, field: &str) -> Option<Value> {
    let obj = value.as_object()?;
    obj.iter()
        .find(|(key, value)| key.eq_ignore_ascii_case(field) && is_scalar(value))
        .map(|(_, value)| value.clone())
}

fn field_value(value: &Value, keys: &[&str]) -> Option<Value> {
    let obj = value.as_object()?;
    keys.iter()
        .find_map(|key| obj.get(*key).filter(|v| is_scalar(v)).cloned())
}

fn fixture_body_for_op<'a>(fixtures: &'a FixtureStore, op: &OperationSpec) -> Option<&'a Value> {
    let parent = fixture_parent_path(op.path);
    fixtures
        .body_for(op.path)
        .or_else(|| fixtures.body_for(parent))
        .or_else(|| {
            fixture_parent_aliases(parent)
                .iter()
                .find_map(|alias| fixtures.body_for(alias))
        })
        .or_else(|| {
            let leaf = parent.rsplit('/').next().unwrap_or(parent);
            fixtures
                .bodies
                .iter()
                .find(|(path, _)| path.ends_with(&format!("/{leaf}")))
                .and_then(|(_, bodies)| bodies.first())
        })
}

fn fixture_parent_aliases(parent: &str) -> &'static [&'static str] {
    match parent {
        // Jellyfin exposes media operations through type-specific routes, but the
        // broad `/Items` collection is the reliable source of item ids in a small
        // fixture library.
        "/Videos" | "/Audio" | "/UserItems" | "/Shows" => &["/Items"],
        "/Artists" | "/Persons" | "/Studios" | "/MusicGenres" => &["/Items"],
        // Plex section and metadata ids are frequently nested under collection
        // endpoints rather than exposed by the exact templated parent.
        "/library/sections" => &["/library/sections/all"],
        "/library/metadata" => &["/library/all"],
        "/hubs/sections" => &["/library/sections/all"],
        "/hubs/metadata" => &["/library/metadata"],
        _ => &[],
    }
}

pub(super) fn op_requires_stack_reset(op: &OperationSpec) -> bool {
    let lp = op.path.to_ascii_lowercase();
    op_is_self_destructive_control(op)
        || lp.contains("/backup/restore")
        || (!op.method.is_read()
            && (lp.contains("/settings")
                || lp.contains("/auth")
                || lp.contains("/config")
                || lp.contains("/configuration")
                || lp.contains("/startup")
                || lp.contains("/prefs")
                || lp.contains("apikey")))
}

fn op_is_self_destructive_control(op: &OperationSpec) -> bool {
    let lp = op.path.to_ascii_lowercase();
    lp.contains("shutdown") || lp.contains("restart")
}

/// Invoke `rustarr <svc> op <name> --args <json> [--confirm]`. Returns the parsed
/// JSON result on a 2xx, `None` for an empty body, or an error with the upstream
/// message on a non-2xx / CLI error.
const CONTRACT_INVOKE_ATTEMPTS: usize = 3;

fn invoke(
    rustarr: &process::RustarrProcess,
    svc: &str,
    name: &str,
    args: &Map<String, Value>,
    confirm: bool,
) -> Result<Option<Value>> {
    let mut last_error = None;
    for attempt in 1..=CONTRACT_INVOKE_ATTEMPTS {
        match invoke_once(rustarr, svc, name, args, confirm) {
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

pub(super) fn is_retryable_contract_error(detail: &str) -> bool {
    detail.contains("request failed")
        || detail.contains("tcp connect error")
        || detail.contains("connection closed")
        || detail.contains("error sending request")
}

fn invoke_once(
    rustarr: &process::RustarrProcess,
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
    let output = rustarr.output(&argv)?;
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
    // `RustarrClient` returns `{"ok":true,"status":<code>}` for an empty 2xx body
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

fn write_detail(svc: &str, results: &[OpResult]) -> Result<()> {
    let dir = std::path::Path::new("target/live-full");
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("contract-{svc}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(results)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
#[path = "contract_tests.rs"]
mod tests;
