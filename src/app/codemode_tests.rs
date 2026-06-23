//! Code Mode app-bridge tests — exercise the full async dispatch bridge through a
//! stub `RustarrService` (no real upstreams). `help` is a local, non-networked
//! action, so it round-trips end to end; the destructive-delete refusal exercises
//! the per-service callable path entirely offline.

use crate::testing::{ENV_LOCK, loopback_state};

/// Run a Code Mode script to completion on a fresh current-thread runtime.
/// Synchronous so a caller can hold [`ENV_LOCK`] across the run: the destructive
/// gate reads `RUSTARR_ALLOW_DESTRUCTIVE`, and a sync test body lets us serialise
/// that env mutation without tripping clippy's `await_holding_lock`.
fn run_codemode(service: &crate::app::RustarrService, code: &str) -> serde_json::Value {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(service.codemode(code))
        .unwrap()
}

/// Build a stub `RustarrService` configured with the given kinds (no real
/// upstreams) so a test can exercise multi-service discovery (e.g. ambiguous bare
/// type names across two configured services).
fn multi_service(kinds: &[(&str, crate::config::ServiceKind)]) -> crate::app::RustarrService {
    let config = crate::config::RustarrConfig {
        services: kinds
            .iter()
            .map(|(name, kind)| crate::config::ServiceConfig {
                name: (*name).to_string(),
                kind: *kind,
                base_url: "http://localhost:1".into(),
                api_key: Some("test".into()),
                ..Default::default()
            })
            .collect(),
    };
    let client = crate::rustarr::RustarrClient::new(&config).expect("stub client builds");
    crate::app::RustarrService::new(client, config)
}

#[tokio::test]
async fn codemode_roundtrips_a_local_action() {
    let service = loopback_state().service;
    let code = r#"
        async () => {
            const h = await callTool("help", {});
            return { hasHelp: typeof h.help === "string" };
        }
    "#;
    let out = service.codemode(code).await.unwrap();
    assert_eq!(out["result"]["hasHelp"], true);
    // One recorded call, succeeded.
    assert_eq!(out["calls"].as_array().unwrap().len(), 1);
    assert_eq!(out["calls"][0]["action"], "help");
    assert_eq!(out["calls"][0]["ok"], true);
}

// The destructive-gate tests (these three refusals + the positive override below)
// all read/serialise `RUSTARR_ALLOW_DESTRUCTIVE` via ENV_LOCK and set the env to the
// state they require, so they're deterministic regardless of run order and can't
// race each other. They're sync (`run_codemode`) so the lock isn't held across await.
#[test]
fn per_service_callable_bakes_in_the_service() {
    // The loopback stub configures a `sonarr` (spec-backed) service, so its
    // generated callables exist. `sonarr.delete_series_by_id({id})` is a generated
    // DELETE op: it dispatches through the `op` action with the service baked in,
    // and is refused mid-script before any network call (DELETE = destructive) — a
    // clean, offline assertion of the generated per-service callable path.
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::remove_var("RUSTARR_ALLOW_DESTRUCTIVE") };
    let service = loopback_state().service;
    let code = r#"
        async () => {
            try { await sonarr.delete_series_by_id({ id: 1 }); return "ran"; }
            catch (e) { return "blocked:" + e.message; }
        }
    "#;
    let out = run_codemode(&service, code);
    let result = out["result"].as_str().unwrap();
    assert!(result.starts_with("blocked:"), "got: {result}");
    assert!(
        result.contains("DELETE") || result.contains("destructive"),
        "got: {result}"
    );
    assert_eq!(out["calls"][0]["action"], "op");
}

#[test]
fn codemode_refuses_destructive_actions() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::remove_var("RUSTARR_ALLOW_DESTRUCTIVE") };
    let service = loopback_state().service;
    // api_delete is destructive; even with confirm=true it must be refused inside
    // codemode (no confirmation channel mid-script). The JS catches the throw.
    let code = r#"
        async () => {
            try {
                await callTool("api_delete", { service: "sonarr", path: "/api/v3/series/1", confirm: true });
                return "ran";
            } catch (e) {
                return "blocked:" + e.message;
            }
        }
    "#;
    let out = run_codemode(&service, code);
    let result = out["result"].as_str().unwrap();
    assert!(result.starts_with("blocked:"), "got: {result}");
    assert!(result.contains("destructive"), "got: {result}");
}

#[test]
fn codemode_allows_destructive_op_when_override_set() {
    // RUSTARR_ALLOW_DESTRUCTIVE lifts the mid-script destructive gate (the
    // trusted-test-stack override the contract harness uses). With it ON, the DELETE
    // op is NOT refused for being destructive — it proceeds to dispatch and fails
    // only at the network (the stub points at localhost:1). This is the positive
    // counterpart to the refusal tests above; a regression that dropped the override
    // check (or inverted the gate) would turn this red.
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("RUSTARR_ALLOW_DESTRUCTIVE", "true") };
    let service = loopback_state().service;
    let code = r#"
        async () => {
            try { await sonarr.delete_series_by_id({ id: 1 }); return "ran"; }
            catch (e) { return "err:" + e.message; }
        }
    "#;
    let out = run_codemode(&service, code);
    unsafe { std::env::remove_var("RUSTARR_ALLOW_DESTRUCTIVE") };

    let result = out["result"].as_str().unwrap();
    // Gate lifted → never blocked for being a DELETE (it either "ran" or hit a
    // network error, but the destructive-refusal message must be absent).
    assert!(!result.contains("destructive"), "gate not lifted: {result}");
    assert!(!result.contains("cannot run"), "gate not lifted: {result}");
    assert_eq!(out["calls"][0]["action"], "op");
}

#[tokio::test]
async fn codemode_discovery_search_and_describe_run() {
    // Exercise the injected discovery JS end-to-end (a .contains() string check
    // would not catch a syntax error in the preamble — this actually runs it).
    let service = loopback_state().service;
    let code = r#"
        async () => {
            const hits = codemode.search("api");
            const desc = codemode.describe("api.<service>.delete");
            return {
                found: hits.results.some(e => e.path === "api.<service>.get"),
                total: hits.total,
                describedDestructive: desc.destructive,
                signature: desc.signature,
                missing: codemode.describe("nope_not_real"),
            };
        }
    "#;
    let out = service.codemode(code).await.unwrap();
    assert_eq!(out["result"]["found"], true);
    assert!(out["result"]["total"].as_i64().unwrap() >= 4);
    assert_eq!(out["result"]["describedDestructive"], true);
    assert_eq!(out["result"]["signature"], "api.<service>.delete(path)");
    assert!(out["result"]["missing"].is_null());
}

#[tokio::test]
async fn codemode_describe_surfaces_response_types_on_demand() {
    // The whole point: an agent discovers a response TYPE's TS interface ON DEMAND
    // via codemode.describe — only the type it asks for comes back (not a context
    // dump). End-to-end through the engine.
    // Only `sonarr` is configured, so its generated types are the surface.
    let service = loopback_state().service;
    let code = r#"
        async () => {
            const byQualified = codemode.describe("sonarr.SeriesResource");
            const byBare = codemode.describe("SeriesResource");
            const found = codemode.search("series").results.some(r => r.kind === "type");
            return {
                kind: byQualified.kind,
                hasInterface: byQualified.dts.indexOf("export interface SeriesResource") !== -1,
                hasOptionalField: byQualified.dts.indexOf("?:") !== -1,
                bareResolved: byBare && byBare.name,
                searchFindsType: found,
            };
        }
    "#;
    let out = service.codemode(code).await.unwrap();
    assert_eq!(out["result"]["kind"], "type");
    assert_eq!(out["result"]["hasInterface"], true);
    assert_eq!(out["result"]["hasOptionalField"], true);
    // Bare name is unambiguous (only sonarr configured) → resolves to the qualified.
    assert_eq!(out["result"]["bareResolved"], "sonarr.SeriesResource");
    assert_eq!(out["result"]["searchFindsType"], true);
}

#[tokio::test]
async fn codemode_describe_ambiguous_bare_type_is_null() {
    // QualityProfileResource exists under both sonarr and radarr; a bare name must
    // NOT silently resolve to the first match — only an unambiguous name resolves.
    let service = multi_service(&[
        ("sonarr", crate::config::ServiceKind::Sonarr),
        ("radarr", crate::config::ServiceKind::Radarr),
    ]);
    let code = r#"async () => ({
        ambiguous: codemode.describe("QualityProfileResource"),
        qualified: codemode.describe("sonarr.QualityProfileResource") ? "ok" : "missing",
        unique: codemode.describe("SeriesResource") ? "ok" : "missing",
    })"#;
    let out = service.codemode(code).await.unwrap();
    assert!(out["result"]["ambiguous"].is_null());
    assert_eq!(out["result"]["qualified"], "ok");
    assert_eq!(out["result"]["unique"], "ok");
}

#[tokio::test]
async fn snippet_input_binding_is_injection_safe() {
    // The `input` binding's central safety claim: a snippet input is DATA, never
    // source. Round-trip a payload full of quotes/backslashes/JS/unicode and assert
    // it arrives byte-identical (if it were spliced into source, these would break
    // out or corrupt parsing).
    let tmp = tempfile::tempdir().unwrap();
    let service = loopback_state()
        .service
        .with_data_dir(tmp.path().to_path_buf());
    service
        .snippet_save("echo", "async () => input", None)
        .await
        .unwrap();
    let tricky = serde_json::json!({
        "quote": "he said \"hi\" and \\ ; return 1; //",
        // `\u{..}` escapes (ASCII source, same runtime bytes) so the repo ASCII
        // check stays clean while still round-tripping accented latin, an emoji,
        // the JS line/paragraph separators, and a NUL through the input binding.
        "unicode": "h\u{e9}llo \u{1f389} \u{2028}\u{2029} \u{0}end",
        "nested": { "js": "\"); maliciousCode(); //", "n": 42 },
        "arr": [1, "two", null, true],
    });
    let out = service.snippet_run("echo", &tricky).await.unwrap();
    assert_eq!(out["result"], tricky, "input must arrive byte-identical");
}

#[test]
fn codemode_api_client_delete_is_refused() {
    // The loopback stub configures a `sonarr` service, so `api.sonarr` exists in
    // the preamble. `.delete` resolves to the destructive `api_delete`, which is
    // refused mid-script before any network call — a clean, offline assertion.
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::remove_var("RUSTARR_ALLOW_DESTRUCTIVE") };
    let service = loopback_state().service;
    let code = r#"
        async () => {
            try {
                await api.sonarr.delete("/api/v3/series/1");
                return "ran";
            } catch (e) {
                return "blocked:" + e.message;
            }
        }
    "#;
    let out = run_codemode(&service, code);
    let result = out["result"].as_str().unwrap();
    assert!(result.starts_with("blocked:"), "got: {result}");
    assert!(result.contains("destructive"), "got: {result}");
    assert_eq!(out["calls"][0]["action"], "api_delete");
}

#[tokio::test]
async fn codemode_write_artifact_persists_file_and_returns_receipt() {
    let tmp = tempfile::tempdir().unwrap();
    let service = loopback_state()
        .service
        .with_data_dir(tmp.path().to_path_buf());
    let code = r#"
        async () => await writeArtifact(
            "out/report.json",
            JSON.stringify({ hello: "world" }),
            { contentType: "application/json" }
        )
    "#;
    let out = service.codemode(code).await.unwrap();

    assert_eq!(out["artifacts"][0]["ok"], true);
    assert_eq!(out["artifacts"][0]["path"], "out/report.json");
    assert_eq!(out["result"]["contentType"], "application/json");
    let run_id = out["artifactsRunId"].as_str().expect("run id present");

    let written = tmp
        .path()
        .join("codemode/artifacts")
        .join(run_id)
        .join("out/report.json");
    assert!(written.exists(), "artifact file should exist on disk");
    assert!(std::fs::read_to_string(&written).unwrap().contains("world"));
}

#[tokio::test]
async fn codemode_write_artifact_partial_failure_does_not_drop_writes() {
    // Exercises the dual-channel drain loop: a refused `../escape` write between two
    // good writes must NOT terminate the loop early or drop either good write.
    let tmp = tempfile::tempdir().unwrap();
    let service = loopback_state()
        .service
        .with_data_dir(tmp.path().to_path_buf());
    let code = r#"
        async () => {
            const a = await writeArtifact("a.txt", "AAA");
            let escaped = null;
            try { await writeArtifact("../escape.txt", "x"); }
            catch (e) { escaped = e.message; }
            const b = await writeArtifact("b.txt", "BBB");
            return { a: a.path, escaped, b: b.path };
        }
    "#;
    let out = service.codemode(code).await.unwrap();
    assert_eq!(out["result"]["a"], "a.txt");
    assert_eq!(out["result"]["b"], "b.txt");
    assert!(
        out["result"]["escaped"].as_str().unwrap().contains(".."),
        "escape write should be refused with a `..` error"
    );

    // Three recorded attempts: two ok, one refused (order preserved).
    let arts = out["artifacts"].as_array().unwrap();
    assert_eq!(arts.len(), 3);
    assert_eq!(arts[0]["ok"], true);
    assert_eq!(arts[1]["ok"], false);
    assert_eq!(arts[2]["ok"], true);

    let run_id = out["artifactsRunId"].as_str().unwrap();
    let base = tmp.path().join("codemode/artifacts").join(run_id);
    assert!(base.join("a.txt").exists());
    assert!(base.join("b.txt").exists());
}

#[tokio::test]
async fn codemode_concurrent_runs_get_isolated_artifact_dirs() {
    // Two CONCURRENT codemode runs sharing one data dir must never collide into the
    // same per-run artifacts dir — even if they land on the same nanosecond, the
    // monotonic run-seq keeps their run-ids distinct.
    let shared = tempfile::tempdir().unwrap();
    let service_a = loopback_state()
        .service
        .with_data_dir(shared.path().to_path_buf());
    let service_b = service_a.clone();

    let code = r#"async () => await writeArtifact("out.txt", "hi")"#;
    let (out_a, out_b) = tokio::join!(service_a.codemode(code), service_b.codemode(code));
    let out_a = out_a.unwrap();
    let out_b = out_b.unwrap();

    // Both writes succeeded.
    assert_eq!(out_a["artifacts"][0]["ok"], true);
    assert_eq!(out_b["artifacts"][0]["ok"], true);

    // Distinct run-ids → distinct artifacts dirs.
    let run_a = out_a["artifactsRunId"].as_str().expect("run id A present");
    let run_b = out_b["artifactsRunId"].as_str().expect("run id B present");
    assert_ne!(run_a, run_b, "concurrent runs must get distinct run-ids");

    // Each run's file lives under its own dir (no collision).
    let base = shared.path().join("codemode/artifacts");
    assert!(base.join(run_a).join("out.txt").exists());
    assert!(base.join(run_b).join("out.txt").exists());
}

#[tokio::test]
async fn codemode_write_artifact_disabled_without_root() {
    // loopback service has no artifacts root → writeArtifact throws.
    let service = loopback_state().service;
    let code = r#"
        async () => {
            try { await writeArtifact("x.txt", "y"); return "ran"; }
            catch (e) { return "blocked:" + e.message; }
        }
    "#;
    let out = service.codemode(code).await.unwrap();
    let result = out["result"].as_str().unwrap();
    assert!(result.starts_with("blocked:"), "got: {result}");
    assert!(result.contains("unavailable"), "got: {result}");
}

#[tokio::test]
async fn snippet_save_list_run_delete_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let service = loopback_state()
        .service
        .with_data_dir(tmp.path().to_path_buf());

    service
        .snippet_save("greet", "async () => ({ hi: input.who })", Some("greets"))
        .await
        .unwrap();

    let listed = service.snippet_list().await.unwrap();
    let snippets = listed["snippets"].as_array().unwrap();
    assert_eq!(snippets.len(), 1);
    assert_eq!(snippets[0]["name"], "greet");

    // snippet_run binds `input` as globalThis.input and returns the run envelope.
    let run = service
        .snippet_run("greet", &serde_json::json!({ "who": "world" }))
        .await
        .unwrap();
    assert_eq!(run["result"]["hi"], "world");

    assert_eq!(
        service.snippet_delete("greet").await.unwrap()["deleted"],
        true
    );
    assert!(
        service.snippet_list().await.unwrap()["snippets"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn codemode_run_invokes_saved_snippet() {
    let tmp = tempfile::tempdir().unwrap();
    let service = loopback_state()
        .service
        .with_data_dir(tmp.path().to_path_buf());
    service
        .snippet_save("double", "async () => input.n * 2", None)
        .await
        .unwrap();

    // A top-level script calls the saved snippet via codemode.run(name, input).
    let code = r#"async () => {
        const r = await codemode.run("double", { n: 21 });
        return r.result;
    }"#;
    let out = service.codemode(code).await.unwrap();
    assert_eq!(out["result"], 42);
}

#[tokio::test]
async fn snippet_cannot_run_another_snippet() {
    let tmp = tempfile::tempdir().unwrap();
    let service = loopback_state()
        .service
        .with_data_dir(tmp.path().to_path_buf());
    service
        .snippet_save("inner", "async () => 1", None)
        .await
        .unwrap();
    service
        .snippet_save(
            "outer",
            r#"async () => {
                try { await codemode.run("inner", {}); return "ran"; }
                catch (e) { return "blocked:" + e.message; }
            }"#,
            None,
        )
        .await
        .unwrap();

    let out = service
        .codemode(r#"async () => (await codemode.run("outer", {})).result"#)
        .await
        .unwrap();
    let result = out["result"].as_str().unwrap();
    assert!(result.starts_with("blocked:"), "got: {result}");
    assert!(result.contains("snippet"), "got: {result}");
}

#[tokio::test]
async fn snippets_disabled_without_data_dir() {
    let service = loopback_state().service; // no data dir
    assert!(
        service
            .snippet_save("x", "async () => 1", None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn codemode_rejects_empty_code() {
    let service = loopback_state().service;
    assert!(service.codemode("   ").await.is_err());
}

#[tokio::test]
async fn codemode_self_invocation_is_blocked() {
    let service = loopback_state().service;
    let code = r#"
        async () => {
            try { await callTool("codemode", { code: "async () => 1" }); return "ran"; }
            catch (e) { return "blocked:" + e.message; }
        }
    "#;
    let out = service.codemode(code).await.unwrap();
    assert!(out["result"].as_str().unwrap().starts_with("blocked:"));
}

#[tokio::test]
async fn codemode_records_per_call_elapsed_ms() {
    // Every recorded call carries an elapsed_ms timing (>= 0).
    let service = loopback_state().service;
    let out = service
        .codemode(r#"async () => { await callTool("help", {}); return 1; }"#)
        .await
        .unwrap();
    let call = &out["calls"][0];
    assert_eq!(call["action"], "help");
    assert!(
        call["elapsed_ms"].is_u64(),
        "elapsed_ms must be recorded: {call}"
    );
}

#[tokio::test]
async fn codemode_budgets_an_oversized_result_into_a_parseable_marker() {
    // A script that returns a large array (no network) must come back as a parseable
    // envelope whose result is a structured truncation marker — not a blind cut.
    let service = loopback_state().service;
    let out = service
        .codemode(r#"async () => Array.from({length: 50000}, (_, i) => ({ i, s: "row-" + i }))"#)
        .await
        .unwrap();
    // The whole envelope is valid JSON and fits below the transport cap.
    let serialized = serde_json::to_string(&out).unwrap();
    assert!(
        serialized.len() < rustarr_max_response_bytes(),
        "shaped envelope ({}) must stay below the transport cap",
        serialized.len()
    );
    assert_eq!(out["result"]["truncated"], true);
    assert!(out["result"]["preview"].as_str().unwrap().len() <= 1024);
    assert!(out["result"]["original_bytes"].as_u64().unwrap() > 0);
}

/// Mirror of token_limit::MAX_RESPONSE_BYTES for the assertion above (the const is
/// crate-private to that module; this keeps the test independent of its path).
fn rustarr_max_response_bytes() -> usize {
    40_000
}
