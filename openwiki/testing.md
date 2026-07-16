# Testing

## Local gates

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo nextest run --profile ci
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps
cargo xtask patterns
cargo xtask check-test-siblings
cargo xtask tool-docs --check
python3 scripts/check-schema-docs.py --check
```

Tests for private Rust modules live in sibling `_tests.rs` files. Repository
integration tests are `tests/cli_parse.rs`, `tests/tool_dispatch.rs`,
`tests/parity.rs`, `tests/plugin_contract.rs`, and
`tests/template_invariants.rs`. There is no `tests/e2e_tests.rs`,
`tests/live_tests.rs`, or `docs/contracts/` output tree.

## Live testing

The guarded disposable-stack harness is `cargo xtask live`. Valid suites are:

```text
guard, cli, rest, mcp, mcporter, services, contract, lifecycles,
all, coverage-check, coverage-write
```

```bash
cargo xtask live --suite guard
cargo xtask live --suite contract
cargo xtask live --suite mcporter
cargo xtask live --suite all
```

It requires `YARR_HOME=/home/jmagar/.yarr-shart`, all 11 service kinds, and the
disposable shart host allowlist. Never point it at the normal `~/.yarr` config
or production upstreams.

The contract suite verifies the generated executor's supported transport
behavior. Generator tests separately prove that unsupported operations are
excluded with explicit reasons in the runtime-derived capability matrix.
Assertions must check payload meaning, failure class, or before/after state
rather than only a successful status.
