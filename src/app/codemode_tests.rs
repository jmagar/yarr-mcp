//! Code Mode app-bridge tests — exercise the full async dispatch bridge through a
//! stub `RustarrService` (no real upstreams). `help` is a local, non-networked
//! action, so it round-trips end to end; the destructive-delete refusal exercises
//! the per-service callable path entirely offline.

use crate::testing::loopback_state;

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

#[tokio::test]
async fn per_service_callable_bakes_in_the_service() {
    // The loopback stub configures a `sonarr` service, so `sonarr.<verb>()` exists.
    // `sonarr.delete({id})` is a curated DESTRUCTIVE delete: it requires `service`
    // (proving the namespace baked it in) and is then refused mid-script before any
    // network call — a clean, offline assertion of the per-service callable path.
    let service = loopback_state().service;
    let code = r#"
        async () => {
            try { await sonarr.delete({ id: 1 }); return "ran"; }
            catch (e) { return "blocked:" + e.message; }
        }
    "#;
    let out = service.codemode(code).await.unwrap();
    let result = out["result"].as_str().unwrap();
    // Refused as destructive — NOT a "service is required" parse error, which proves
    // the service was baked into the callable.
    assert!(result.starts_with("blocked:"), "got: {result}");
    assert!(result.contains("destructive"), "got: {result}");
    assert_eq!(out["calls"][0]["action"], "delete");
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
    let service = loopback_state().service;
    let code = r#"
        async () => {
            const byQualified = codemode.describe("sonarr.SeriesResource");
            const byBare = codemode.describe("TorrentInfo");
            const found = codemode.search("torrent").results.some(r => r.kind === "type");
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
    assert_eq!(out["result"]["bareResolved"], "qbittorrent.TorrentInfo");
    assert_eq!(out["result"]["searchFindsType"], true);
}

#[tokio::test]
async fn codemode_describe_ambiguous_bare_type_is_null() {
    // QualityProfileResource exists under both sonarr and radarr; a bare name must
    // NOT silently resolve to the first match — only an unambiguous name resolves.
    let service = loopback_state().service;
    let code = r#"async () => ({
        ambiguous: codemode.describe("QualityProfileResource"),
        qualified: codemode.describe("sonarr.QualityProfileResource") ? "ok" : "missing",
        unique: codemode.describe("TorrentInfo") ? "ok" : "missing",
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
        "unicode": "héllo 🎉 \u{2028}\u{2029} \u{0}end",
        "nested": { "js": "\"); maliciousCode(); //", "n": 42 },
        "arr": [1, "two", null, true],
    });
    let out = service.snippet_run("echo", &tricky).await.unwrap();
    assert_eq!(out["result"], tricky, "input must arrive byte-identical");
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
