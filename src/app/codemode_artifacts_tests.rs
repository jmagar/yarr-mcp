use crate::testing::loopback_state;

#[tokio::test]
async fn persists_file_and_returns_receipt() {
    let tmp = tempfile::tempdir().unwrap();
    let service = loopback_state()
        .service
        .with_data_dir(tmp.path().to_path_buf());
    let out = service.codemode(r#"async () => await writeArtifact("out/report.json", JSON.stringify({ hello: "world" }), { contentType: "application/json" })"#).await.unwrap();
    assert_eq!(out["artifacts"][0]["ok"], true);
    assert_eq!(out["result"]["contentType"], "application/json");
    let run = out["artifactsRunId"].as_str().unwrap();
    let written = tmp
        .path()
        .join("codemode/artifacts")
        .join(run)
        .join("out/report.json");
    assert!(written.exists());
    assert!(std::fs::read_to_string(written).unwrap().contains("world"));
}

#[tokio::test]
async fn partial_failure_does_not_drop_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let service = loopback_state()
        .service
        .with_data_dir(tmp.path().to_path_buf());
    let out = service.codemode(r#"async () => { const a = await writeArtifact("a.txt", "AAA"); let escaped; try { await writeArtifact("../escape.txt", "x"); } catch (e) { escaped = e.message; } const b = await writeArtifact("b.txt", "BBB"); return { a: a.path, escaped, b: b.path }; }"#).await.unwrap();
    assert_eq!(out["result"]["a"], "a.txt");
    assert_eq!(out["result"]["b"], "b.txt");
    assert!(out["result"]["escaped"].as_str().unwrap().contains(".."));
    let artifacts = out["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 3);
    assert_eq!(artifacts[0]["ok"], true);
    assert_eq!(artifacts[1]["ok"], false);
    assert_eq!(artifacts[2]["ok"], true);
}

#[tokio::test]
async fn concurrent_runs_get_isolated_directories() {
    let tmp = tempfile::tempdir().unwrap();
    let first = loopback_state()
        .service
        .with_data_dir(tmp.path().to_path_buf());
    let second = first.clone();
    let code = r#"async () => await writeArtifact("out.txt", "hi")"#;
    let (first, second) = tokio::join!(first.codemode(code), second.codemode(code));
    let first = first.unwrap();
    let second = second.unwrap();
    let first_id = first["artifactsRunId"].as_str().unwrap();
    let second_id = second["artifactsRunId"].as_str().unwrap();
    assert_ne!(first_id, second_id);
    let root = tmp.path().join("codemode/artifacts");
    assert!(root.join(first_id).join("out.txt").exists());
    assert!(root.join(second_id).join("out.txt").exists());
}

#[tokio::test]
async fn write_is_disabled_without_a_data_root() {
    let out = loopback_state().service.codemode(r#"async () => { try { await writeArtifact("x.txt", "y"); return "ran"; } catch (e) { return "blocked:" + e.message; } }"#).await.unwrap();
    assert!(out["result"].as_str().unwrap().contains("unavailable"));
}

#[tokio::test]
async fn enforces_aggregate_bytes_per_run() {
    let tmp = tempfile::tempdir().unwrap();
    let service = loopback_state()
        .service
        .with_data_dir(tmp.path().to_path_buf());
    let out = service.codemode(r#"async () => { const chunk = "x".repeat(8 * 1024 * 1024); await writeArtifact("one.bin", chunk); await writeArtifact("two.bin", chunk); try { await writeArtifact("three.bin", "x"); } catch (error) { return error.message; } }"#).await.unwrap();
    assert!(out["result"].as_str().unwrap().contains("aggregate"));
}
