//! Engine harness tests — drive `run` directly with a mock tool caller (no tokio,
//! no real services).

use std::time::{Duration, Instant};

use super::{EngineLimits, ToolCaller, run};
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

#[test]
fn plain_expression_returns_value() {
    let out = run(
        "async () => 6 * 7",
        &build_preamble(&[]),
        &limits(Duration::from_secs(5)),
        echo_caller(),
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
    )
    .unwrap_err();
    assert!(err.contains("nope"), "got: {err}");
}

#[test]
fn infinite_loop_is_interrupted_by_deadline() {
    let code = r#"async () => { while (true) {} }"#;
    let err = run(
        code,
        &build_preamble(&[]),
        &limits(Duration::from_millis(300)),
        echo_caller(),
    )
    .unwrap_err();
    // Either the timeout guard or the interrupted job surfaces — both are errors.
    assert!(!err.is_empty(), "expected an error for a runaway loop");
}
