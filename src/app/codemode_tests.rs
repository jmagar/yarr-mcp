//! Code Mode app-bridge tests — exercise the full async dispatch bridge through a
//! stub `RustarrService` (no real upstreams). `integrations` is a local,
//! non-networked action, so it round-trips end to end.

use crate::testing::loopback_state;

#[tokio::test]
async fn codemode_calls_integrations_and_returns_result() {
    let service = loopback_state().service;
    let code = r#"
        async () => {
            const info = await callTool("integrations", {});
            return { kinds: info.supported.length };
        }
    "#;
    let out = service.codemode(code).await.unwrap();
    // 11 supported ServiceKinds.
    assert_eq!(out["result"]["kinds"], 11);
    // One recorded call, succeeded.
    assert_eq!(out["calls"].as_array().unwrap().len(), 1);
    assert_eq!(out["calls"][0]["action"], "integrations");
    assert_eq!(out["calls"][0]["ok"], true);
}

#[tokio::test]
async fn codemode_tools_namespace_works() {
    let service = loopback_state().service;
    let code = r#"async () => (await tools.integrations({})).supported.map(s => s.kind)"#;
    let out = service.codemode(code).await.unwrap();
    let kinds = out["result"].as_array().unwrap();
    assert!(kinds.iter().any(|k| k == "sonarr"));
}

#[tokio::test]
async fn codemode_refuses_destructive_actions() {
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
    let out = service.codemode(code).await.unwrap();
    let result = out["result"].as_str().unwrap();
    assert!(result.starts_with("blocked:"), "got: {result}");
    assert!(result.contains("destructive"), "got: {result}");
}

#[tokio::test]
async fn codemode_discovery_search_and_describe_run() {
    // Exercise the injected discovery JS end-to-end (a .contains() string check
    // would not catch a syntax error in the preamble — this actually runs it).
    let service = loopback_state().service;
    let code = r#"
        async () => {
            const hits = codemode.search("api");
            const desc = codemode.describe("api_delete");
            return {
                found: hits.results.some(e => e.name === "api_get"),
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
    assert_eq!(out["result"]["signature"], "api_delete(path)");
    assert!(out["result"]["missing"].is_null());
}

#[tokio::test]
async fn codemode_api_client_delete_is_refused() {
    // The loopback stub configures a `sonarr` service, so `api.sonarr` exists in
    // the preamble. `.delete` resolves to the destructive `api_delete`, which is
    // refused mid-script before any network call — a clean, offline assertion.
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
    let out = service.codemode(code).await.unwrap();
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
        .with_artifacts_root(tmp.path().to_path_buf());
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
        .with_artifacts_root(tmp.path().to_path_buf());
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
