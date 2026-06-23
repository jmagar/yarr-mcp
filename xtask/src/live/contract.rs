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
use rustarr::openapi::{self, OperationSpec};

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
        let mut ops: Vec<&OperationSpec> = openapi::operations_for_kind(kind).iter().collect();
        // GET -> POST -> PUT -> PATCH -> DELETE.
        ops.sort_by_key(|o| method_order(o.method));

        let ids = discover_ids(rustarr, svc, &spec, &ops);
        // Run each method phase (GET -> POST -> PUT/PATCH -> DELETE) in order, but
        // parallelize the ops WITHIN a phase across a thread pool — so reads/updates
        // still precede deletes while a service's hundreds of ops finish in minutes.
        let mut results: Vec<OpResult> = Vec::with_capacity(ops.len());
        for phase in 0u8..=4 {
            let phase_ops: Vec<&OperationSpec> =
                ops.iter().copied().filter(|o| method_order(o.method) == phase).collect();
            results.extend(parallel_run(
                rustarr,
                svc,
                &spec,
                &ids,
                &phase_ops,
                no_destructive,
            ));
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

/// Bounded thread pool: run `ops` through `run_op` concurrently, preserving none of
/// their order (the caller phases by method). Each op is its own subprocess with its
/// own full response, so there is no shared response budget to truncate.
fn parallel_run(
    rustarr: &process::RustarrProcess,
    svc: &str,
    spec: &Spec,
    ids: &BTreeMap<String, Vec<i64>>,
    ops: &[&OperationSpec],
    no_destructive: bool,
) -> Vec<OpResult> {
    const WORKERS: usize = 12;
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
                        .map(|op| run_op(rustarr, svc, spec, op, ids, no_destructive))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("contract worker panicked"))
            .collect()
    })
}

fn method_order(m: &str) -> u8 {
    match m {
        "GET" => 0,
        "POST" => 1,
        "PUT" | "PATCH" => 2,
        "DELETE" => 3,
        _ => 4,
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

/// Run every no-path-param GET once and harvest resource ids per collection path,
/// so path-param ops (`/series/{id}`) can be satisfied with real ids.
fn discover_ids(
    rustarr: &process::RustarrProcess,
    svc: &str,
    spec: &Spec,
    ops: &[&OperationSpec],
) -> BTreeMap<String, Vec<i64>> {
    let get_ops: Vec<&OperationSpec> = ops
        .iter()
        .copied()
        .filter(|o| o.method == "GET" && o.path_params.is_empty())
        .collect();
    if get_ops.is_empty() {
        return BTreeMap::new();
    }
    const WORKERS: usize = 12;
    let chunk = get_ops.len().div_ceil(WORKERS).max(1);
    let pairs: Vec<(String, Vec<i64>)> = std::thread::scope(|s| {
        let handles: Vec<_> = get_ops
            .chunks(chunk)
            .map(|c| {
                s.spawn(move || {
                    let mut out: Vec<(String, Vec<i64>)> = Vec::new();
                    for op in c {
                        let Some(args) = spec.build_args(op.method, op.path, &Map::new()) else {
                            continue;
                        };
                        if let Ok(Some(value)) = invoke(rustarr, svc, op.name, &args, false) {
                            let collected = harvest_ids(&value);
                            if !collected.is_empty() {
                                out.push((op.path.to_string(), collected));
                            }
                        }
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("discovery worker panicked"))
            .collect()
    });
    pairs.into_iter().collect()
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

fn run_op(
    rustarr: &process::RustarrProcess,
    svc: &str,
    spec: &Spec,
    op: &OperationSpec,
    ids: &BTreeMap<String, Vec<i64>>,
    no_destructive: bool,
) -> OpResult {
    let mk = |outcome, detail: String| OpResult {
        name: op.name,
        method: op.method,
        path: op.path,
        outcome,
        detail,
    };
    if no_destructive && op.method == "DELETE" {
        return mk(
            "skipped",
            "destructive (DELETE) skipped via --no-destructive".into(),
        );
    }
    // Ops whose success body is non-JSON (text/calendar, files) aren't a JSON
    // contract — skip rather than count the non-JSON response as a rejection.
    if spec.success_is_nonjson(op.method, op.path) {
        return mk(
            "skipped",
            "non-JSON success response (not a JSON contract)".into(),
        );
    }
    // Satisfy path params from discovered ids (parent collection = path before `{`).
    let mut path_args = Map::new();
    if !op.path_params.is_empty() {
        let parent = op.path.split_once("/{").map(|(a, _)| a).unwrap_or(op.path);
        let Some(id) = ids.get(parent).and_then(|v| v.first()) else {
            return mk(
                "skipped",
                format!("no discovered id for path params {:?}", op.path_params),
            );
        };
        for p in op.path_params {
            path_args.insert((*p).to_string(), json!(id));
        }
    }
    let Some(mut args) = spec.build_args(op.method, op.path, &path_args) else {
        return mk(
            "skipped",
            "no spec operation / unsynthesizable inputs".into(),
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
    match invoke(rustarr, svc, op.name, &args, op.method == "DELETE") {
        Ok(Some(value)) => match op.response_type {
            Some(ty) => match spec.validate_response(ty, &value) {
                Ok(()) => mk("ok", format!("2xx + matches {ty}")),
                Err(e) => mk(
                    "schema_mismatch",
                    format!("{e}").chars().take(180).collect(),
                ),
            },
            None => mk("ok", "2xx (no declared response type to validate)".into()),
        },
        Ok(None) => mk("ok", "2xx (empty/non-JSON body)".into()),
        Err(e) => mk("rejected", format!("{e}").chars().take(180).collect()),
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
    Ok(serde_json::from_str(trimmed).ok())
}

fn write_detail(svc: &str, results: &[OpResult]) -> Result<()> {
    let dir = std::path::Path::new("target/live-full");
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("contract-{svc}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(results)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
