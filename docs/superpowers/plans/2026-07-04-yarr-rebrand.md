# Yarr Rebrand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fully rebrand the Rustarr application, binary, package, plugin, docs, and runtime-facing identity to Yarr.

**Architecture:** Treat this as a source-of-truth rename, not a compatibility layer. The MCP tool is already named `yarr`; the work aligns Cargo metadata, configuration, scopes, plugin packaging, live harnesses, active docs, and internal app-owned symbols to that identity while leaving upstream service names such as Sonarr and Radarr untouched. Keep internal Rust symbol/module churn out of the first public runtime contract pass, then do the internal cleanup after the public contract is green. Work proceeds from public runtime identity first, then plugin/runtime/docs, then internal symbol cleanup, then a stale-name audit.

**Tech Stack:** Rust 2024, Cargo workspace, Axum/rmcp MCP server, QuickJS Code Mode, shell plugin hooks, Claude/Codex/Gemini plugin manifests, Beads, xtask live harness, Docker Compose.

## Global Constraints

- Canonical identity is `yarr` / `Yarr` / `YARR_`.
- The npm package/distribution name is `yarr-mcp`; everything else is `yarr`.
- MCP tool name remains `yarr`.
- Upstream media service names stay unchanged: `sonarr`, `radarr`, `prowlarr`, `tautulli`, `overseerr`, `bazarr`, `tracearr`, `lidarr`, `readarr`, `sabnzbd`, `qbittorrent`, `wizarr`, `notifiarr`, `plex`, `jellyfin`.
- Prefer a hard rename with no rustarr compatibility wrappers unless the user explicitly asks for compatibility later.
- Old `RUSTARR_*` env vars and `rustarr:*` scopes must not be accepted as compatibility aliases after the hard rename.
- Plugin manifests must not contain a `version` field.
- `CLAUDE.md` is the source of truth for agent memory; do not edit `AGENTS.md` or `GEMINI.md` directly.
- Keep the thin-shim rule: CLI and MCP shims parse inputs, call service/app logic, and format output only.
- Do not hand-edit vendored OpenAPI specs or generated OpenAPI tables unless generated content genuinely contains app identity and the generator is run.
- Runtime sync proof uses compose rebuild/recreate plus health/version checks; do not `docker cp` into the runtime container.
- Do not run `cargo xtask gen-openapi`, `scripts/refresh-docs.sh`, full `cargo xtask live --suite all`, or all-service mcporter/contract checks as inner-loop validation. Reserve expensive/generated checks for the final audit unless a targeted change requires them.

---

## File Structure

- `Cargo.toml` / `Cargo.lock`: package, binary, crate references, test-support dependency name.
- `src/config.rs` and `src/config/*`: public config structs, env prefixes, default app directories, server name defaults.
- `src/main.rs`, `src/server.rs`, `src/mcp/*`, `src/actions/*`: scopes, metadata, resource URIs, schema names, user-facing errors.
- `src/rustarr.rs` and `src/rustarr/*`: transport facade; internal module/type rename happens after public runtime identity is green.
- `src/app/*`, `src/cli*`, `src/logging/*`: public type references and user-facing copy.
- `tests/*` and `src/*_tests.rs`: parser, plugin, parity, config, server, and schema assertions.
- `plugins/rustarr/**`: full MCP plugin package; rename to `plugins/yarr/**`.
- `.claude-plugin/marketplace.json`, `.agents/plugins/marketplace.json`, `plugins/README.md`: marketplace and plugin catalog identity.
- `Justfile`, `Dockerfile`, `docker-compose.yml`, `.github/workflows/*`: binary install, image/service names, CI/release paths.
- `xtask/src/live*`, `xtask/src/main.rs`, `xtask/src/tool_docs.rs`: live harness env, binary selection, fixture names, generated docs.
- Active docs: `README.md`, `CHANGELOG.md`, `CLAUDE.md`, `docs/AUTH.md`, `docs/DOCKER.md`, `docs/MCPORTER.md`, `docs/PLUGINS.md`, `docs/TESTING.md`, `docs/XTASKS.md`, `docs/TOOLS_ACTIONS_ENDPOINTS.md`, `docs/LIVE_ENDPOINT_COVERAGE.md`, `docs/PATTERNS.md`, `docs/PHILOSOPHY.md`, `docs/no-mcp-variant.md`.

### Task 1: Public Runtime Cargo, Config, Scopes, and MCP Identity

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/config.rs`
- Modify: `src/config/*`
- Modify: `src/server.rs`
- Modify: `src/mcp/*`
- Modify: `src/actions/*`
- Modify: `src/rustarr.rs` and `src/rustarr/*` only where public/runtime strings or constants require it
- Modify: `src/*_tests.rs`
- Modify: `tests/cli_parse.rs`
- Modify: `tests/tool_dispatch.rs`
- Modify: `tests/parity.rs`

**Interfaces:**
- Consumes: Existing public identity `rustarr`, `Rustarr*`, `RUSTARR_*`, `rustarr:*`, `rustarr://schema/mcp-tool`.
- Produces: Cargo package/binary `yarr`, scopes `yarr:read` and `yarr:write`, env prefix `YARR_*`, resource URI `yarr://schema/mcp-tool`, default app data under `~/.yarr`, and fail-closed legacy namespace behavior.

- [ ] **Step 1: Write failing tests for core identity**

Add or update assertions in the relevant existing test files:

```rust
assert_eq!(env!("CARGO_PKG_NAME"), "yarr");
assert_eq!(rustarr::READ_SCOPE, "yarr:read");
assert_eq!(rustarr::WRITE_SCOPE, "yarr:write");
assert_ne!(rustarr::READ_SCOPE, "rustarr:read");
assert_ne!(rustarr::WRITE_SCOPE, "rustarr:write");
```

In schema/resource tests, assert:

```rust
assert_eq!(tool["name"], "yarr");
assert_eq!(tool["inputSchema"]["x-yarr-guidance"]["schema_resource"], "yarr://schema/mcp-tool");
```

In config tests, assert renamed env variables are accepted:

```rust
unsafe {
    std::env::set_var("YARR_HOME", dir.path());
    std::env::set_var("YARR_SERVICES", "sonarr");
    std::env::set_var("YARR_SONARR_KIND", "sonarr");
    std::env::set_var("YARR_SONARR_URL", "https://api.yarr.test");
    std::env::set_var("YARR_SONARR_API_KEY", "secret");
}
```

Also add tests for hard-break behavior:

```rust
unsafe {
    std::env::set_var("RUSTARR_SERVICES", "sonarr");
    std::env::remove_var("YARR_SERVICES");
}
let result = Config::load();
assert!(result.is_err(), "legacy RUSTARR_* must not configure yarr");
```

Add `.env` allowlist tests:

```rust
fs::write(
    dir.path().join(".env"),
    "YARR_SERVICES=sonarr\nRUSTARR_NOAUTH=true\nPATH=/tmp/evil\nLD_PRELOAD=/tmp/evil.so\nRUST_LOG=debug\n",
)?;
```

Expected behavior: `YARR_*` and `RUST_LOG` are injectable; `RUSTARR_*`, `PATH`, `LD_PRELOAD`, and other unrelated keys are rejected or ignored with a clear diagnostic; already-set process env still wins.

- [ ] **Step 2: Run focused tests to verify failure**

Run:

```bash
cargo check
cargo test config_tests mcp::schemas_tests mcp::rmcp_server_tests actions::model_tests tests::parity -- --nocapture
```

Expected: failures naming missing `YARR_*`, `yarr:*`, or remaining `rustarr` metadata.

- [ ] **Step 3: Rename Cargo package and binary**

Change `Cargo.toml`:

```toml
[package]
name = "yarr"

[[bin]]
name = "yarr"
path = "src/main.rs"
```

Update dev-dependency self-reference:

```toml
yarr = { path = ".", features = ["test-support"] }
```

Run:

```bash
cargo metadata --format-version 1 >/tmp/yarr-metadata.json
cargo check
```

Expected: compile errors point to import/type names still using `rustarr`.

- [ ] **Step 4: Rename public runtime identity without broad internal churn**

Rename current public/user-facing identity consistently. Do not rename internal Rust types/modules in this task unless required for compile/runtime correctness. It is acceptable for internal type names such as `RustarrService` or `RustarrClient` to remain temporarily here; Task 5 removes those remaining app-owned names after the public contract is green.

```rust
pub const READ_SCOPE: &str = "yarr:read";
pub const WRITE_SCOPE: &str = "yarr:write";
```

Internal symbol/file rename is handled later in this plan so the public contract review is not buried under mechanical churn.

- [ ] **Step 5: Rename env/config constants and fail closed on stale names**

```rust
const SERVICE_HOME_DIRNAME: &str = ".yarr";
const ENV_PREFIX: &str = "YARR";
const MCP_ENV_PREFIX: &str = "YARR_MCP";
```

Update service loading patterns:

```rust
let services = std::env::var("YARR_SERVICES").ok();
let key = format!("YARR_{name}_API_KEY");
```

Reject legacy runtime namespace during startup/config load:

```rust
if std::env::vars().any(|(key, _)| key.starts_with("RUSTARR_")) {
    anyhow::bail!("legacy RUSTARR_* variables are not supported; rename them to YARR_*");
}
```

Add service env-name collision protection:

```rust
let env_name = service_env_name(name);
if !seen.insert(env_name.clone()) {
    anyhow::bail!("duplicate service env namespace YARR_{env_name}_*");
}
```

- [ ] **Step 6: Rename OAuth/auth config atomically**

Update OAuth/static auth paths together:

```rust
env_prefix("YARR_MCP");
cookie_name("yarr_mcp_session");
```

Required acceptance:

```rust
assert_eq!(READ_SCOPE, "yarr:read");
assert_eq!(WRITE_SCOPE, "yarr:write");
assert!(write_token_scopes.contains("yarr:write"));
assert!(!write_token_scopes.contains("rustarr:write"));
```

Update `auth_config_sources` to emit only `YARR_MCP_*` keys. Add tests proving old `rustarr:*` scopes do not authorize actions requiring `yarr:*`, while `yarr:write` satisfies `yarr:read`.

- [ ] **Step 7: Rename scopes, resource URIs, and schema extension names**

Update MCP metadata keys and resource strings:

```rust
const SCHEMA_RESOURCE_URI: &str = "yarr://schema/mcp-tool";
```

Prefer `x-yarr-action-metadata`, `x-yarr-service-metadata`, and `x-yarr-agent-guidance` for current schema extension keys. Update tests and docs together.

- [ ] **Step 8: Run focused core verification**

Run:

```bash
cargo fmt
cargo check
cargo test config_tests
cargo test mcp::schemas_tests
cargo test mcp::rmcp_server_tests
cargo test tests::parity
cargo run -- --help
cargo run -- --version
```

Expected: tests pass; help/version display `yarr` and not `rustarr` as current identity.

- [ ] **Step 9: Commit core identity changes**

Run:

```bash
git add Cargo.toml Cargo.lock src tests
git commit -m "refactor: rename public runtime identity to yarr"
```

### Task 2: Plugin and Marketplace Identity

**Files:**
- Modify/rename: `plugins/rustarr/**` to `plugins/yarr/**`
- Modify: `.claude-plugin/marketplace.json`
- Modify: `.agents/plugins/marketplace.json`
- Modify: `plugins/README.md`
- Modify: `plugins/*/README.md`
- Modify: `plugins/*/gemini-extension.json`
- Modify: `tests/plugin_contract.rs`
- Modify: `tests/template_invariants.rs`

**Interfaces:**
- Consumes: `yarr` binary from Task 1.
- Produces: full MCP plugin package `plugins/yarr`, npm package/distribution metadata `yarr-mcp`, all other plugin display/runtime identity `yarr`, setup hooks that invoke `bin/yarr`, no-MCP scripts that target the renamed package, and contract tests that validate the renamed package.

- [ ] **Step 1: Write failing plugin contract assertions**

Update plugin contract tests to expect `plugins/yarr` and `YARR_*` setup output:

```rust
assert!(manifest_path.ends_with("plugins/yarr/.claude-plugin/plugin.json"));
assert!(env_file.contains("YARR_SONARR_URL=https://api.yarr.test"));
assert!(env_file.contains("YARR_MCP_TOKEN=mcp-secret"));
assert!(!plugin_json.contains("\"version\""));
assert!(!setup_output.contains("CLAUDE_PLUGIN_OPTION_RUSTARR_"));
```

- [ ] **Step 2: Run plugin tests to verify failure**

Run:

```bash
cargo test --test plugin_contract
cargo test --test template_invariants
```

Expected: tests fail on old `plugins/rustarr`, `rustarr-mcp`, or `RUSTARR_*` assumptions.

- [ ] **Step 3: Rename hook executable, plugin bin, and setup env mapping first**

Update hook and script commands before directory movement:

```json
"command": "${CLAUDE_PLUGIN_ROOT}/bin/yarr setup plugin-hook"
```

Update shell defaults:

```bash
binary="${YARR_MCP_BIN:-yarr}"
```

Update plugin setup option promotion so every full-plugin option maps to `YARR_*` and no `CLAUDE_PLUGIN_OPTION_RUSTARR_*` entries remain. Preserve `sensitive: true` on API tokens, OAuth client secret, service API keys, Plex token, qBittorrent password, and any other secret/password fields.

- [ ] **Step 4: Rename full plugin directory**

Run:

```bash
git mv plugins/rustarr plugins/yarr
```

- [ ] **Step 5: Update manifests without adding versions**

Use `yarr-mcp` only where the schema is specifically a package/distribution/npm-style name. Use `yarr` for display/plugin/runtime identity everywhere else.

For npm/package distribution metadata:

```json
{
  "name": "yarr-mcp",
  "description": "MCP bridge to the yarr media-automation fleet."
}
```

For non-npm plugin display/runtime metadata:

```json
{
  "name": "yarr",
  "description": "Connects agents to the yarr MCP server."
}
```

Repository/homepage should be the renamed repo URL once the repo is renamed:

```json
"homepage": "https://github.com/jmagar/yarr",
"repository": "https://github.com/jmagar/yarr"
```

Before proceeding, run:

```bash
if grep -R '"version"' .claude-plugin .agents/plugins plugins/yarr/.claude-plugin plugins/yarr/.codex-plugin; then
  echo "version field found" >&2
  exit 1
fi
```

Expected: no output and exit 0.

- [ ] **Step 6: Update no-MCP generator/check scripts**

Update no-MCP protected-branch tooling in the same bead:

```text
scripts/apply-no-mcp-marketplace
scripts/check-no-mcp-drift
.github/workflows/sync-marketplace-no-mcp.yml
scripts/validate-plugin-layout.sh
```

Acceptance: no script still targets `plugins/rustarr` or `skills/rustarr` as the current full-plugin path.

- [ ] **Step 7: Update bundled and standalone plugin docs**

Update full plugin docs to use `yarr`, and standalone service plugin docs to say:

```markdown
For the full media fleet behind one MCP tool, install the `yarr` plugin instead.
```

- [ ] **Step 8: Run plugin verification**

Run:

```bash
cargo test --test plugin_contract
cargo test --test template_invariants
```

Expected: pass.

- [ ] **Step 9: Commit plugin identity changes**

Run:

```bash
git add .claude-plugin .agents/plugins plugins scripts .github tests/plugin_contract.rs tests/template_invariants.rs
git commit -m "refactor: rename plugin package to yarr"
```

### Task 3: Live Harness, Install Paths, Docker, and CI Identity

**Files:**
- Modify: `Justfile`
- Modify: `Dockerfile`
- Modify: `docker-compose.yml`
- Modify: `.github/workflows/*`
- Modify: `xtask/src/main.rs`
- Modify: `xtask/src/live.rs`
- Modify: `xtask/src/live/*`
- Modify: `xtask/src/*_tests.rs`
- Modify: `xtask/src/tool_docs.rs`

**Interfaces:**
- Consumes: `yarr` binary/env names from Task 1 and `plugins/yarr/bin/yarr` from Task 2.
- Produces: local install path `~/.local/bin/yarr`, repository binary `bin/yarr`, plugin binary `plugins/yarr/bin/yarr`, live env `YARR_HOME`, `YARR_BIN`, and `YARR_ALLOW_DESTRUCTIVE`. Compose service/container/image rename is deferred unless the implementation proves it is required for current user-facing behavior.

- [ ] **Step 1: Write failing live/install assertions**

Update xtask and install tests to expect:

```rust
env.insert("YARR_HOME".into(), SHART_HOME.into());
env.insert("YARR_SERVICES".into(), "sonarr,radarr,prowlarr,tautulli,overseerr,bazarr,tracearr,sabnzbd,qbittorrent,plex,jellyfin".into());
assert!(err.contains("YARR_HOME must be /home/jmagar/.yarr-shart"));
assert!(!guarded.keys().any(|key| key.starts_with("RUSTARR_")));
```

Update install assertions around:

```bash
~/.local/bin/yarr
bin/yarr
plugins/yarr/bin/yarr
```

- [ ] **Step 2: Run focused live harness tests to verify failure**

Run:

```bash
cargo test -p xtask live
cargo test -p xtask tool_docs
```

Expected: failures point to `RUSTARR_*`, `rustarr` binary paths, or old compose names.

- [ ] **Step 3: Rename Justfile install and smoke commands**

Update install path logic:

```bash
YARR_TARGET_DIR="${CARGO_TARGET_DIR:-target}"
YARR_BIN="$repo/$YARR_TARGET_DIR/$profile/yarr"
ln -sf "$YARR_BIN" ~/.local/bin/yarr
install -m 755 "$YARR_BIN" bin/yarr
install -m 755 "$YARR_BIN" plugins/yarr/bin/yarr
```

Update dev commands to use:

```bash
YARR_MCP_HOST=127.0.0.1 YARR_MCP_PORT=40070 YARR_MCP_NO_AUTH=true cargo run -- serve mcp
```

- [ ] **Step 4: Rename xtask live environment**

Update guard and process setup:

```rust
if key.starts_with("YARR_") {
    guarded.insert(key, value);
}
env.insert("YARR_HOME".into(), super::guard::SHART_HOME.into());
env.entry("YARR_HTTP_TIMEOUT_SECS".into()).or_insert_with(|| "20".into());
```

Set:

```rust
pub const SHART_HOME: &str = "/home/jmagar/.yarr-shart";
```

Use `YARR_BIN` for binary override.

- [ ] **Step 5: Keep compose/container rename deferred unless required**

Do not rename `rustarr-mcp` compose service/container/image as part of the core rebrand unless a direct user-facing contract requires it. If the external deployment name stays transitional, document that clearly and keep runtime proof commands using the existing compose service while verifying that the binary inside is `yarr`.

If a rename is required, update all compose, workflow, health, reverse-proxy, and runtime sync commands atomically.

- [ ] **Step 6: Harden destructive override behavior**

Update `YARR_ALLOW_DESTRUCTIVE` semantics:

```rust
assert!(!destructive_override_enabled());
unsafe { std::env::set_var("RUSTARR_ALLOW_DESTRUCTIVE", "true") };
assert!(!destructive_override_enabled(), "legacy destructive override must be ignored");
```

If `YARR_ALLOW_DESTRUCTIVE=true` is accepted outside tests, it must emit a prominent warning. Prefer restricting it to `YARR_HOME=/home/jmagar/.yarr-shart` or an equivalent live-harness-only marker.

- [ ] **Step 7: Run live/install verification**

Run:

```bash
cargo test -p xtask
just install-local
~/.local/bin/yarr --version
cargo xtask live --suite guard
cargo xtask live --suite mcp
```

If shart live credentials are available, run:

```bash
cargo xtask live --suite contract --service sonarr
```

Expected: tests pass; local `yarr --version` reports the renamed binary.

- [ ] **Step 8: Commit runtime identity changes**

Run:

```bash
git add Justfile Dockerfile docker-compose.yml .github xtask
git commit -m "refactor: rename runtime harness to yarr"
```

### Task 4: Active Docs and Changelog

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `CLAUDE.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/AUTH.md`
- Modify: `docs/DOCKER.md`
- Modify: `docs/MCPORTER.md`
- Modify: `docs/PLUGINS.md`
- Modify: `docs/TESTING.md`
- Modify: `docs/XTASKS.md`
- Modify: `docs/LIVE_ENDPOINT_COVERAGE.md`
- Modify: `docs/no-mcp-variant.md`

**Interfaces:**
- Consumes: final names and paths from Tasks 1-3.
- Produces: active operator/generated docs that use `yarr`, `YARR_*`, `~/.yarr`, `plugins/yarr`, npm package `yarr-mcp`, and the current runtime proof commands. Historical/session docs are not rewritten.

- [ ] **Step 1: Add changelog entry**

Under `[Unreleased]`, add:

```markdown
- Rebranded the application, CLI binary, plugin package, configuration namespace, MCP metadata, live harness, and active docs from Rustarr to Yarr. The MCP tool remains `yarr`; upstream service names such as Sonarr and Radarr are unchanged.
```

- [ ] **Step 2: Update README quickstart**

Use:

```bash
cargo install --path .
yarr --help

YARR_MCP_HOST=127.0.0.1
YARR_MCP_PORT=40070
YARR_MCP_TOKEN=change-me
YARR_SERVICES=sonarr,radarr,prowlarr,tautulli,overseerr,bazarr,tracearr,sabnzbd,qbittorrent,plex,jellyfin
YARR_SONARR_URL=http://sonarr:8989
YARR_SONARR_API_KEY=...
```

- [ ] **Step 3: Update auth and Docker docs**

Replace active examples:

```bash
export YARR_MCP_TOKEN=$(openssl rand -hex 32)
YARR_MCP_AUTH_MODE=oauth
YARR_MCP_PUBLIC_URL=https://your-server.yarr.example
YARR_MCP_HOST=127.0.0.1 YARR_MCP_NO_AUTH=true
```

Docker examples should use `YARR_MCP_HOST_PORT`; compose service/container name stays documented as transitional if implementation leaves it as `rustarr-mcp`.

- [ ] **Step 4: Update testing and mcporter docs**

Use:

```bash
YARR_HOME=/home/jmagar/.yarr-shart cargo xtask live --suite all
YARR_BIN=target/release/yarr cargo xtask live --suite all
mcporter call --http-url http://127.0.0.1:40170/mcp --allow-http yarr
```

- [ ] **Step 5: Update agent/plugin docs**

`CLAUDE.md` and plugin docs should state:

```markdown
The MCP surface is a single `yarr` tool. The CLI binary is `yarr`. Public configuration uses `YARR_*`.
```

Do not edit sibling `AGENTS.md` or `GEMINI.md` directly; ensure they remain symlinks to `CLAUDE.md` if touched by tooling.

- [ ] **Step 6: Run doc-sensitive verification**

Run:

```bash
cargo test
python3 scripts/check-schema-docs.py --check || python3 scripts/check-schema-docs.py --write
cargo xtask live --coverage-check
```

If `cargo xtask live --coverage-check` requires live state not available in this session, record the exact blocker in the final verification notes.

- [ ] **Step 7: Commit docs**

Run:

```bash
git add README.md CHANGELOG.md CLAUDE.md docs
git commit -m "docs: update active docs for yarr"
```

### Task 5: Internal Rust Symbol and Module Cleanup

**Files:**
- Modify/rename: `src/rustarr.rs` to `src/yarr.rs`
- Modify/rename: `src/rustarr/*` to `src/yarr/*`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/app/*`
- Modify: `src/actions/*`
- Modify: `src/cli*`
- Modify: `src/mcp/*`
- Modify: `src/*_tests.rs`
- Modify: `tests/*`
- Modify: `xtask/src/patterns/*`

**Interfaces:**
- Consumes: public runtime identity from Task 1 and green plugin/runtime/docs tasks.
- Produces: internal app-owned Rust symbols and modules named `Yarr*` / `yarr`, with no app-owned `Rustarr*` symbols left. Upstream product names such as `Sonarr` and `Radarr` remain unchanged.

- [ ] **Step 1: Add internal-name audit target**

Run:

```bash
git grep -n -e 'Rustarr' -e 'rustarr::' -e 'crate::rustarr' -- src tests xtask \
  > /tmp/yarr-internal-symbol-audit.txt || true
```

Expected: the file lists app-owned internal symbols to rename, not upstream service names like Sonarr/Radarr.

- [ ] **Step 2: Rename transport module and public app-owned types**

Use `git mv` for module files if the diff stays reviewable:

```bash
git mv src/rustarr.rs src/yarr.rs
git mv src/rustarr src/yarr
```

Rename app-owned symbols consistently:

```rust
RustarrConfig -> YarrConfig
RustarrClient -> YarrClient
RustarrService -> YarrService
RustarrAction -> YarrAction
RustarrRmcpServer -> YarrRmcpServer
```

Do not rename upstream service domain words containing `arr`, such as Sonarr or Radarr.

- [ ] **Step 3: Update tests and pattern guards**

Update imports and thin-shim pattern checks:

```rust
use yarr::{YarrAction, testing::loopback_state};
```

Update any pattern-enforcement messages that name `RustarrService` so they now expect `YarrService`.

- [ ] **Step 4: Run internal cleanup verification**

Run:

```bash
cargo fmt
cargo test
cargo clippy -- -D warnings
```

Expected: all pass.

- [ ] **Step 5: Commit internal cleanup**

Run:

```bash
git add src tests xtask
git commit -m "refactor: rename internal symbols to yarr"
```

### Task 6: Final Stale-Name Audit and Full Verification

**Files:**
- Modify: any file with unintentional current app identity leaks found by audit.
- Test: full repository gates.

**Interfaces:**
- Consumes: all previous task outputs.
- Produces: verified yarr rebrand with classified remaining historical `rustarr` mentions and clean Beads/PR handoff.

- [ ] **Step 1: Run semantic stale-name audit**

Run a semantic check for current-identity leaks:

```bash
# Use Lumen semantic search first for conceptual code discovery.
# Then use exact literal checks for audit completeness.
```

Search concepts:

```text
current app identity still named rustarr in source config plugin runtime docs
```

- [ ] **Step 2: Run exact literal audit**

Run:

```bash
git grep -n -e 'rustarr' -e 'Rustarr' -e 'RUSTARR' -- . \
  ':!docs/sessions/**' \
  ':!docs/superpowers/plans/2026-05-23-rustarr-fleet-mcp.md' \
  ':!docs/superpowers/plans/2026-06-13-rustarr-full-live-test-matrix.md' \
  ':!target/**' \
  > /tmp/yarr-stale-name-audit.txt || true
```

Classify every remaining match as one of:

```text
historical: changelog/session/old plan record
upstream: legitimate third-party/service name or fixture URL
bug: current app identity leak to fix now
```

- [ ] **Step 3: Fix every `bug` classification**

For each bug row, update the file and add a test if it is an executable surface. Re-run the exact literal audit until no current app identity leaks remain.

- [ ] **Step 4: Run full verification**

Run:

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo test --test plugin_contract
cargo test --test template_invariants
cargo run -- --help
cargo run -- --version
bd swarm validate rustarr-pq2
```

If the live stack is available, also run:

```bash
YARR_BIN=target/release/yarr cargo xtask live --suite smoke
cargo xtask live --suite all
```

- [ ] **Step 5: Update Beads with evidence**

Run:

```bash
bd comments add rustarr-pq2.5 "VERIFICATION: cargo fmt --check; cargo test; cargo clippy -- -D warnings; plugin_contract; template_invariants; yarr help/version; stale-name audit completed."
bd close rustarr-pq2.1 rustarr-pq2.2 rustarr-pq2.3 rustarr-pq2.4 rustarr-pq2.5
bd close rustarr-pq2
```

Only close beads after the matching work is genuinely done.

- [ ] **Step 6: Commit final audit fixes**

Run:

```bash
git add .
git commit -m "test: verify yarr rebrand"
```

## Self-Review

**Spec coverage:** The plan covers core package/binary/env/scopes/resource identity, npm package `yarr-mcp`, plugin manifests and hooks, no-MCP generator scripts, live harness/install/runtime proof, destructive override safety, active docs/changelog, internal app-owned Rust symbols/modules, and final audit.

**Placeholder scan:** No task uses TBD/TODO/fill-in placeholders. Each task includes exact files, commands, expected outcomes, and concrete examples.

**Type consistency:** The canonical names are consistent across tasks: npm package `yarr-mcp`; all other public/runtime identity `yarr`, `YARR_*`, `yarr:*`, `yarr://schema/mcp-tool`, `plugins/yarr`, and binary `yarr`.
