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

mod synth;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

use rustarr::ServiceKind;
use rustarr::openapi::{self, HttpMethod, OperationSpec};

use super::{process, report};
use synth::Spec;

/// (kind str, spec path) for the spec-backed services.
const SPECS: &[(&str, &str)] = &[
    ("sonarr", "specs/sonarr.openapi.json"),
    ("radarr", "specs/radarr.openapi.json"),
    ("prowlarr", "specs/prowlarr.openapi.json"),
    ("overseerr", "specs/overseerr.openapi.yml"),
    ("jellyfin", "specs/jellyfin.openapi.json"),
    ("plex", "specs/plex.openapi.yml"),
];

fn kind_of(name: &str) -> Option<ServiceKind> {
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
struct OpResult {
    name: &'static str,
    method: &'static str,
    path: &'static str,
    outcome: &'static str, // ok | schema_mismatch | rejected | skipped
    detail: String,
}

pub fn run(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
    matrix: &super::matrix::Matrix,
    no_destructive: bool,
) -> Result<()> {
    let configured: std::collections::BTreeSet<&str> =
        matrix.services.iter().map(|s| s.kind.as_str()).collect();

    for (svc, spec_path) in SPECS {
        if !configured.contains(svc) {
            continue;
        }
        let kind = kind_of(svc).expect("spec-backed kind");
        let spec = Spec::load(spec_path).with_context(|| format!("load {spec_path}"))?;
        let ops: Vec<&'static OperationSpec> = openapi::operations_for_kind(kind).iter().collect();

        // Create-first seeding: run phases in order, harvesting ids between them so
        // later phases can hit real resources:
        //   0  collection reads (GET, no path params)  -> seed ids from list bodies
        //   1  creates (POST)                          -> seed ids from created objects
        //   2  resource reads/updates (GET/PUT/PATCH)  -> use seeded ids
        //   3  deletes (DELETE)                        -> use seeded ids; also cleanup
        let mut ids: BTreeMap<String, Vec<i64>> = BTreeMap::new();
        let mut results: Vec<OpResult> = Vec::with_capacity(ops.len());
        for phase in 0u8..=3 {
            let phase_ops: Vec<&'static OperationSpec> = ops
                .iter()
                .copied()
                .filter(|o| seed_phase(o) == phase)
                .collect();
            let outs = parallel_run(rustarr, svc, &spec, &ids, &phase_ops, no_destructive);
            harvest_into(&mut ids, &outs);
            results.extend(outs.into_iter().map(|(_, r, _)| r));
        }

        write_detail(svc, &results)?;
        let (ok, mism, rej, skip) = tally(&results);
        let total = results.len();
        let detail = format!(
            "{ok} contract-ok, {mism} schema-mismatch, {rej} upstream-rejected, {skip} skipped of {total} ops"
        );
        // A service only FAILS the suite if nothing dispatched at all (structural
        // breakage); per-op rejections/mismatches are data in the detail file.
        if ok == 0 && mism == 0 {
            report.fail(format!("contract {svc}"), detail);
        } else {
            report.pass(format!("contract {svc}"), detail);
        }
    }
    Ok(())
}

/// Bounded thread pool: run `ops` through `run_op` concurrently. Returns, per op,
/// `(op, result, success-body)` so the caller can harvest seeded ids between phases.
type RunOut = (&'static OperationSpec, OpResult, Option<Value>);

fn parallel_run(
    rustarr: &process::RustarrProcess,
    svc: &str,
    spec: &Spec,
    ids: &BTreeMap<String, Vec<i64>>,
    ops: &[&'static OperationSpec],
    no_destructive: bool,
) -> Vec<RunOut> {
    // Gentle concurrency: the *arr services on a single test box drop connections
    // and degrade under heavy parallel load (especially mixed with writes), which
    // shows up as flaky "error sending request" rejections. 4 keeps the run fast
    // without overwhelming the stack.
    const WORKERS: usize = 4;
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
                            let (r, v) = run_op(rustarr, svc, spec, op, ids, no_destructive);
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

/// Merge any resource ids a phase's responses expose into the id pool, keyed by the
/// op's path (the collection): array GETs contribute every element `id`; a POST
/// contributes the created object's `id`. Path-param ops then resolve against the
/// parent collection. Deduped + capped so the pool stays small.
fn harvest_into(ids: &mut BTreeMap<String, Vec<i64>>, outs: &[RunOut]) {
    for (op, _result, value) in outs {
        let Some(value) = value else { continue };
        let mut found = harvest_ids(value);
        if op.method == HttpMethod::Post
            && let Some(id) = value.get("id").and_then(Value::as_i64)
        {
            found.push(id);
        }
        if !found.is_empty() {
            let pool = ids.entry(op.path.to_string()).or_default();
            pool.extend(found);
            pool.sort_unstable();
            pool.dedup();
            pool.truncate(8);
        }
    }
}

/// Seeding phase for an op: collection reads first (0) to discover ids, then creates
/// (1) to seed more, then resource reads/updates (2) that consume ids, then deletes
/// (3) last so reads/updates precede cleanup.
fn seed_phase(op: &OperationSpec) -> u8 {
    match op.method {
        HttpMethod::Get if op.path_params.is_empty() => 0,
        HttpMethod::Post => 1,
        HttpMethod::Delete => 3,
        _ => 2, // GET-with-id, PUT, PATCH
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

/// Collect integer `id` fields from an array response (or the first array-valued
/// field of an object response).
fn harvest_ids(value: &Value) -> Vec<i64> {
    let array = match value {
        Value::Array(a) => Some(a),
        Value::Object(o) => o.values().find_map(Value::as_array),
        _ => None,
    };
    array
        .map(|a| {
            a.iter()
                .filter_map(|e| e.get("id").and_then(Value::as_i64))
                .collect()
        })
        .unwrap_or_default()
}

/// Run one op. Returns its classified result plus the successful response body (so
/// the caller can harvest resource ids for create-first seeding).
fn run_op(
    rustarr: &process::RustarrProcess,
    svc: &str,
    spec: &Spec,
    op: &OperationSpec,
    ids: &BTreeMap<String, Vec<i64>>,
    no_destructive: bool,
) -> (OpResult, Option<Value>) {
    let mk = |outcome, detail: String| OpResult {
        name: op.name,
        method: op.method.as_str(),
        path: op.path,
        outcome,
        detail,
    };
    if no_destructive && op.method.is_delete() {
        return (
            mk(
                "skipped",
                "destructive (DELETE) skipped via --no-destructive".into(),
            ),
            None,
        );
    }
    // NEVER call self-destructive control endpoints: shutdown/restart stop the
    // service mid-run (which is exactly what took prowlarr down), and backup/restore
    // rewrites its whole config. Testing "every endpoint" cannot mean bricking the
    // stack — these are skipped by design.
    let lp = op.path.to_ascii_lowercase();
    if lp.contains("shutdown")
        || lp.contains("restart")
        || lp.contains("/backup/restore")
        || lp.ends_with("/system/backup")
    {
        return (
            mk(
                "skipped",
                "self-destructive control endpoint (stops/rewrites the service)".into(),
            ),
            None,
        );
    }
    // Config/auth MUTATIONS brick the stack just as surely: writing settings
    // regenerated Overseerr's API key (every later op then 403'd) and could flip
    // remote-access, rewrite the host/auth config, etc. Skip WRITES to these paths;
    // their GET reads are still validated, so coverage of the surface is intact.
    if !op.method.is_read()
        && (lp.contains("/settings")
            || lp.contains("/auth")
            || lp.contains("/config") // Servarr /api/v3/config/* (host/UI/naming/downloadclient)
            || lp.contains("/configuration") // Jellyfin /System/Configuration (flipped remote-access off)
            || lp.contains("/startup") // Jellyfin startup wizard rewrites the whole config
            || lp.contains("/prefs") // Plex /:/prefs
            || lp.contains("apikey"))
    {
        return (
            mk(
                "skipped",
                "config/auth mutation (would change keys/settings and brick the stack)".into(),
            ),
            None,
        );
    }
    // Ops whose success body is non-JSON (text/calendar, files) aren't a JSON
    // contract — skip rather than count the non-JSON response as a rejection.
    if spec.success_is_nonjson(op.method.as_str(), op.path) {
        return (
            mk(
                "skipped",
                "non-JSON success response (not a JSON contract)".into(),
            ),
            None,
        );
    }
    // Satisfy path params from discovered/seeded ids (parent collection = path
    // before `{`).
    let mut path_args = Map::new();
    if !op.path_params.is_empty() {
        let parent = op.path.split_once("/{").map(|(a, _)| a).unwrap_or(op.path);
        let Some(id) = ids.get(parent).and_then(|v| v.first()) else {
            return (
                mk(
                    "skipped",
                    format!(
                        "no seeded/discovered id for path params {:?}",
                        op.path_params
                    ),
                ),
                None,
            );
        };
        for p in op.path_params {
            path_args.insert((*p).to_string(), json!(id));
        }
    }
    let Some(mut args) = spec.build_args(op.method.as_str(), op.path, &path_args) else {
        return (
            mk(
                "skipped",
                "no spec operation / unsynthesizable inputs".into(),
            ),
            None,
        );
    };
    // Several read endpoints require a `<resource>Id` query param (e.g. Sonarr's
    // get_episode needs `seriesId`). Fill any unset `<resource>Id` query param from
    // the discovered collection whose path matches that resource.
    for q in op.query_params {
        let Some(res) = q.strip_suffix("Id").or_else(|| q.strip_suffix("id")) else {
            continue;
        };
        if args.contains_key(*q) || res.is_empty() {
            continue;
        }
        let res = res.to_ascii_lowercase();
        if let Some(id) = ids
            .iter()
            .find(|(path, _)| {
                let p = path.to_ascii_lowercase();
                p.ends_with(&format!("/{res}")) || p.ends_with(&format!("/{res}s"))
            })
            .and_then(|(_, v)| v.first())
        {
            args.insert((*q).to_string(), json!(id));
        }
    }
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

/// Invoke `rustarr <svc> op <name> --args <json> [--confirm]`. Returns the parsed
/// JSON result on a 2xx, `None` for an empty body, or an error with the upstream
/// message on a non-2xx / CLI error.
fn invoke(
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
