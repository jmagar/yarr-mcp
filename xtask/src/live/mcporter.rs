//! mcporter-backed live contract harness for the generated MCP Code Mode surface.
//!
//! This starts a local Yarr MCP server against the guarded shart environment and
//! uses `mcporter call ... yarr` to execute generated per-service callables over
//! the MCP transport. It mirrors the CLI contract suite's synthesis, seeding, skip,
//! and response-schema validation rules so both suites cover the same OpenAPI
//! surface through different transports.

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::process::Command;

use yarr::ServiceKind;
use yarr::openapi::{self, OperationSpec};

use super::contract::{self, PreparedOp, RunOut, synth::Spec};
use super::{LIVE_PORT, guard, live_base_url, process, report, reset};

// Keep chunks small enough to avoid Code Mode's 30s script budget while still
// avoiding a separate Node/mcporter process for every generated operation. If a
// chunk trips a transport/length limit, `run_chunk` recursively splits it so one
// large response cannot poison neighboring operations.
const BATCH_SIZE: usize = 5;
const MCPORTER_ATTEMPTS: usize = 3;
const MCPORTER_TIMEOUT: &str = "45s";

pub(super) fn run(
    report: &mut report::Report,
    yarr: &process::YarrProcess,
    matrix: &super::matrix::Matrix,
    no_destructive: bool,
    only_service: Option<&str>,
) -> Result<()> {
    ensure_mcporter_available()?;

    let configured: std::collections::BTreeSet<&str> =
        matrix.services.iter().map(|s| s.kind.as_str()).collect();
    let base = live_base_url();
    let mut env = BTreeMap::new();
    env.insert("YARR_MCP_NO_AUTH".into(), "true".into());
    env.insert("YARR_NOAUTH".into(), "false".into());
    env.insert("YARR_HTTP_TIMEOUT_SECS".into(), "20".into());
    if !no_destructive {
        env.insert("YARR_ALLOW_DESTRUCTIVE".into(), "true".into());
    }
    let mut server = yarr.start_server_args(&["serve", "mcp"], "127.0.0.1", LIVE_PORT, &env)?;
    server.wait_healthy(&base)?;

    for (svc, spec_path) in contract::SPECS {
        if only_service.is_some_and(|only| only != *svc) {
            continue;
        }
        if !configured.contains(svc) {
            continue;
        }
        let kind = contract::kind_of(svc).expect("spec-backed kind");
        contract::seed_service_fixtures(yarr, svc, kind)
            .with_context(|| format!("seed live fixtures for {svc}"))?;
        let spec = Spec::load(spec_path).with_context(|| format!("load {spec_path}"))?;
        let ops: Vec<&'static OperationSpec> = openapi::operations_for_kind(kind).iter().collect();
        println!(
            "mcporter contract {svc}: calling {} generated OpenAPI callables via yarr",
            ops.len()
        );

        let mut fixtures = contract::FixtureStore::default();
        let mut results = Vec::with_capacity(ops.len());
        for phase in 0u8..=4 {
            let phase_ops: Vec<&'static OperationSpec> = ops
                .iter()
                .copied()
                .filter(|op| contract::seed_phase(op) == phase)
                .collect();
            let outs = run_phase(
                &base,
                svc,
                kind,
                &spec,
                &fixtures,
                &phase_ops,
                no_destructive,
            );
            contract::harvest_into(&mut fixtures, &outs);
            results.extend(outs.into_iter().map(|(_, result, _)| result));
        }
        let reset_outs = run_reset_required_ops(&base, yarr, svc, kind, &spec, &fixtures, &ops);
        results.extend(reset_outs.into_iter().map(|(_, result, _)| result));

        write_detail(svc, &results)?;
        let status = contract::contract_status(&results);
        let detail = format!("{} via mcporter/yarr over MCP against shart", status.detail);
        if status.passed {
            report.pass(format!("mcporter contract {svc}"), detail);
        } else {
            report.fail(format!("mcporter contract {svc}"), detail);
        }
    }

    Ok(())
}

fn run_phase(
    base: &str,
    svc: &str,
    kind: ServiceKind,
    spec: &Spec,
    fixtures: &contract::FixtureStore,
    ops: &[&'static OperationSpec],
    no_destructive: bool,
) -> Vec<RunOut> {
    let mut outs = Vec::with_capacity(ops.len());
    let mut prepared = Vec::new();
    for op in ops {
        if contract::op_requires_stack_reset(op) {
            continue;
        }
        match contract::prepare_op_args(kind, spec, op, fixtures, no_destructive, false) {
            PreparedOp::Call(args) => prepared.push(PreparedCall { op: *op, args }),
            PreparedOp::Skip(detail) => outs.push((*op, op_result(op, "skipped", detail), None)),
        }
    }

    for chunk in prepared.chunks(BATCH_SIZE) {
        outs.extend(run_chunk(base, svc, spec, chunk));
    }
    outs
}

fn run_reset_required_ops(
    base: &str,
    yarr: &process::YarrProcess,
    svc: &str,
    kind: ServiceKind,
    spec: &Spec,
    fixtures: &contract::FixtureStore,
    ops: &[&'static OperationSpec],
) -> Vec<RunOut> {
    let reset_ops: Vec<_> = ops
        .iter()
        .copied()
        .filter(|op| contract::op_requires_stack_reset(op))
        .collect();
    if reset_ops.is_empty() {
        return Vec::new();
    }

    if reset::target_for(svc).is_none() {
        return reset_ops
            .into_iter()
            .map(|op| {
                (
                    op,
                    op_result(
                        op,
                        "skipped",
                        "requires stack reset/reseed but no shart ZFS golden target exists for this service".into(),
                    ),
                    None,
                )
            })
            .collect();
    }

    let mut outs = Vec::with_capacity(reset_ops.len());
    let mut prepared = Vec::new();
    for op in reset_ops {
        match contract::prepare_op_args(kind, spec, op, fixtures, false, true) {
            PreparedOp::Call(args) => prepared.push(PreparedCall { op, args }),
            PreparedOp::Skip(detail) => outs.push((op, op_result(op, "skipped", detail), None)),
        }
    }

    if let Err(err) = reset_after_op(yarr, svc) {
        outs.extend(prepared.into_iter().map(|call| {
            (
                call.op,
                op_result(
                    call.op,
                    "rejected",
                    format!("pre-phase reset failed: {err}"),
                ),
                None,
            )
        }));
        return outs;
    }

    let mut reset_results = Vec::new();
    for chunk in prepared.chunks(BATCH_SIZE) {
        reset_results.extend(run_chunk(base, svc, spec, chunk));
    }

    if let Err(err) = reset_after_op(yarr, svc) {
        outs.extend(prepared.into_iter().map(|call| {
            (
                call.op,
                op_result(
                    call.op,
                    "rejected",
                    format!("post-phase reset failed: {err}"),
                ),
                None,
            )
        }));
    } else {
        outs.extend(reset_results);
    }
    outs
}

fn reset_after_op(yarr: &process::YarrProcess, svc: &str) -> Result<()> {
    reset::reset_service(svc)?;
    if let Some(url) = reset::service_url(&yarr.env, svc) {
        reset::wait_service_url(&url)?;
    }
    Ok(())
}

struct PreparedCall {
    op: &'static OperationSpec,
    args: Map<String, Value>,
}

fn run_chunk(base: &str, svc: &str, spec: &Spec, calls: &[PreparedCall]) -> Vec<RunOut> {
    let mut last_error = None;
    for attempt in 1..=MCPORTER_ATTEMPTS {
        match invoke_chunk(base, svc, calls) {
            Ok(values) => return classify_chunk(spec, calls, values),
            Err(err) => {
                let detail = err.to_string();
                if attempt < MCPORTER_ATTEMPTS && contract::is_retryable_contract_error(&detail) {
                    last_error = Some(detail);
                    std::thread::sleep(std::time::Duration::from_millis(750));
                    continue;
                }
                let prior = if attempt > 1 {
                    last_error
                        .map(|e| format!("; previous retryable error: {e}"))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let detail = format!("mcporter batch failed: {detail}{prior}");
                if calls.len() > 1 && should_split_failed_batch(&detail) {
                    let mid = calls.len() / 2;
                    let mut split = run_chunk(base, svc, spec, &calls[..mid]);
                    split.extend(run_chunk(base, svc, spec, &calls[mid..]));
                    return split;
                }
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
        }
    }
    unreachable!("mcporter invoke loop always returns")
}

fn should_split_failed_batch(detail: &str) -> bool {
    detail.contains("length limit exceeded")
        || detail.contains("mcporter output was not JSON")
        || detail.contains("mcporter result did not include artifact")
}

fn invoke_chunk(base: &str, svc: &str, calls: &[PreparedCall]) -> Result<Vec<Value>> {
    let code = batch_code(svc, calls)?;
    let payload = serde_json::to_string(&json!({ "code": code }))?;
    let url = format!("{base}/mcp");
    let output = Command::new("timeout")
        .args([
            "--kill-after=5s",
            MCPORTER_TIMEOUT,
            "mcporter",
            "call",
            "--http-url",
            &url,
            "--allow-http",
            "yarr",
            "--args",
            &payload,
            "--output",
            "text",
        ])
        .output()
        .context("failed to run mcporter")?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    let value: Value = serde_json::from_str(trimmed)
        .with_context(|| format!("mcporter output was not JSON: {}", preview(trimmed)))?;
    let run_id = value
        .get("artifactsRunId")
        .and_then(Value::as_str)
        .context("mcporter output did not include artifactsRunId")?;
    let artifact_path = operation_artifact_candidate(&value)
        .and_then(Value::as_str)
        .with_context(|| format!("mcporter result did not include artifact path: {trimmed}"))?;
    let full_path = std::path::Path::new(guard::SHART_HOME)
        .join("codemode/artifacts")
        .join(run_id)
        .join(artifact_path);
    let raw = std::fs::read_to_string(&full_path)
        .with_context(|| format!("read mcporter result artifact {}", full_path.display()))?;
    let array: Vec<Value> = serde_json::from_str(&raw)
        .with_context(|| format!("parse mcporter result artifact {}", full_path.display()))?;
    Ok(array)
}

fn operation_artifact_candidate(value: &Value) -> Option<&Value> {
    let mut result = value;
    for _ in 0..4 {
        if let Some(path) = result
            .pointer("/artifact/path")
            .or_else(|| result.get("artifact"))
        {
            return Some(path);
        }
        let next = result.get("result")?;
        result = next;
    }
    None
}

fn batch_code(svc: &str, calls: &[PreparedCall]) -> Result<String> {
    let mut code = String::from("async () => {\n  const out = [];\n");
    for call in calls {
        let args = serde_json::to_string(&call.args)?;
        code.push_str(&format!(
            "  try {{ out.push({{ name: {:?}, args: {}, ok: true, value: await {}.{}({}) }}); }} catch (e) {{ out.push({{ name: {:?}, args: {}, ok: false, error: String(e) }}); }}\n",
            call.op.name, args, svc, call.op.name, args, call.op.name, args
        ));
    }
    let artifact = format!(
        "live-mcporter/{}-{}-{}.json",
        svc,
        calls.first().map(|call| call.op.name).unwrap_or("empty"),
        calls.len()
    );
    code.push_str(&format!(
        "  const artifact = writeArtifact({}, JSON.stringify(out), {{ contentType: 'application/json' }});\n  return {{ artifact: artifact.path }};\n}}",
        serde_json::to_string(&artifact)?
    ));
    Ok(code)
}

fn classify_chunk(spec: &Spec, calls: &[PreparedCall], values: Vec<Value>) -> Vec<RunOut> {
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

fn is_empty_body_sentinel(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(map)
            if map.len() == 2
                && map.get("ok") == Some(&Value::Bool(true))
                && map.get("status").is_some_and(Value::is_number)
    )
}

fn op_result(op: &OperationSpec, outcome: &'static str, detail: String) -> contract::OpResult {
    contract::OpResult {
        name: op.name,
        method: op.method.as_str(),
        path: op.path,
        outcome,
        detail,
        args: None,
    }
}

fn op_result_with_args(
    op: &OperationSpec,
    outcome: &'static str,
    detail: String,
    args: &Map<String, Value>,
) -> contract::OpResult {
    contract::OpResult {
        name: op.name,
        method: op.method.as_str(),
        path: op.path,
        outcome,
        detail,
        args: Some(Value::Object(args.clone())),
    }
}

fn ensure_mcporter_available() -> Result<()> {
    let output = Command::new("mcporter")
        .arg("--version")
        .output()
        .context("mcporter is required for `cargo xtask live --suite mcporter`")?;
    if !output.status.success() {
        bail!(
            "mcporter is required for `cargo xtask live --suite mcporter`: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn write_detail(svc: &str, results: &[contract::OpResult]) -> Result<()> {
    let dir = std::path::Path::new("target/live-full");
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("mcporter-contract-{svc}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(results)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn preview(raw: &str) -> String {
    raw.chars().take(240).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_failures_split_only_for_transport_artifact_limits() {
        assert!(should_split_failed_batch(
            "mcporter batch failed: Streamable HTTP error: length limit exceeded"
        ));
        assert!(should_split_failed_batch(
            "mcporter batch failed: mcporter output was not JSON"
        ));
        assert!(!should_split_failed_batch(
            "mcporter batch failed: Error: sonarr returned HTTP 400"
        ));
    }
}
