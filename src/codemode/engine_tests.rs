//! Engine harness tests — drive `run` directly with a mock tool caller (no tokio,
//! no real services).

use std::time::{Duration, Instant};

use super::{ArtifactWriter, EngineLimits, ToolCaller, run};
use crate::codemode::build_preamble;

fn limits(ttl: Duration) -> EngineLimits {
    EngineLimits {
        memory_bytes: 64 * 1024 * 1024,
        stack_bytes: 512 * 1024,
        deadline: Instant::now() + ttl,
    }
}

/// A mock caller that echoes the action id + params back as a JSON object.
fn echo_caller() -> ToolCaller {
    Box::new(|id: &str, params_json: &str| {
        Ok(format!(r#"{{"echo":"{id}","params":{params_json}}}"#))
    })
}

/// A no-op artifact writer for tests that don't exercise `writeArtifact`.
fn no_write() -> ArtifactWriter {
    Box::new(|_path, _content, _opts| Err("artifacts disabled in this test".to_string()))
}

#[test]
fn plain_expression_returns_value() {
    let out = run(
        "async () => 6 * 7",
        &build_preamble(&[]),
        &limits(Duration::from_secs(5)),
        echo_caller(),
        no_write(),
    )
    .unwrap();
    assert_eq!(out.result, serde_json::json!(42));
}

#[test]
fn calltool_round_trips_through_on_call() {
    let code = r#"
        async () => {
            const a = await callTool("list", { service: "sonarr" });
            const b = await tools.service_status({ service: "radarr" });
            return { first: a.echo, second: b.echo, viaParams: a.params.service };
        }
    "#;
    let out = run(
        code,
        &build_preamble(&[]),
        &limits(Duration::from_secs(5)),
        echo_caller(),
        no_write(),
    )
    .unwrap();
    assert_eq!(out.result["first"], "list");
    assert_eq!(out.result["second"], "service_status");
    assert_eq!(out.result["viaParams"], "sonarr");
}

#[test]
fn console_output_is_captured() {
    let code = r#"async () => { console.log("hello", 1); console.error("boom"); return "ok"; }"#;
    let out = run(
        code,
        &build_preamble(&[]),
        &limits(Duration::from_secs(5)),
        echo_caller(),
        no_write(),
    )
    .unwrap();
    assert_eq!(out.result, serde_json::json!("ok"));
    assert_eq!(
        out.logs,
        vec!["hello 1".to_string(), "ERROR boom".to_string()]
    );
}

#[test]
fn thrown_tool_error_surfaces_as_err() {
    let failing: ToolCaller = Box::new(|_id, _params| Err("upstream exploded".to_string()));
    let code =
        r#"async () => { await callTool("list", { service: "sonarr" }); return "unreachable"; }"#;
    let err = run(
        code,
        &build_preamble(&[]),
        &limits(Duration::from_secs(5)),
        failing,
        no_write(),
    )
    .unwrap_err();
    assert!(err.contains("upstream exploded"), "got: {err}");
}

#[test]
fn script_throw_surfaces_as_err() {
    let code = r#"async () => { throw new Error("nope"); }"#;
    let err = run(
        code,
        &build_preamble(&[]),
        &limits(Duration::from_secs(5)),
        echo_caller(),
        no_write(),
    )
    .unwrap_err();
    assert!(err.contains("nope"), "got: {err}");
}

#[test]
fn write_artifact_round_trips_through_on_write() {
    let code = r#"async () => {
        const r = await writeArtifact("out/report.json", JSON.stringify({ ok: true }), { contentType: "application/json" });
        return r;
    }"#;
    // Capturing writer: echoes a receipt for whatever it's handed.
    let writer: ArtifactWriter = Box::new(|path: &str, content: &str, _opts: &str| {
        Ok(format!(r#"{{"path":"{path}","bytes":{}}}"#, content.len()))
    });
    let out = run(
        code,
        &build_preamble(&[]),
        &limits(Duration::from_secs(5)),
        echo_caller(),
        writer,
    )
    .unwrap();
    assert_eq!(out.result["path"], "out/report.json");
    assert!(out.result["bytes"].as_i64().unwrap() > 0);
}

#[test]
fn write_artifact_error_surfaces_as_throw() {
    let code = r#"async () => {
        try { await writeArtifact("x.txt", "hi"); return "ran"; }
        catch (e) { return "blocked:" + e.message; }
    }"#;
    let writer: ArtifactWriter = Box::new(|_p, _c, _o| Err("disk full".to_string()));
    let out = run(
        code,
        &build_preamble(&[]),
        &limits(Duration::from_secs(5)),
        echo_caller(),
        writer,
    )
    .unwrap();
    assert_eq!(out.result, serde_json::json!("blocked:disk full"));
}

#[test]
fn infinite_loop_is_interrupted_by_deadline() {
    let code = r#"async () => { while (true) {} }"#;
    let err = run(
        code,
        &build_preamble(&[]),
        &limits(Duration::from_millis(300)),
        echo_caller(),
        no_write(),
    )
    .unwrap_err();
    // Either the timeout guard or the interrupted job surfaces — both are errors.
    assert!(!err.is_empty(), "expected an error for a runaway loop");
}
