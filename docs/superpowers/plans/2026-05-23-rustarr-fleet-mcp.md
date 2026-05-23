# Rustarr Fleet MCP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `rustarr`, a Rust rmcp MCP server for media automation and monitoring services.

**Architecture:** `RustarrClient` owns HTTP transport and auth placement only. `RustarrService` owns service lookup, path validation, action semantics, and response shaping. MCP and CLI shims parse arguments, delegate to `RustarrService`, and keep MCP + CLI parity.

**Tech Stack:** Rust, rmcp, reqwest, serde/serde_json, toml, tokio, axum inherited from rustarr.

---

## Files

- Rename: `src/rustarr.rs` -> `src/rustarr.rs`
- Rename: `src/rustarr_tests.rs` -> `src/rustarr_tests.rs`
- Rename: `plugins/rustarr/` -> `plugins/rustarr/`
- Modify: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/main.rs`, `src/config.rs`, `src/app.rs`, `src/actions.rs`, `src/cli.rs`, `src/mcp/tools.rs`, `src/mcp/schemas.rs`
- Modify tests: `tests/tool_dispatch.rs`, `tests/cli_parse.rs`, `src/*_tests.rs`
- Modify docs/config: `README.md`, `AGENTS.md`, `.env.rustarr`, `config.rustarr.toml`, `config.toml`, `server.json`, plugin docs

### Task 1: Rename Template Identity

- [ ] Replace package/library/binary identity with `rustarr`.

Run:

```bash
git mv src/rustarr.rs src/rustarr.rs
git mv src/rustarr_tests.rs src/rustarr_tests.rs
git mv plugins/rustarr plugins/rustarr
perl -0pi -e 's/rustarr/rustarr/g; s/rustarr/rustarr/g; s/Rustarr/Rustarr/g; s/rustarr/Rustarr_TMP/g; s/RUSTARR/RUSTARR/g; s/Rustarr_TMP/rustarr/g' $(git ls-files)
```

Expected: `cargo test` may fail only for not-yet-updated rustarr action behavior, not unresolved `rustarr` module names.

- [ ] Run:

```bash
cargo fmt
cargo test --no-run
```

Expected: compile errors identify remaining stale names; fix them before Task 2.

### Task 2: Service Catalog And Config

- [ ] Replace the single upstream config with a catalog:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RustarrConfig {
    pub services: Vec<ServiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ServiceConfig {
    pub name: String,
    pub kind: ServiceKind,
    pub base_url: String,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}
```

- [ ] Add `ServiceKind` variants for `sonarr`, `radarr`, `prowlarr`, `tautulli`, `overseerr`, `bazarr`, `tracearr`, `lidarr`, `readarr`, `sabnzbd`, `qbittorrent`, `wizarr`, `notifiarr`, `plex`, and `jellyfin`.

- [ ] Add env loading for `RUSTARR_SERVICES` as comma-separated names and per-service variables:

```text
RUSTARR_<NAME>_KIND
RUSTARR_<NAME>_URL
RUSTARR_<NAME>_API_KEY
RUSTARR_<NAME>_USERNAME
RUSTARR_<NAME>_PASSWORD
RUSTARR_<NAME>_TOKEN
```

- [ ] Test default required service kinds are represented once and duplicate Tautulli is not created.

### Task 3: Shared Client

- [ ] Implement `RustarrClient` with:

```rust
pub async fn get_json(&self, service: &ServiceConfig, path: &str) -> Result<Value>;
pub async fn post_json(&self, service: &ServiceConfig, path: &str, body: Value) -> Result<Value>;
```

- [ ] Add auth placement by kind:

```text
Arr family and Overseerr: X-Api-Key header
SABnzbd: apikey query and output=json
Tautulli: apikey query and cmd-style paths
Plex: X-Plex-Token query
Jellyfin: X-Emby-Token header
qBittorrent: form login then cookie jar
Fallback services: X-Api-Key if api_key is set, bearer token if token is set
```

- [ ] Reject absolute URLs, `..`, empty paths, and paths with query secrets supplied by callers.

### Task 4: Service Actions

- [ ] Replace greet/echo with rustarr actions:

```text
integrations
service_status
api_get
api_post
help
```

- [ ] `integrations` returns configured and supported services with secret fields omitted.

- [ ] `service_status` uses safe default endpoints by kind: Arr `/api/v3/system/status` or `/api/v1/system/status`, Overseerr `/api/v1/status`, SABnzbd `/api?mode=version`, qBittorrent `/api/v2/app/version`, Jellyfin `/System/Info/Public`, Plex `/identity`, generic `/api`.

- [ ] `api_get` and `api_post` require a configured service name plus safe relative path.

### Task 5: MCP And CLI Parity

- [ ] Add action metadata: `api_post` uses `rustarr:write`; all other non-help actions use `rustarr:read`; `help` is public.

- [ ] Update MCP tool name to `rustarr`, schema properties to `service`, `path`, and `body`.

- [ ] Update CLI:

```text
rustarr integrations
rustarr status --service NAME
rustarr get --service NAME --path PATH
rustarr post --service NAME --path PATH --body JSON
rustarr help
```

- [ ] Add parity tests for each action through MCP and CLI parsing.

### Task 6: Docs, Review, And Verification

- [ ] Update README and rustarrs to say this is an upstream-client server and REST/Web are not expanded.

- [ ] Record researched API docs and sparse-doc caveats in README.

- [ ] Run:

```bash
cargo fmt
cargo test
cargo clippy -- -D warnings
```

- [ ] Commit and push from the worktree. If a GitHub remote exists, open a PR with the epic id and verification evidence.

## Self-Review

Spec coverage: all requested integrations are represented by `ServiceKind`; MCP + CLI parity is explicit; common abstractions are required; REST/Web expansion is excluded.

Placeholder scan: no task contains TBD or unspecified follow-up work.

Type consistency: config, client, service, MCP, and CLI all use `Rustarr*`, `ServiceConfig`, `ServiceKind`, `service`, `path`, and `body`.
