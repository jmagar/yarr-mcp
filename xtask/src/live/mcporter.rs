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
use std::net::TcpListener;
use std::process::Command;

use yarr::ServiceKind;
use yarr::openapi::{self, OperationSpec};

use super::contract::{self, PreparedOp, RunOut, synth::Spec};
use super::{guard, process, report, reset};

mod classify;
mod io;

use classify::{classify_chunk, should_retry_domain_result};
use io::mcporter_output;

// Keep chunks small enough to avoid Code Mode's 30s script budget while still
// avoiding a separate Node/mcporter process for every generated operation. If a
// chunk trips a transport/length limit, `run_chunk` recursively splits it so one
// large response cannot poison neighboring operations.
const BATCH_SIZE: usize = 4;
const MCPORTER_ATTEMPTS: usize = 1;
const MCPORTER_TIMEOUT: &str = "40s";

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

    for (svc, spec_path) in contract::SPECS {
        if only_service.is_some_and(|only| only != *svc) {
            continue;
        }
        if !configured.contains(svc) {
            continue;
        }
        let kind = contract::kind_of(svc).expect("spec-backed kind");
        if reset::target_for(svc).is_some() {
            reset_after_op(yarr, svc)
                .with_context(|| format!("reset live fixture baseline for {svc}"))?;
        }
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
                yarr,
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
        let reset_outs =
            run_reset_required_ops(yarr, svc, kind, &spec, &fixtures, &ops, no_destructive);
        results.extend(reset_outs.into_iter().map(|(_, result, _)| result));

        write_detail(svc, &results)?;
        let status = contract::contract_status(&results);
        let detail = format!("{} via mcporter/yarr over MCP against shart", status.detail);
        if status.passed {
            report.pass(format!("mcporter contract {svc}"), detail);
        } else {
            report.fail(format!("mcporter contract {svc}"), detail);
        }
        if let Err(err) = contract::cleanup_service_fixtures(kind) {
            eprintln!("warning: failed to clean live fixtures for {svc}: {err:#}");
        }
    }

    Ok(())
}

fn reserve_local_port() -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).context("reserve mcporter MCP port")?;
    Ok(listener.local_addr()?.port())
}

fn run_phase(
    yarr: &process::YarrProcess,
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
            PreparedOp::Call(args) => prepared.push(PreparedCall { kind, op, args }),
            PreparedOp::Skip(detail) => outs.push((
                *op,
                op_result(
                    op,
                    "rejected",
                    format!("missing executable fixture: {detail}"),
                ),
                None,
            )),
        }
    }

    let mut harness = match McpHarness::start(yarr, no_destructive) {
        Ok(harness) => harness,
        Err(err) => {
            let detail = format!("failed to start isolated MCP server: {err}");
            outs.extend(prepared.iter().map(|call| {
                (
                    call.op,
                    op_result(call.op, "rejected", detail.clone()),
                    None,
                )
            }));
            return outs;
        }
    };
    for chunk in prepared.chunks(BATCH_SIZE) {
        outs.extend(harness.run_chunk(svc, spec, chunk));
    }
    outs
}

fn run_reset_required_ops(
    yarr: &process::YarrProcess,
    svc: &str,
    kind: ServiceKind,
    spec: &Spec,
    fixtures: &contract::FixtureStore,
    ops: &[&'static OperationSpec],
    no_destructive: bool,
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
                        "rejected",
                        "requires stack reset/reseed but no shart ZFS golden target exists for this service".into(),
                    ),
                    None,
                )
            })
            .collect();
    }
    if no_destructive {
        return reset_ops
            .into_iter()
            .map(|op| {
                (
                    op,
                    op_result(
                        op,
                        "rejected",
                        "requires stack reset/reseed and is skipped via --no-destructive".into(),
                    ),
                    None,
                )
            })
            .collect();
    }

    let mut outs = Vec::with_capacity(reset_ops.len());
    for op in reset_ops {
        match contract::prepare_op_args(kind, spec, op, fixtures, no_destructive, true) {
            PreparedOp::Call(args) => {
                let call = PreparedCall { kind, op, args };
                let mut result =
                    run_chunk(yarr, svc, spec, std::slice::from_ref(&call), no_destructive);
                outs.append(&mut result);
                if let Err(err) = reset_after_op(yarr, svc) {
                    mark_reset_failure(
                        &mut outs,
                        call.op,
                        format!("post-operation reset failed: {err}"),
                    );
                } else if let Err(err) = contract::seed_service_fixtures(yarr, svc, kind) {
                    mark_reset_failure(
                        &mut outs,
                        call.op,
                        format!("post-operation reseed failed: {err}"),
                    );
                }
            }
            PreparedOp::Skip(detail) => outs.push((
                op,
                op_result(
                    op,
                    "rejected",
                    format!("missing executable reset fixture: {detail}"),
                ),
                None,
            )),
        }
    }
    outs
}

fn mark_reset_failure(outs: &mut [RunOut], op: &'static OperationSpec, detail: String) {
    if let Some((_, result, value)) = outs.iter_mut().rev().find(|(candidate, _, _)| {
        candidate.name == op.name && candidate.method == op.method && candidate.path == op.path
    }) {
        result.outcome = "rejected";
        result.detail = detail;
        *value = None;
    }
}

fn reset_after_op(yarr: &process::YarrProcess, svc: &str) -> Result<()> {
    reset::reset_service(svc)?;
    if let Some(url) = reset::service_url(&yarr.env, svc) {
        reset::wait_service_url(&url)?;
    }
    Ok(())
}

struct PreparedCall {
    kind: ServiceKind,
    op: &'static OperationSpec,
    args: Map<String, Value>,
}

struct McpHarness<'a> {
    yarr: &'a process::YarrProcess,
    no_destructive: bool,
    _server: Option<process::Server>,
    base: String,
}

impl<'a> McpHarness<'a> {
    fn start(yarr: &'a process::YarrProcess, no_destructive: bool) -> Result<Self> {
        let (server, base) = start_isolated_server(yarr, no_destructive)?;
        Ok(Self {
            yarr,
            no_destructive,
            _server: Some(server),
            base,
        })
    }

    fn restart(&mut self) -> Result<()> {
        self._server.take();
        let (server, base) = start_isolated_server(self.yarr, self.no_destructive)?;
        self._server = Some(server);
        self.base = base;
        Ok(())
    }

    fn run_chunk(&mut self, svc: &str, spec: &Spec, calls: &[PreparedCall]) -> Vec<RunOut> {
        let mut last_error = None;
        for attempt in 1..=2 {
            log_chunk_progress(svc, calls, attempt, "invoke");
            match invoke_chunk(&self.base, svc, calls) {
                Ok(values) => {
                    log_chunk_progress(svc, calls, attempt, "classify");
                    if attempt == 1 && should_retry_domain_result(calls, &values) {
                        last_error = Some("retryable domain response".into());
                        std::thread::sleep(std::time::Duration::from_millis(5000));
                        continue;
                    }
                    return classify_chunk(spec, calls, values);
                }
                Err(err) => {
                    log_chunk_progress(svc, calls, attempt, "error");
                    let detail = err.to_string();
                    if attempt == 1 && is_transport_restart_error(&detail) {
                        last_error = Some(detail);
                        if let Err(restart_err) = self.restart() {
                            let prior = last_error
                                .map(|e| format!("; previous transport error: {e}"))
                                .unwrap_or_default();
                            let detail =
                                format!("mcporter server restart failed: {restart_err}{prior}");
                            return rejected_calls(calls, detail);
                        }
                        continue;
                    }
                    let prior = if attempt > 1 {
                        last_error
                            .map(|e| format!("; previous transport error: {e}"))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    let detail = format!("mcporter batch failed: {detail}{prior}");
                    if calls.len() > 1 && should_split_failed_batch(&detail) {
                        let mid = calls.len() / 2;
                        let mut split = self.run_chunk(svc, spec, &calls[..mid]);
                        split.extend(self.run_chunk(svc, spec, &calls[mid..]));
                        return split;
                    }
                    return rejected_calls(calls, detail);
                }
            }
        }
        unreachable!("mcporter invoke loop always returns")
    }
}

fn run_chunk(
    yarr: &process::YarrProcess,
    svc: &str,
    spec: &Spec,
    calls: &[PreparedCall],
    no_destructive: bool,
) -> Vec<RunOut> {
    let (server, base) = match start_isolated_server(yarr, no_destructive) {
        Ok(server) => server,
        Err(err) => {
            let detail = format!("failed to start isolated MCP server: {err}");
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
    };
    let _server = server;
    run_chunk_on_base(&base, svc, spec, calls)
}

fn start_isolated_server(
    yarr: &process::YarrProcess,
    no_destructive: bool,
) -> Result<(process::Server, String)> {
    let mut env = BTreeMap::new();
    env.insert("YARR_MCP_NO_AUTH".into(), "true".into());
    env.insert("YARR_NOAUTH".into(), "false".into());
    env.insert("YARR_HTTP_TIMEOUT_SECS".into(), "20".into());
    if !no_destructive {
        env.insert("YARR_ALLOW_DESTRUCTIVE".into(), "true".into());
    }

    let mut last_error = None;
    for attempt in 1..=2 {
        let port = reserve_local_port()?;
        let base = format!("http://127.0.0.1:{port}");
        match yarr.start_server_args(&["serve", "mcp"], "127.0.0.1", port, &env) {
            Ok(mut server) => match server.wait_healthy(&base) {
                Ok(()) => return Ok((server, base)),
                Err(err) => {
                    last_error = Some(err);
                    drop(server);
                }
            },
            Err(err) => last_error = Some(err),
        }
        if attempt == 1 {
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    }
    Err(last_error.expect("start attempts always record an error"))
}

fn run_chunk_on_base(base: &str, svc: &str, spec: &Spec, calls: &[PreparedCall]) -> Vec<RunOut> {
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
                    let mut split = run_chunk_on_base(base, svc, spec, &calls[..mid]);
                    split.extend(run_chunk_on_base(base, svc, spec, &calls[mid..]));
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

fn log_chunk_progress(svc: &str, calls: &[PreparedCall], attempt: usize, phase: &str) {
    let names = calls
        .iter()
        .map(|call| call.op.name)
        .collect::<Vec<_>>()
        .join(",");
    eprintln!("mcporter {svc} attempt {attempt} {phase}: {names}");
}

fn rejected_calls(calls: &[PreparedCall], detail: String) -> Vec<RunOut> {
    calls
        .iter()
        .map(|call| {
            (
                call.op,
                op_result(call.op, "rejected", detail.clone()),
                None,
            )
        })
        .collect()
}

fn is_transport_restart_error(detail: &str) -> bool {
    detail.contains("exit status: 124")
        || detail.contains("signal: 9")
        || detail.contains("mcporter output was not JSON")
        || detail.contains("mcporter result did not include artifact")
        || detail.contains("response body read failed")
}

fn should_split_failed_batch(detail: &str) -> bool {
    detail.contains("length limit exceeded")
        || detail.contains("exit status: 124")
        || detail.contains("signal: 9")
        || detail.contains("mcporter output was not JSON")
        || detail.contains("mcporter result did not include artifact")
}

fn invoke_chunk(base: &str, svc: &str, calls: &[PreparedCall]) -> Result<Vec<Value>> {
    let code = batch_code(svc, calls)?;
    let payload = serde_json::to_string(&json!({ "code": code }))?;
    let url = format!("{base}/mcp");
    let output = mcporter_output(&[
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
    ])?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "mcporter exited with {}; stderr: {}; stdout: {}",
            output.status,
            preview(stderr.trim()),
            preview(stdout.trim())
        );
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
    eprintln!("mcporter artifact read: {}", full_path.display());
    let raw = std::fs::read_to_string(&full_path)
        .with_context(|| format!("read mcporter result artifact {}", full_path.display()))?;
    eprintln!("mcporter artifact parse: {}", full_path.display());
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
        let report_args =
            serde_json::to_string(&redact_sensitive_value(&Value::Object(call.args.clone())))?;
        if matches!(svc, "prowlarr" | "sonarr") && is_tag_resource_op(call.op.name) {
            code.push_str(&format!(
                "  try {{ const label = {:?} + '-' + Date.now(); const seed = await {}.post_tag({{ body: {{ label }} }}); const callArgs = {}; callArgs.id = seed.id; if (callArgs.body && typeof callArgs.body === 'object' && !Array.isArray(callArgs.body)) {{ callArgs.body.id = seed.id; callArgs.body.label = label + '-updated'; }} out.push({{ name: {:?}, args: callArgs, ok: true, value: await {}.{}(callArgs) }}); }} catch (e) {{ out.push({{ name: {:?}, args: {}, ok: false, error: String(e) }}); }}\n",
                format!("yarr-live-{}", call.op.name.replace('_', "-")),
                svc,
                args,
                call.op.name,
                svc,
                call.op.name,
                call.op.name,
                report_args
            ));
        } else {
            code.push_str(&format!(
                "  try {{ out.push({{ name: {:?}, args: {}, ok: true, value: await {}.{}({}) }}); }} catch (e) {{ out.push({{ name: {:?}, args: {}, ok: false, error: String(e) }}); }}\n",
                call.op.name, report_args, svc, call.op.name, args, call.op.name, report_args
            ));
        }
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

fn is_tag_resource_op(name: &str) -> bool {
    matches!(
        name,
        "get_tag_by_id" | "get_tag_detail_by_id" | "put_tag_by_id" | "delete_tag_by_id"
    )
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
        args: Some(redact_sensitive_value(&Value::Object(args.clone()))),
    }
}

fn redact_sensitive_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    if is_sensitive_key(key) {
                        (key.clone(), Value::String("<redacted>".into()))
                    } else {
                        (key.clone(), redact_sensitive_value(value))
                    }
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(redact_sensitive_value).collect()),
        other => other.clone(),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("apikey")
        || key.contains("api_key")
        || key.contains("token")
        || key.contains("password")
        || key.contains("secret")
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

pub(super) fn preview(raw: &str) -> String {
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
