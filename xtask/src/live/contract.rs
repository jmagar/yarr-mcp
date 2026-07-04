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

use rustarr::ServiceKind;
use rustarr::openapi::{self, HttpMethod, OperationSpec};

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
    pub(super) outcome: &'static str, // ok | schema_mismatch | rejected | skipped
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
        _ => Ok(()),
    }
}

fn ensure_sonarr_download_client(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    ensure_sonarr_qbittorrent_download_client(rustarr, svc)?;
    ensure_sonarr_newznab_indexer(rustarr, svc)?;
    ensure_sonarr_custom_script_notification(rustarr, svc)?;
    ensure_sonarr_remote_path_mapping(rustarr, svc)?;
    ensure_sonarr_autotagging(rustarr, svc)
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
        "implementation": "Newznab",
        "implementationName": "Newznab",
        "configContract": "NewznabSettings",
        "fields": [
            {"name": "baseUrl", "value": "http://127.0.0.1:9"},
            {"name": "apiPath", "value": "/api"},
            {"name": "apiKey", "value": "rustarr-live"},
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
        let bodies = harvest_objects(value);
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
        PreparedOp::Skip(detail) => return (mk("skipped", detail), None),
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
    // NEVER call self-destructive control endpoints: shutdown/restart stop the
    // service mid-run (which is exactly what took prowlarr down), and backup/restore
    // rewrites its whole config. Testing "every endpoint" cannot mean bricking the
    // stack — these are skipped by design.
    if op_requires_stack_reset(op) && !allow_reset_required {
        return PreparedOp::Skip(
            "requires stack reset/reseed (control endpoint or config/auth mutation)".into(),
        );
    }
    // Satisfy path params from discovered/seeded fixtures (parent collection =
    // path before `{`). No fallback IDs: a contract call that needs a resource but
    // has no resource fixture is not a meaningful live test.
    let mut path_args = Map::new();
    if !op.path_params.is_empty() {
        let parent = fixture_parent_path(op.path);
        for p in op.path_params {
            let Some(value) = fixture_path_value(fixtures, parent, p) else {
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
    apply_fixture_args(kind, op, fixtures, &mut args);
    if op.has_body
        && let Some(body) = live_fixture_body_for_op(kind, op)
    {
        args.insert("body".into(), body);
        return PreparedOp::Call(args);
    }
    if op.has_body && can_reuse_fixture_body(op) {
        if let Some(body) = fixture_body_for_op(fixtures, op) {
            args.insert("body".into(), body.clone());
        }
    }
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
        (ServiceKind::Sonarr | ServiceKind::Radarr | ServiceKind::Prowlarr, "post_tag") => {
            Some(json!({ "label": unique_live_label(kind, op.name) }))
        }
        _ => None,
    }
}

fn unique_live_label(kind: ServiceKind, op_name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("rustarr-live-{}-{op_name}-{nanos}", kind.as_str())
}

fn apply_fixture_args(
    kind: ServiceKind,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    args: &mut Map<String, Value>,
) {
    for param in op.query_params {
        if let Some(value) = fixture_arg_value(kind, op, fixtures, param) {
            if args.contains_key(*param) || should_seed_optional_query(param) {
                args.insert((*param).to_string(), value);
            }
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

fn fixture_first_id(fixtures: &FixtureStore, paths: &[&str]) -> Option<Value> {
    paths.iter().find_map(|path| {
        fixtures
            .values_for(path)
            .and_then(|values| values.first().cloned())
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
        "/action", "/test", "/testall", "/failed", "/grab", "/reorder", "/refresh",
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
        "/hubs/sections" => &["/library/sections/all"],
        "/hubs/metadata" => &["/library/metadata"],
        _ => &[],
    }
}

pub(super) fn op_requires_stack_reset(op: &OperationSpec) -> bool {
    let lp = op.path.to_ascii_lowercase();
    lp.contains("shutdown")
        || lp.contains("restart")
        || lp.contains("/backup/restore")
        || lp.ends_with("/system/backup")
        || (!op.method.is_read()
            && (lp.contains("/settings")
                || lp.contains("/auth")
                || lp.contains("/config")
                || lp.contains("/configuration")
                || lp.contains("/startup")
                || lp.contains("/prefs")
                || lp.contains("apikey")))
}

#[cfg(test)]
fn is_known_non_contract_endpoint(path: &str) -> bool {
    let lp = path.to_ascii_lowercase();
    lp == "/login" || lp == "/logout" || lp.ends_with(".ics") || lp.ends_with("/system/routes")
}

#[cfg(test)]
fn is_unseeded_optional_feature_endpoint(kind: ServiceKind, path: &str) -> bool {
    let lp = path.to_ascii_lowercase();
    match kind {
        ServiceKind::Jellyfin => {
            lp.starts_with("/livetv/")
                || lp.starts_with("/syncplay/")
                || lp.starts_with("/items/remotesearch/")
                || lp.starts_with("/quickconnect/")
        }
        ServiceKind::Plex => {
            lp.starts_with("/livetv/")
                || lp.starts_with("/media/subscriptions")
                || lp.starts_with("/media/grabbers")
                || lp.starts_with("/media/providers")
                || lp.starts_with("/downloadqueue")
        }
        _ => false,
    }
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
