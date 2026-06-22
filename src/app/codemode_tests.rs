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
