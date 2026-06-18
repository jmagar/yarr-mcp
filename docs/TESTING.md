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
fn mutating_gate_blocks_without_confirm() {
    let svc = RustarrService::new(stub_client(), false);
    let err = svc.mutating_gate(false).unwrap_err();
    assert!(err.to_string().contains("confirm=true"));
}

#[test]
fn mutating_gate_allows_with_confirm() {
    let svc = RustarrService::new(stub_client(), false);
    assert!(svc.mutating_gate(true).is_ok());
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
    let result = execute_tool(&state, "sonarr", json!({"action": "help"})).await.unwrap();
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

The full suite validates every shart test-stack service kind, every CLI business command,
CLI infrastructure lifecycles (`serve`, `serve mcp`, stdio `mcp`, `watch`, and
isolated setup repair/install), REST health/status/auth/OAuth metadata routes,
the MCP protocol surface, every MCP tool action, MCP resources/prompts, and the
service matrix of live GETs, mutation guards, and confirmed stateful writes on
the disposable shart stack. Assertions must check semantic payload shape,
expected errors, or observable before/after state, not just response success.

The live harness also has a surface inventory gate. If a required CLI/API/MCP
surface is listed in `xtask/src/live/surface.rs` but no exact report marker is
recorded during `cargo xtask live --suite all`, the suite fails before writing
the final report. This is the guard against silently shrinking "every action"
coverage again.

Unless `RUSTARR_BIN` is set, `cargo xtask live` builds and runs
`target/debug/rustarr` from the current checkout. This keeps the live suite from
silently testing a stale release binary while iterating locally.

## mcporter live MCP tests

```bash
bash tests/mcporter/test-mcp.sh
just test-mcporter
cargo xtask live --suite mcporter
```

## Shart live stack prerequisites

Full live tests are allowed to target only the dedicated, disposable shart test stack through
`RUSTARR_HOME=/home/jmagar/.rustarr-shart`. The guard requires all supported
service kinds to be present before the complete suite runs:

```text
sonarr, radarr, prowlarr, tautulli, overseerr, bazarr, tracearr,
sabnzbd, qbittorrent, plex, jellyfin
```

All service URLs must point at `shart`, `shart.manatee-triceratops.ts.net`, or
`100.118.209.1`. The stack uses curated test config under
`/mnt/user/lab/live/golden/*`; live tests must never use the production
`/home/jmagar/.rustarr` environment.
Because shart is a fake test stack, the live suite is expected to exercise
confirmed writes, removals, deletes, process-like operations, and cleanup flows.
Those are mutating test cases, not destructive actions under the project
definition.

The mcporter harness is an xtask-driven exhaustive MCP transport test. It runs
the shart guard, starts a local MCP server against `/home/jmagar/.rustarr-shart`,
uses `mcporter list --schema` to discover the advertised service tools/actions,
then calls every advertised action through `mcporter call`.

The test validates:
- the advertised tool set exactly matches the shart service matrix
- every advertised action for every advertised service is called through
  mcporter
- reads and expected errors satisfy action-specific JSON/error-shape assertions
- confirmed writes validate observable before/after state or accepted upstream
  maintenance state
- confirmed write lifecycles cover Sonarr, Radarr, Prowlarr, Overseerr,
  Jellyfin, Plex, SABnzbd, qBittorrent, Tautulli, Bazarr, and Tracearr
- generic actions, including all matrix-backed `api_get` cases
- curated read actions with semantic payload shape/content assertions
- seeded fixture content for Prowlarr (`Rustarr Live LinuxTracker`), Plex/Jellyfin
  (`Rustarr Live Movies` / `Rustarr Fixture Movie`), and Tautulli library inventory
- unconfirmed mutating actions through `confirm=false` guard/error or preview paths
- confirmed mutating lifecycles for the disposable shart stack: generic tag
  create/update/delete where supported, Sonarr/Radarr item add/edit/search/refresh/delete
  cleanup, Prowlarr indexer tests, Overseerr request create/approve/decline cleanup,
  Jellyfin scan, SABnzbd queue add/pause/resume/remove via a local NZB payload,
  qBittorrent queue add/pause/resume/remove via a test magnet, Plex scan
  against the seeded library, Tautulli maintenance writes, Bazarr seeded blacklist
  delete, and Tracearr seeded debug-session delete

Protected live actions require working shart credentials. For example, a missing
Jellyfin token should make protected Jellyfin actions fail with a live 401 rather
than being counted as success.

Use semantic assertions, not liveness-only checks:

```bash
# Bad test — only proves MCP responded
run_test "server info" "sonarr" '{"action":"integrations"}'

# Good test — proves the service actually returned real data
run_test "integrations lists sonarr support" "sonarr" '{"action":"integrations"}' "sonarr"
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
