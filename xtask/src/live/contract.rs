//! Exhaustive contract harness for the generated OpenAPI surface.
//!
//! shart is a disposable, dedicated test stack, so this drives **every generated
//! operation** of every spec-backed service (all methods — reads, writes, and
//! destructive deletes) via the `yarr <service> op <name>` CLI verb, with inputs
//! synthesized from the vendored spec, and validates each 2xx response against the
//! operation's declared response schema. Output is a per-service summary plus a
//! per-operation breakdown written to `target/live-full/contract-<svc>.json`.
//!
//! Operations run GET -> POST -> PUT/PATCH -> DELETE so reads/updates see existing
//! resources before deletes remove them. Pass `--no-destructive` to skip DELETEs.

pub(super) mod invoke;
pub(super) mod reset_ops;
pub(super) mod seeding;
pub(super) mod synth;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

use yarr::ServiceKind;
use yarr::openapi::{self, HttpMethod, OperationSpec};

use super::{process, report, reset};
pub(super) use invoke::is_retryable_contract_error;
use invoke::{invoke, write_detail};
pub(super) use reset_ops::cleanup_service_fixtures;
use reset_ops::run_reset_required_ops;
pub(super) use seeding::seed_service_fixtures;
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
    yarr: &process::YarrProcess,
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
            if let Some(url) = reset::service_url(&yarr.env, svc) {
                reset::wait_service_url(&url)?;
            }
        }
        seed_service_fixtures(yarr, svc, kind)
            .with_context(|| format!("seed live fixtures for {svc}"))?;
        let spec = Spec::load(spec_path).with_context(|| format!("load {spec_path}"))?;
        let ops: Vec<&'static OperationSpec> = openapi::operations_for_kind(kind).iter().collect();
        let mut fixtures = FixtureStore {
            provider: seeding::prime_provider_fixtures(yarr, svc, kind),
            ..Default::default()
        };

        // Create-first seeding: run phases in order, harvesting ids between them so
        // later phases can hit real resources:
        //   0  base collection reads (GET, no path/query fixture dependency)
        //   1  query collection reads (GETs needing seeded query ids)
        //   2  creates (POST)                          -> seed ids from created objects
        //   3  resource reads/updates (GET/PUT/PATCH)  -> use seeded ids
        //   4  deletes (DELETE)                        -> use seeded ids; also cleanup
        let mut results: Vec<OpResult> = Vec::with_capacity(ops.len());
        for phase in 0u8..=4 {
            let phase_ops: Vec<&'static OperationSpec> = ops
                .iter()
                .copied()
                .filter(|o| seed_phase(o) == phase)
                .filter(|o| !op_requires_stack_reset(o))
                .collect();
            let outs = parallel_run(
                yarr,
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
        let reset_outs =
            run_reset_required_ops(yarr, svc, kind, &spec, &fixtures, &ops, no_destructive);
        results.extend(reset_outs.into_iter().map(|(_, r, _)| r));

        write_detail(svc, &results)?;
        let status = contract_status(&results);
        if status.passed {
            report.pass(format!("contract {svc}"), status.detail);
        } else {
            report.fail(format!("contract {svc}"), status.detail);
        }
        if let Err(err) = cleanup_service_fixtures(kind) {
            eprintln!("warning: failed to clean live fixtures for {svc}: {err:#}");
        }
    }
    Ok(())
}

/// Bounded thread pool: run `ops` through `run_op` concurrently. Returns, per op,
/// `(op, result, success-body)` so the caller can harvest seeded ids between phases.
pub(super) type RunOut = (&'static OperationSpec, OpResult, Option<Value>);

fn parallel_run(
    yarr: &process::YarrProcess,
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
                                run_op(yarr, svc, kind, spec, op, fixtures, no_destructive);
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
    provider: seeding::ProviderFixtures,
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
        // A single-resource response almost always carries its own scalar id —
        // treat that as a resource, not a paginated-envelope wrapper, even if it
        // also happens to have an unrelated array-valued field (`tags`, `fields`,
        // `specifications`, ...). Nearly every Servarr resource has one of those,
        // so unconditionally unwrapping the first array field (the old behavior)
        // silently dropped real single-resource create/update responses in favor
        // of whatever unrelated nested array happened to sort first.
        Value::Object(map) if first_id_value(value).is_none() => {
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
        Value::Object(_) => vec![value.clone()],
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
    yarr: &process::YarrProcess,
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
    match invoke(yarr, svc, op.name, &args) {
        Ok(Some(value)) => {
            let result = match op.response_type {
                Some(ty) => match spec.validate_response(ty, &value) {
                    Ok(()) => mk("ok", format!("2xx + matches {ty}")),
                    Err(e) => mk(
                        "schema_mismatch",
                        format!("{e}").chars().take(500).collect(),
                    ),
                },
                None => mk("ok", "2xx (no declared response type to validate)".into()),
            };
            (result, Some(value))
        }
        Ok(None) => (mk("ok", "2xx (empty/non-JSON body)".into()), None),
        Err(e) => (
            mk("rejected", format!("{e}").chars().take(500).collect()),
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
        && let Some(body) = live_fixture_body_for_op(kind, op, fixtures)
    {
        args.insert("body".into(), body);
        return PreparedOp::Call(args);
    }
    if op.has_body
        && can_reuse_fixture_body(op)
        && let Some(body) = fixture_body_for_op(fixtures, op)
    {
        args.insert("body".into(), body.clone());
    }
    PreparedOp::Call(args)
}

fn can_reuse_fixture_body(op: &OperationSpec) -> bool {
    matches!(op.method, HttpMethod::Put | HttpMethod::Patch)
        || op.path.ends_with("/test")
        || op.path.contains("/test/")
        || op.path.contains("/action/")
}

fn live_fixture_body_for_op(
    kind: ServiceKind,
    op: &OperationSpec,
    fixtures: &FixtureStore,
) -> Option<Value> {
    match (kind, op.name) {
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_command") => {
            Some(json!({ "name": "RefreshMonitoredDownloads" }))
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr | ServiceKind::Prowlarr, "post_tag") => {
            Some(json!({ "label": unique_live_label(kind, op.name) }))
        }
        // Provider-backed resources (downloadclient/indexer/notification/metadata)
        // validate `implementation`/`configContract`/`fields` server-side in a way
        // the generic OpenAPI-schema synthesizer can't discover. Reuse the live
        // `GET .../schema` template primed into `fixtures.provider` — it's already
        // a valid, ready-to-POST body — with a fresh unique `name`.
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_downloadclient") => {
            named_provider_template(&fixtures.provider.downloadclient, kind, op.name)
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_indexer") => {
            named_provider_template(&fixtures.provider.indexer, kind, op.name)
        }
        // The `/test` sibling of a create is also a POST, so it runs in the SAME
        // seed phase (see `seed_phase`) — cross-op fixture harvesting can't see a
        // same-phase create's result yet, so `/test` needs its own copy of the
        // same valid template rather than relying on `can_reuse_fixture_body`'s
        // reuse-from-fixtures fallback (which would otherwise pick up whatever a
        // PRIOR phase happened to harvest, e.g. a seeded fixture with an invalid
        // path for this specific check).
        (
            ServiceKind::Sonarr | ServiceKind::Radarr,
            "post_notification" | "post_notification_test",
        ) => named_provider_template(&fixtures.provider.notification, kind, op.name),
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_metadata") => {
            named_provider_template(&fixtures.provider.metadata, kind, op.name)
        }
        // ImportListResource also requires a real `rootFolderPath` and
        // `qualityProfileId` that the schema template leaves as placeholders.
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_importlist" | "post_importlist_test") => {
            let mut body = named_provider_template(&fixtures.provider.importlist, kind, op.name)?;
            let obj = body.as_object_mut()?;
            if let Some(path) = &fixtures.provider.root_folder_path {
                obj.insert("rootFolderPath".into(), json!(path));
            }
            if let Some(id) = &fixtures.provider.quality_profile_id {
                obj.insert("qualityProfileId".into(), id.clone());
            }
            Some(body)
        }
        // AutoTagging/CustomFormat both reject an empty `tags`/`specifications`
        // array (business-rule validation, not visible in the OpenAPI schema).
        // `GET .../schema` gives a valid specification item; a live tag was
        // created during priming for the `tags` requirement. The specification
        // item's own `name` ("condition name") isn't in the schema template
        // either — Sonarr rejects it as empty unless set explicitly.
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_autotagging") => {
            let spec = named_provider_template(
                &fixtures.provider.autotagging_spec,
                kind,
                "autotagging-condition",
            )?;
            let tag = fixtures.provider.tag_id.clone()?;
            Some(json!({
                "name": unique_live_label(kind, op.name),
                "removeTagsAutomatically": false,
                "tags": [tag],
                "specifications": [spec],
            }))
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_customformat") => {
            let spec = named_provider_template(
                &fixtures.provider.customformat_spec,
                kind,
                "customformat-condition",
            )?;
            Some(json!({
                "name": unique_live_label(kind, op.name),
                "specifications": [spec],
            }))
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_delayprofile") => {
            let tag = fixtures.provider.tag_id.clone()?;
            Some(json!({
                "enableUsenet": true,
                "enableTorrent": true,
                "preferredProtocol": "usenet",
                "usenetDelay": 0,
                "torrentDelay": 0,
                "bypassIfHighestQuality": false,
                "bypassIfAboveCustomFormatScore": false,
                "minimumCustomFormatScore": 0,
                "order": 1,
                "tags": [tag],
            }))
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_rootfolder") => fixtures
            .provider
            .unmapped_root_folder_path
            .as_ref()
            .map(|path| json!({ "path": path })),
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_remotepathmapping") => Some(json!({
            "host": unique_live_label(kind, op.name),
            "remotePath": "/downloads/",
            "localPath": fixtures.provider.root_folder_path.as_deref().unwrap_or("/data"),
        })),
        // `ReleaseProfileResource.required`/`ignored` are untyped (nullable) in the
        // spec, so the generic synthesizer emits `{}`; Sonarr needs at least one of
        // the two populated with a term (string or string array both accepted).
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_releaseprofile") => Some(json!({
            "required": unique_live_label(kind, op.name),
        })),
        // Bulk PUT ops need a real `ids: [...]` array pulled from a resource this
        // same contract sweep already created earlier in phase 2 (POST creates run
        // before PUTs; see `seed_phase`) — the generic synth leaves `ids` empty.
        (ServiceKind::Sonarr | ServiceKind::Radarr, "put_customformat_bulk") => {
            bulk_ids_body(fixtures, "/api/v3/customformat")
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "put_downloadclient_bulk") => {
            bulk_ids_body(fixtures, "/api/v3/downloadclient")
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "put_importlist_bulk") => {
            bulk_ids_body(fixtures, "/api/v3/importlist")
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "put_indexer_bulk") => {
            bulk_ids_body(fixtures, "/api/v3/indexer")
        }
        _ => None,
    }
}

fn named_provider_template(
    template: &Option<Value>,
    kind: ServiceKind,
    op_name: &str,
) -> Option<Value> {
    let mut body = template.clone()?;
    body.as_object_mut()?
        .insert("name".into(), json!(unique_live_label(kind, op_name)));
    Some(body)
}

fn bulk_ids_body(fixtures: &FixtureStore, path: &str) -> Option<Value> {
    let ids = fixtures.values_for(path)?;
    if ids.is_empty() {
        return None;
    }
    Some(json!({ "ids": ids }))
}

pub(super) fn unique_live_label(kind: ServiceKind, op_name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("yarr-live-{}-{op_name}-{nanos}", kind.as_str())
}

fn apply_fixture_args(
    kind: ServiceKind,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    args: &mut Map<String, Value>,
) {
    for param in op.query_params {
        if let Some(value) = fixture_arg_value(kind, op, fixtures, param)
            && (args.contains_key(*param) || should_seed_optional_query(param))
        {
            args.insert((*param).to_string(), value);
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
        return Some(json!(["FriendlyName=Yarr Live Plex"]));
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
        return fixture_path_value(fixtures, parent, param).or_else(|| Some(json!("yarr-live")));
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
        ServiceKind::Jellyfin | ServiceKind::Plex => "/data/yarr-live-plex-movies",
        _ => "/tmp",
    }
}

fn live_search_term(kind: ServiceKind) -> &'static str {
    match kind {
        ServiceKind::Sonarr => "silo",
        ServiceKind::Radarr => "the matrix",
        ServiceKind::Prowlarr => "ubuntu",
        ServiceKind::Jellyfin | ServiceKind::Plex => "yarr",
        _ => "yarr",
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

#[cfg(test)]
#[path = "contract_tests.rs"]
mod tests;
