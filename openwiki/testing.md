# Testing

yarr uses a multi-layer testing strategy: unit tests, integration tests, parity enforcement, and live contract testing against real upstream services.

## Running tests

### All tests

```bash
# Run all tests
cargo test

# Run with nextest (faster, better output)
cargo nextest run

# CI profile (same order as GitHub Actions)
cargo xtask ci
# or
just ci
```

### Specific test modules

```bash
# Config tests
cargo test config_tests

# Action registry tests
cargo test actions_tests

# CLI tests
cargo test cli_tests

# MCP tests
cargo test mcp_tests
```

### Test utilities

```bash
# Check environment before tests
cargo xtask check-env

# Pattern checks (architecture rules)
cargo xtask patterns

# Live contract tests (requires real services)
cargo xtask live-contracts
```

## Test structure

### Unit tests

Unit tests live in `*_tests.rs` files alongside the source:

```
src/
  config.rs           → config_tests.rs
  actions.rs          → actions_tests.rs
  actions/registry.rs → actions/registry_tests.rs
  app.rs              → app_tests.rs
  cli.rs              → cli_tests.rs
  mcp.rs              → mcp_tests.rs
  # etc.
```

Example from `config_tests.rs`:

```rust
#[test]
fn test_load_empty_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("YARR_SERVICES", "");
    let config = Config::load().unwrap();
    assert!(config.services.is_empty());
}
```

### Integration tests

Full-stack integration tests live in `/tests/`:

```
tests/
  parity.rs      ← MCP ↔ CLI behavioral parity enforcement
  e2e_tests.rs   ← End-to-end scenarios
  live_tests.rs  ← Tests against real upstream services (if available)
```

## Parity tests

`tests/parity.rs` enforces **MCP ↔ CLI behavioral parity** — every action should behave identically whether called via MCP tool or CLI command.

### How it works

For each action:

1. Call via MCP (JSON over stdio or HTTP)
2. Call via CLI (argv parsing)
3. Compare outputs (modulo formatting differences)
4. Fail if behavior diverges

### Running parity tests

```bash
cargo test parity
```

### Adding new actions

When adding a new action:

1. Implement in `src/actions/registry.rs`
2. Add MCP dispatch in `src/mcp/tools.rs`
3. Add CLI dispatch in `src/cli.rs`
4. Add parity case in `tests/parity.rs`

## Live contract testing

Live contracts test against real upstream services. They're run via `xtask` and generate fixture contracts for regression testing.

### Running live tests

```bash
# Requires configured services in .env
cargo xtask live-contracts

# Individual service tests
cargo xtask live-contracts -- radarr
cargo xtask live-contracts -- sonarr
```

### Contract structure

Live contracts are generated in `/docs/contracts/`:

```
docs/contracts/
  radarr/
    get_system_status.json
    get_movie.json
    post_movie.json
  sonarr/
    get_series.json
    post_series.json
  # etc.
```

These contracts are checked into the repo and used as fixtures for regression tests.

### Live test coverage

Current coverage (from git history and `/docs/LIVE_ENDPOINT_COVERAGE.md`):

- **Radarr**: Full coverage of read/write/delete operations
- **Sonarr**: Full coverage of read/write/delete operations
- **Prowlarr**: Full coverage
- **Overseerr**: Full coverage
- **Plex/Jellyfin**: Coverage of core operations
- **Download clients**: Coverage of add/remove/status operations
- **Stats services**: Coverage of query operations

See `/docs/LIVE_ENDPOINT_COVERAGE.md` for the detailed matrix.

## Code Mode testing

Code Mode tests exercise the QuickJS sandbox and built-in callables:

```bash
cargo test codemode
```

Test areas:

- Engine lifecycle (start/stop/reuse)
- Built-in functions (`codemode.search`, `codemode.describe`)
- Per-service callables (radarr, sonarr, etc.)
- Generic passthrough (`api.get`, `api.post`)
- Artifact writing (`writeArtifact`)

## OpenAPI generation tests

Generated OpenAPI operations are tested by:

```bash
# Regenerate operations from specs
cargo xtask gen-openapi

# Verify compilation
cargo build

# Run operation tests
cargo test openapi
```

See `/xtask/src/openapi/` for the generator code.

## Environment variable testing

Tests that mutate environment variables use the `ENV_LOCK` to serialize across threads:

```rust
#[test]
fn test_env_override() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("YARR_MCP_PORT", "3000");
    let config = Config::load().unwrap();
    assert_eq!(config.mcp.port, 3000);
}
```

**Always acquire `ENV_LOCK`** before mutating env vars in tests. See `/src/lib.rs` `testing::ENV_LOCK` documentation.

## Test helpers

Public test helpers are available in `src/lib.rs`:

```rust
pub mod testing {
    pub static ENV_LOCK: Mutex<()> = Mutex::new(());

    pub fn stub_service() -> YarrService { /* ... */ }
    pub fn test_app_state() -> AppState { /* ... */ }
}
```

Use these in integration tests to construct `AppState` without real credentials.

## Pattern checking

`cargo xtask patterns` enforces architecture rules from `/docs/PATTERNS.md`:

| Check | Purpose |
|--------|---------|
| Required files | `README.md`, `config.example.toml`, etc. |
| Modern module layout | No `mod.rs` files |
| Thin shims | MCP/CLI shims < 400 lines |
| Surface parity | CLI/MCP have same action coverage |
| Plugin version rules | Plugin manifests follow versioning schema |
| Binary-owned hooks | Plugin hooks enforced by binary |
| Auth/config basics | Bearer/OAuth config valid |
| Routes | `/mcp`, `/health`, `/status` present |
| Tooling | `just ci`, `cargo xtask` commands available |

```bash
# Standard check
cargo xtask patterns

# Strict mode (fail on warnings)
cargo xtask patterns --strict

# JSON output (for CI dashboards)
cargo xtask patterns --json
```

## CI test pipeline

The GitHub Actions workflow runs tests in this order (see `.github/workflows/ci.yml`):

1. `cargo fmt --check` — Format check
2. `cargo clippy -- -D warnings` — Lint
3. `cargo nextest run --profile ci` — Tests (or `cargo test` if nextest unavailable)
4. `taplo check` — TOML format (if `taplo` installed)
5. `cargo xtask patterns` — Pattern checks
6. `cargo audit` — Security audit (if `cargo-audit` installed)

Run the same locally with:

```bash
cargo xtask ci
# or
just ci
```

## Coverage

### What's tested

- ✅ Config loading and env override
- ✅ Action registry and curated commands
- ✅ CLI parsing and dispatch
- ✅ MCP tool dispatch
- ✅ OpenAPI operation generation
- ✅ Code Mode engine and built-ins
- ✅ HTTP transport and auth
- ✅ MCP ↔ CLI parity
- ✅ Live upstream contracts

### What's NOT tested

- ❌ Upstream service behavior (assume upstream works)
- ❌ Full OAuth flow (requires real Google callback)
- ❌ Cross-platform binary edge cases (assume Rust handles it)

## Adding tests

### Unit test for a new function

```rust
// src/my_module.rs
pub fn my_helper(input: &str) -> String {
    input.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_helper() {
        assert_eq!(my_helper("hello"), "HELLO");
    }
}
```

### Integration test for a new action

1. Add action in `src/actions/registry.rs`
2. Implement in `src/app.rs`
3. Wire in `src/mcp/tools.rs` and `src/cli.rs`
4. Add parity case in `tests/parity.rs`

### Live contract for a new operation

1. Add operation to `/xtask/src/live/contract/synth.rs`
2. Run `cargo xtask live-contracts`
3. Commit generated fixture to `/docs/contracts/`

## Troubleshooting tests

### Flaky tests

If tests are flaky:

1. Check for missing `ENV_LOCK` acquisition in env-mutating tests
2. Check for non-deterministic ordering or timing dependencies
3. Run with `cargo test -- --test-threads=1` to serialize

### Tests fail locally but pass in CI

- Check for environment-specific assumptions (paths, credentials)
- Verify you have the same Rust version as CI (`rust-toolchain.toml`)
- Check for conditional compilation (`#[cfg(test)]` vs feature flags)

### "No such file or directory" errors

- Ensure you're running from the repo root
- Check that vendored specs exist in `/specs/`
- Verify `Cargo.toml` workspace members are correct

## Further reading

- `/docs/TESTING.md` — Original testing documentation
- `/docs/LIVE_ENDPOINT_COVERAGE.md` — Live contract coverage matrix
- `/docs/PATTERNS.md` — Architecture rules enforced by pattern checks
