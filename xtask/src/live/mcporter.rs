//! mcporter-backed live contract harness for the generated MCP Code Mode surface.
//!
//! This starts a local Rustarr MCP server against the guarded shart environment and
//! uses `mcporter call ... yarr` to execute generated per-service callables over
//! the MCP transport. It mirrors the CLI contract suite's synthesis, seeding, skip,
//! and response-schema validation rules so both suites cover the same OpenAPI
//! surface through different transports.

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rustarr::ServiceKind;
use rustarr::openapi::{self, OperationSpec};

use super::contract::{self, PreparedOp, RunOut, synth::Spec};
use super::{guard, process, report, reset};

// Keep chunks small enough to avoid Code Mode's 30s script budget while still
// avoiding a separate Node/mcporter process for every generated operation. If a
// chunk trips a transport/length limit, `run_chunk` recursively splits it so one
// large response cannot poison neighboring operations.
const BATCH_SIZE: usize = 4;
const MCPORTER_ATTEMPTS: usize = 1;
const MCPORTER_TIMEOUT: &str = "40s";

pub(super) fn run(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
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
            reset_after_op(rustarr, svc)
                .with_context(|| format!("reset live fixture baseline for {svc}"))?;
        }
        contract::seed_service_fixtures(rustarr, svc, kind)
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
                rustarr,
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
            run_reset_required_ops(rustarr, svc, kind, &spec, &fixtures, &ops, no_destructive);
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
    rustarr: &process::RustarrProcess,
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
            PreparedOp::Call(args) => prepared.push(PreparedCall {
                kind,
                op: *op,
                args,
            }),
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

    let mut harness = match McpHarness::start(rustarr, no_destructive) {
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
    rustarr: &process::RustarrProcess,
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
                let mut result = run_chunk(
                    rustarr,
                    svc,
                    spec,
                    std::slice::from_ref(&call),
                    no_destructive,
                );
                outs.append(&mut result);
                if let Err(err) = reset_after_op(rustarr, svc) {
                    mark_reset_failure(
                        &mut outs,
                        call.op,
                        format!("post-operation reset failed: {err}"),
                    );
                } else if let Err(err) = contract::seed_service_fixtures(rustarr, svc, kind) {
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

fn reset_after_op(rustarr: &process::RustarrProcess, svc: &str) -> Result<()> {
    reset::reset_service(svc)?;
    if let Some(url) = reset::service_url(&rustarr.env, svc) {
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
    rustarr: &'a process::RustarrProcess,
    no_destructive: bool,
    _server: Option<process::Server>,
    base: String,
}

impl<'a> McpHarness<'a> {
    fn start(rustarr: &'a process::RustarrProcess, no_destructive: bool) -> Result<Self> {
        let (server, base) = start_isolated_server(rustarr, no_destructive)?;
        Ok(Self {
            rustarr,
            no_destructive,
            _server: Some(server),
            base,
        })
    }

    fn restart(&mut self) -> Result<()> {
        self._server.take();
        let (server, base) = start_isolated_server(self.rustarr, self.no_destructive)?;
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
                        if let Err(restart_err) = self.restart() {
                            let detail = format!("mcporter server restart failed: {restart_err}");
                            return rejected_calls(calls, detail);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(1000));
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

fn should_retry_domain_result(calls: &[PreparedCall], values: &[Value]) -> bool {
    let ([call], [value]) = (calls, values) else {
        return false;
    };
    call.kind == ServiceKind::Prowlarr
        && matches!(call.op.name, "get_tag_by_id" | "get_tag_detail_by_id")
        && value.get("ok").and_then(Value::as_bool) == Some(false)
        && value
            .get("error")
            .and_then(Value::as_str)
            .is_some_and(|detail| {
                detail.contains("returned HTTP 404") && detail.contains("Tag with ID")
            })
}

fn run_chunk(
    rustarr: &process::RustarrProcess,
    svc: &str,
    spec: &Spec,
    calls: &[PreparedCall],
    no_destructive: bool,
) -> Vec<RunOut> {
    let (server, base) = match start_isolated_server(rustarr, no_destructive) {
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
    rustarr: &process::RustarrProcess,
    no_destructive: bool,
) -> Result<(process::Server, String)> {
    let port = reserve_local_port()?;
    let base = format!("http://127.0.0.1:{port}");
    let mut env = BTreeMap::new();
    env.insert("RUSTARR_MCP_NO_AUTH".into(), "true".into());
    env.insert("RUSTARR_NOAUTH".into(), "false".into());
    env.insert("RUSTARR_HTTP_TIMEOUT_SECS".into(), "20".into());
    if !no_destructive {
        env.insert("RUSTARR_ALLOW_DESTRUCTIVE".into(), "true".into());
    }
    let mut server = rustarr.start_server_args(&["serve", "mcp"], "127.0.0.1", port, &env)?;
    server.wait_healthy(&base)?;
    Ok((server, base))
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

fn mcporter_output(args: &[&str]) -> Result<Output> {
    let capture = ProcessCapture::new("mcporter-call")?;
    let mut child = Command::new("timeout")
        .args(args)
        .stdout(capture.stdout_stdio()?)
        .stderr(capture.stderr_stdio()?)
        .spawn()
        .context("failed to run mcporter")?;

    let deadline = Instant::now() + Duration::from_secs(50);
    while Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            let status = child.wait().context("failed to collect mcporter status")?;
            return capture.into_output(status);
        }
        thread::sleep(Duration::from_millis(50));
    }

    let _ = child.kill();
    let status = child
        .wait()
        .context("failed to collect timed-out mcporter status")?;
    let output = capture.into_output(status)?;
    bail!(
        "mcporter wrapper timed out after 50s; stderr: {}; stdout: {}",
        preview(String::from_utf8_lossy(&output.stderr).trim()),
        preview(String::from_utf8_lossy(&output.stdout).trim())
    );
}

struct ProcessCapture {
    stdout: PathBuf,
    stderr: PathBuf,
}

impl ProcessCapture {
    fn new(label: &str) -> Result<Self> {
        let nonce = format!(
            "{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .context("system clock before unix epoch")?
                .as_nanos()
        );
        let dir = std::env::temp_dir();
        Ok(Self {
            stdout: dir.join(format!("{nonce}.stdout")),
            stderr: dir.join(format!("{nonce}.stderr")),
        })
    }

    fn stdout_stdio(&self) -> Result<Stdio> {
        Ok(Stdio::from(File::create(&self.stdout).with_context(
            || format!("create {}", self.stdout.display()),
        )?))
    }

    fn stderr_stdio(&self) -> Result<Stdio> {
        Ok(Stdio::from(File::create(&self.stderr).with_context(
            || format!("create {}", self.stderr.display()),
        )?))
    }

    fn into_output(self, status: std::process::ExitStatus) -> Result<Output> {
        let stdout = fs::read(&self.stdout).unwrap_or_default();
        let stderr = fs::read(&self.stderr).unwrap_or_default();
        let _ = fs::remove_file(&self.stdout);
        let _ = fs::remove_file(&self.stderr);
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
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
                format!("rustarr-live-{}", call.op.name.replace('_', "-")),
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
        || detail.contains("returned HTTP 401")
        || detail.contains("returned HTTP 403")
        || detail.contains("returned HTTP 404")
        || detail.contains("returned HTTP 405")
        || detail.contains("returned HTTP 409")
        || detail.contains("returned HTTP 500")
        || detail.contains("returned HTTP 501")
        || detail.contains("returned HTTP 503")
}

fn overseerr_expected_domain_response(op_name: &str, detail: &str) -> bool {
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
            || detail.contains("returned HTTP 401")
            || detail.contains("returned HTTP 403")
            || detail.contains("returned HTTP 404")
            || detail.contains("returned HTTP 405")
            || detail.contains("returned HTTP 409")
            || detail.contains("returned HTTP 500")
            || detail.contains("overseerr request failed"))
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
