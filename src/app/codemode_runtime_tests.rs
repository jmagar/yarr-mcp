use crate::testing::loopback_state;

#[tokio::test]
async fn rejects_queue_overload_at_configured_capacity() {
    let service = loopback_state().service.with_codemode_limits(
        1,
        std::time::Duration::from_millis(20),
        std::time::Duration::from_millis(180),
    );
    let first_service = service.clone();
    let first = tokio::spawn(async move {
        first_service
            .codemode("async () => { while (true) {} }")
            .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let error = service
        .codemode("async () => 1")
        .await
        .expect_err("queued run must be rejected");
    assert!(error.to_string().contains("busy"));
    let _ = first.await;
}

#[tokio::test]
async fn rejects_empty_code_and_self_invocation() {
    let service = loopback_state().service;
    assert!(service.codemode("   ").await.is_err());
    let out = service.codemode(r#"async () => { try { await callTool("codemode", { code: "async () => 1" }); return "ran"; } catch (e) { return "blocked:" + e.message; } }"#).await.unwrap();
    assert!(out["result"].as_str().unwrap().starts_with("blocked:"));
}

#[tokio::test]
async fn records_per_call_elapsed_ms() {
    let out = loopback_state()
        .service
        .codemode(r#"async () => { await callTool("help", {}); return 1; }"#)
        .await
        .unwrap();
    assert_eq!(out["calls"][0]["action"], "help");
    assert!(out["calls"][0]["elapsed_ms"].is_u64());
}

#[tokio::test]
async fn oversized_result_becomes_a_parseable_marker() {
    let out = loopback_state()
        .service
        .codemode(r#"async () => Array.from({length: 50000}, (_, i) => ({ i, s: "row-" + i }))"#)
        .await
        .unwrap();
    let serialized = serde_json::to_string(&out).unwrap();
    assert!(serialized.len() < 40_000);
    assert_eq!(out["result"]["truncated"], true);
    assert!(out["result"]["preview"].as_str().unwrap().len() <= 1024);
    assert!(out["result"]["original_bytes"].as_u64().unwrap() > 0);
}
