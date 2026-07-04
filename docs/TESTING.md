---
title: "Testing"
doc_type: "guide"
status: "active"
owner: "yarr"
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
pub struct YarrService { ... }
impl YarrService { ... }

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

// src/app_tests.rs
use super::*;  // access to private items

// Only destructive deletes are gated; non-destructive writes run immediately.
#[tokio::test]
async fn api_delete_requires_confirm() {
    let svc = loopback_state().service;
    let err = svc
        .api_delete("sonarr", "/api/v3/movie/1", None, false)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("confirm=true"));
}

#[tokio::test]
async fn api_post_runs_without_confirm() {
    let svc = loopback_state().service;
    // No confirm gate — the stub client just attempts the (failing) upstream call.
    let _ = svc.api_post("sonarr", "/api/v3/command", json!({})).await;
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

    fn stub_service() -> YarrService {
        let client = YarrClient::new(&YarrConfig {
            url: "http://localhost:1".into(),  // unreachable — never called in unit tests
            api_key: "test".into(),
            ..Default::default()
        }).expect("stub client should build");
        YarrService::new(client, false)
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
    let result = execute_tool(&state, "sonarr", json!({"action": "help"})).await.unwrap();
    assert!(result.get("help").is_some());
    assert!(!result["help"].as_str().unwrap().is_empty());
}
```

## Full shart live suite

The canonical live integration suite is `cargo xtask live`. It is guarded so it
can only use the dedicated shart test stack and `YARR_HOME=/home/jmagar/.yarr-shart`.

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
cargo xtask live --suite mcporter
cargo xtask live --suite services
```

The full suite validates every shart test-stack service kind, every CLI business command,
CLI infrastructure lifecycles (`serve`, `serve mcp`, stdio `mcp`, `watch`, and
isolated setup repair/install), REST health/status/auth/OAuth metadata routes,
the MCP protocol surface, every generated OpenAPI callable through mcporter and
the single `yarr` MCP tool, MCP resources/prompts, and the service matrix of live
GETs, safe upstream-error probes, destructive-delete guards, and confirmed
stateful writes on the disposable shart stack. Assertions
must check semantic payload shape, expected errors, or observable before/after
state, not just response success.

The live harness also has a surface inventory gate. If a required CLI/API/MCP
surface is listed in `xtask/src/live/surface.rs` but no exact report marker is
recorded during `cargo xtask live --suite all`, the suite fails before writing
the final report. This is the guard against silently shrinking "every action"
coverage again.

The generated OpenAPI contract slice is strict. Each generated upstream operation
must end as a successful 2xx contract check, a schema-mismatch record with live
response evidence, or a rejected record with live upstream/transport evidence. A
transport failure, timeout, or unclassified upstream rejection is a failed
contract check and fails the live suite.

The mcporter slice applies the same contract rules over MCP: `cargo xtask live
--suite mcporter` starts Yarr against `YARR_HOME=/home/jmagar/.yarr-shart`
and calls each generated per-service Code Mode callable via `mcporter call ...
yarr`. Generated operations that rewrite config/auth state or stop services are
run in an isolated reset phase when shart has a ZFS golden target for that
service (`backup/lab/live/golden/<service>@configured-v1`); the harness rolls the
dataset back before and after the reset-required group. Missing IDs and unseeded
optional features are exercised with deterministic fallback inputs, not skipped.
Non-JSON endpoints are invoked too; they pass only if the generated transport can
handle the response shape. `--suite mcp` remains the lightweight MCP protocol
smoke slice.

Unless `YARR_BIN` is set, `cargo xtask live` builds and runs
`target/debug/yarr` from the current checkout. This keeps the live suite from
silently testing a stale release binary while iterating locally.

`docs/LIVE_ENDPOINT_COVERAGE.md` is regenerated by `--suite all` and verified by
`cargo xtask live --suite coverage-check`. After editing the coverage map
(`xtask/src/live/coverage/services_part*.rs`) you can refresh the doc from the
last report without a full live re-run: `cargo xtask live --coverage-write`. The
harness orders its timeouts connect (10s) < yarr client (`YARR_HTTP_TIMEOUT_SECS`,
90s in the harness) < per-command (120s) so a slow upstream read resolves inside
yarr instead of being killed mid-call and aborting the run.

## Live MCP transport tests

```bash
cargo xtask live --suite mcp          # MCP transport via the single yarr tool
cargo xtask live --suite mcporter     # every generated callable via mcporter/yarr
bash tests/mcporter/test-mcp.sh       # thin wrapper -> --suite mcporter
```

## Shart live stack prerequisites

Full live tests are allowed to target only the dedicated, disposable shart test stack through
`YARR_HOME=/home/jmagar/.yarr-shart`. The guard requires all supported
service kinds to be present before the complete suite runs:

```text
sonarr, radarr, prowlarr, tautulli, overseerr, bazarr, tracearr,
sabnzbd, qbittorrent, plex, jellyfin
```

All service URLs must point at `shart`, `shart.manatee-triceratops.ts.net`, or
`100.118.209.1`. The stack uses curated test config under
`/mnt/user/lab/live/golden/*`; live tests must never use the production
`/home/jmagar/.yarr` environment.
Because shart is a fake test stack, the live suite is expected to exercise
confirmed writes, removals, deletes, process-like operations, and cleanup flows.
Those are mutating test cases, not destructive actions under the project
definition.

The live suites run the shart guard, start a local MCP server against
`/home/jmagar/.yarr-shart`, and validate, across `--suite mcp|contract|cli|lifecycles`:
- `tools/list` advertises exactly the single `yarr` tool (no per-service tools);
  initialize, the schema resource, and the `quick_start` prompt resolve
- a representative `yarr` Code Mode round-trip reaches an upstream service and
  returns real status fields; a write to a bad path surfaces the service-native
  error through the Code Mode envelope
- (`contract`) every generated OpenAPI operation for the 6 spec-backed services
  (sonarr/radarr/prowlarr/overseerr/jellyfin/plex) dispatches via the `op` action,
  with create-first seeding and schema-validated responses
- (`cli`) per-service `status`, all matrix-backed `api_get` cases, and an
  unconfirmed `api_post` upstream-error probe per service
- (`lifecycles`, destructive — skipped under `--no-destructive`) confirmed stateful
  write lifecycles for the doc-based services: SABnzbd / qBittorrent `download_*`
  add/pause/resume/remove with queue-state polling (against an in-process fixture
  NZB / a test magnet), Tautulli `stats_*` maintenance, and Bazarr / Tracearr seeded
  `api_delete` cleanup (rows seeded over `ssh shart docker exec`, then verified gone)
- seeded fixture content for Prowlarr (`Yarr Live LinuxTracker`), Plex/Jellyfin
  (`Yarr Live Movies` / `Yarr Fixture Movie`), and Tautulli library inventory

The SABnzbd lifecycle needs `YARR_LIVE_FIXTURE_HOST` set to a host/IP reachable
from shart (defaults to the dookie tailnet IP) so its fixture NZB server is fetchable.

Protected live actions require working shart credentials. For example, a missing
Jellyfin token should make protected Jellyfin actions fail with a live 401 rather
than being counted as success.

Use semantic assertions, not liveness-only checks:

```bash
# Bad test — only proves MCP responded
run_test "server status" "yarr" '{"code":"async () => sonarr.service_status()"}'

# Good test — proves the service actually returned real data
run_test "sonarr status reports version" "yarr" '{"code":"async () => (await sonarr.service_status()).version"}' "version"
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
