---
title: "Testing"
doc_type: "guide"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "docs/PATTERNS.md"
last_reviewed: "2026-05-15"
---

# Testing

The test strategy is layered: parse at the CLI layer, test business/service behavior without a server, then run the opt-in shart live suite for real CLI, REST, MCP, and upstream service coverage.

## Rust tests

```bash
cargo nextest run
cargo nextest run --profile ci
cargo test
just test-ci
```

All repos use `cargo nextest` instead of `cargo test`. Configure in `.config/nextest.toml`:

```toml
[profile.default]
fail-fast = false

[profile.ci]
fail-fast = true
retries = 2
```

## Key test files

| File | Purpose |
|---|---|
| `tests/cli_parse.rs` | CLI parser behavior. |
| `tests/tool_dispatch.rs` | Service/action semantics without live credentials. |
| `tests/api_routes.rs` | REST and mounted auth route behavior. |
| `tests/plugin_contract.rs` | Plugin package and hook contracts. |
| `tests/template_invariants.rs` | Automation/template invariants. |
| `src/app_tests.rs` | Private service-layer unit tests (sidecar to `app.rs`). |

## Test sidecars

All tests that need access to private functions live in `_tests.rs` sidecar files, not inline:

```rust
// src/app.rs
pub struct RustarrService { ... }
impl RustarrService { ... }

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

// src/app_tests.rs
use super::*;  // access to private items

#[test]
fn destructive_gate_blocks_without_confirm() {
    let svc = RustarrService::new(stub_client(), false);
    let err = svc.destructive_gate(false).unwrap_err();
    assert!(err.to_string().contains("confirm=true"));
}

#[test]
fn destructive_gate_allows_with_confirm() {
    let svc = RustarrService::new(stub_client(), false);
    assert!(svc.destructive_gate(true).is_ok());
}
```

## Test helpers

`src/lib.rs` exports helpers for integration tests:

```rust
#[cfg(any(test, feature = "test-support"))]
pub mod testing {
    pub fn loopback_state() -> AppState {
        AppState {
            config: McpConfig::default(),
            auth_policy: AuthPolicy::LoopbackDev,
            service: stub_service(),
        }
    }

    fn stub_service() -> RustarrService {
        let client = RustarrClient::new(&RustarrConfig {
            url: "http://localhost:1".into(),  // unreachable — never called in unit tests
            api_key: "test".into(),
            ..Default::default()
        }).expect("stub client should build");
        RustarrService::new(client, false)
    }
}
```

Use `loopback_state()` in integration tests:

```rust
// tests/tool_dispatch.rs
use rustarr_mcp::testing::loopback_state;

#[tokio::test]
async fn help_returns_help_key() {
    let state = loopback_state();
    let result = execute_tool(&state, "rustarr", json!({"action": "help"})).await.unwrap();
    assert!(result.get("help").is_some());
    assert!(!result["help"].as_str().unwrap().is_empty());
}
```

## Full shart live suite

The canonical live integration suite is `cargo xtask live`. It is guarded so it
can only use the dedicated shart test stack and `RUSTARR_HOME=/home/jmagar/.rustarr-shart`.

```bash
cargo xtask live --suite guard
cargo xtask live --suite all
just live-full-test
```

Suite slices are available when iterating:

```bash
cargo xtask live --suite cli
cargo xtask live --suite rest
cargo xtask live --suite mcp
cargo xtask live --suite services
```

The full suite validates every Rustarr service kind, every CLI business command,
CLI infrastructure lifecycles (`serve`, `serve mcp`, stdio `mcp`, `watch`, and
isolated setup repair/install), REST health/status/auth/OAuth metadata routes,
the MCP protocol surface, every MCP tool action, MCP resources/prompts, and the
service matrix of safe live GETs plus mutation guards. Assertions must check
semantic payload shape or expected errors, not just response success.

The live harness also has a surface inventory gate. If a required CLI/API/MCP
surface is listed in `xtask/src/live/surface.rs` but no exact report marker is
recorded during `cargo xtask live --suite all`, the suite fails before writing
the final report. This is the guard against silently shrinking "every action"
coverage again.

## Legacy live MCP tests

```bash
just dev
bash tests/mcporter/test-mcp.sh
just test-mcporter
```

## Shart live stack prerequisites

Full live tests are allowed to target only the dedicated shart test stack through
`RUSTARR_HOME=/home/jmagar/.rustarr-shart`. The guard requires all supported
service kinds to be present before the complete suite runs:

```text
sonarr, radarr, prowlarr, tautulli, overseerr, bazarr, tracearr, lidarr,
readarr, sabnzbd, qbittorrent, wizarr, notifiarr, plex, jellyfin
```

All service URLs must point at `shart`, `shart.manatee-triceratops.ts.net`, or
`100.118.209.1`. The stack uses curated test config under
`/mnt/user/lab/live/golden/*`; live tests must never use the production
`/home/jmagar/.rustarr` environment.

The mcporter harness is a legacy focused MCP transport smoke test. It is still
guarded by `cargo xtask live --suite guard` and logs calls to
`/tmp/test-mcp.<timestamp>.log`. Prefer `cargo xtask live --suite mcp` or the
full suite for canonical live coverage.

The test script validates:
- auth rejection when `RUSTARR_MCP_TOKEN` is set
- tool semantic behavior for `integrations`, `service_status`, `api_get`, `api_post`, and `help`
- MCP resource behavior for `rustarr://schema/mcp-tool`

Use semantic assertions, not liveness-only checks:

```bash
# Bad test — only proves MCP responded
run_test "server info" "rustarr" '{"action":"integrations"}'

# Good test — proves the service actually returned real data
run_test "integrations lists sonarr support" "rustarr" '{"action":"integrations"}' "sonarr"
```

## Template checks

```bash
just template-check
cargo xtask patterns
scripts/pre-release-check.sh
```

## Principles

- Assert semantic values, not just valid JSON.
- Assert defaults explicitly.
- Keep business logic tests below HTTP when possible.
- Use live mcporter tests for transport/resource/auth integration.
- A test that checks `is_error: false` only verifies the protocol layer responded — prove the actual data is correct.

See `docs/PATTERNS.md` §12, §17, §24 for test sidecar, mcporter, and nextest patterns.
