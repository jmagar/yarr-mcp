---
title: "CI"
doc_type: "guide"
status: "active"
owner: "yarr"
audience:
  - "contributors"
  - "agents"
scope: "project"
source_of_truth: false
last_reviewed: "2026-07-16"
---

# CI

CI mirrors local quality gates so failures are reproducible before pushing.

## Local CI commands

```bash
just verify
just template-check
scripts/pre-release-check.sh
```

`just ci` delegates to `cargo xtask ci`, which runs formatting, clippy, tests, TOML checks, pattern checks, and audit when supporting tools are installed.

## GitHub workflows

The repository separates PR gates, maintenance, documentation, containers, and
staged releases. `main` rules require the complete CI and MSRV check set and a
current branch before merge; direct/force pushes are blocked.
Repository Actions policy also requires immutable commit-SHA references for
third-party actions; readable version comments remain beside each pin.

### `.github/workflows/ci.yml`

Runs on push/PR to main:
- `fmt`: `cargo fmt -- --check`
- `clippy`: `cargo clippy -- -D warnings`
- `docs`: rustdoc and generated action/endpoint documentation checks
- `test`: `cargo nextest run --profile ci`
- `actionlint`: workflow syntax and embedded shell validation
- `npm`: launcher/package tests and dry-run pack
- `toml`: `taplo check`
- `template`: repository contracts, patterns, package/plugin layout, and test siblings
- `deny`: `cargo deny check`
- `gitleaks`: secret scanning

`.github/workflows/msrv.yml` supplies the separately required
`Minimum Supported Rust Version (1.90)` check.

The `Cargo Deny` job first runs `scripts/check-security-exceptions.sh`.
The reviewed RSA exception expires on 2026-10-01; CI fails closed on that date
even if cargo-deny would otherwise accept the ignore entry. The Scheduled
workflow applies the same ordering.

Dependabot patch/minor PRs use a pinned, no-checkout `pull_request_target`
workflow because Dependabot `pull_request` runs cannot read the PAT/App token.
It never executes PR code, waits for every required check, then squash-merges
with the existing `RELEASE_PLEASE_TOKEN` so the resulting push triggers trusted
main workflows. Major updates remain manual.

### `.github/workflows/docker-publish.yml`

Runs after successful main CI and for release tags:
- Build linux/amd64 to a source-SHA quarantine tag
- Attach SBOM/provenance and scan the immutable digest with Trivy
- Promote `main`/`latest` or semver tags only after a clean scan
- Create/deduplicate a repository incident issue on publication failure

### `.github/workflows/release.yml`

Runs on version tags (`v*`), while release-please keeps the GitHub Release draft:
- Verify Cargo/npm/registry versions all match the tag
- Build checksummed linux/amd64 and windows/amd64 archives
- Stage assets on the draft release
- Verify or publish the exact npm launcher version
- Publish the GitHub Release only after every required asset/package exists
- Leave the release draft and emit a recovery issue when a stage fails

## nextest configuration

CI uses `cargo nextest` with a dedicated profile in `.config/nextest.toml`:

```toml
[profile.default]
fail-fast = false

[profile.ci]
fail-fast = true
retries = 2
```

## Release gate

`scripts/pre-release-check.sh` runs:

1. `cargo xtask patterns`
2. plugin layout validation
3. schema docs validation
4. template feature smoke tests
5. version sync
6. blob-size check
7. ASCII hygiene
8. `just verify`

Use `--mcporter` when a server is running and live MCP integration should be included.

## TOML formatting

All repos require `taplo` for TOML formatting:

```bash
taplo format     # format
taplo check      # CI check
```

Install: `cargo install taplo-cli` or `mise use taplo`.

`taplo.toml`:
```toml
[formatting]
align_entries = false
array_trailing_comma = true
array_auto_expand = true
array_auto_collapse = true
compact_arrays = true
compact_inline_tables = false
column_width = 100
indent_string = "  "
trailing_newline = true
allowed_blank_lines = 1
```

## Blob policy

Large artifacts are blocked unless explicitly justified in
`scripts/blob-size-allowlist.txt`. Plugins use the pinned npm launcher rather
than committing a platform-specific runtime binary.

## Release artifact distribution

Version tags (`v*`) trigger the release workflow, which builds release binaries and attaches them to the GitHub Release. The release workflow must **not** push generated binaries back to `main`. Local `just dist` / `cargo xtask dist` recipes are operator conveniences for preparing artifacts — they are not a CI write-back path.

Release asset names are `yarr-x86_64.tar.gz` and
`yarr-windows-x86_64.tar.gz`, each with a `.sha256` file plus aggregate
`SHA256SUMS`.

## CHANGELOG.md

Every repo keeps a `CHANGELOG.md` following [Keep a Changelog](https://keepachangelog.com/):

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-05-13

### Added
- Initial release
- MCP server with action-based tool dispatch
- CLI thin shim
- Bearer token + Google OAuth authentication
- Streamable HTTP + stdio transport
- Claude Code plugins with a pinned npm stdio launcher
- Strict mode-0600 JSON plugin configuration; no sourced shell config
```

Update `[Unreleased]` with every meaningful change. The release workflow promotes it to a versioned section on tag.

See `docs/PATTERNS.md` §21, §24, §29, §31, §34 for release artifacts, nextest, taplo, GitHub workflow, and changelog patterns.
