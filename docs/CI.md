---
title: "CI"
doc_type: "guide"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
last_reviewed: "2026-05-15"
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

Three workflows cover CI, Docker publishing, and releases:

### `.github/workflows/ci.yml`

Runs on push/PR to main:
- `fmt`: `cargo fmt -- --check`
- `clippy`: `cargo clippy -- -D warnings`
- `test`: `cargo nextest run --profile ci`
- `web`: `pnpm install --frozen-lockfile`, `pnpm audit`, `pnpm lint`, `pnpm build`
- `toml`: `taplo check`
- `deny`: `cargo deny check`
- `gitleaks`: secret scanning

### `.github/workflows/docker-publish.yml`

Runs on push to main + tags:
- Multi-platform build (linux/amd64, linux/arm64)
- Push to `ghcr.io/jmagar/<repo>:latest` on main, `:<version>` on tags
- Trivy vulnerability scan
- SBOM generation
- MCP registry publish on version tags

### `.github/workflows/release.yml`

Runs on version tags (`v*`):
- Build release binaries for linux/amd64 and linux/arm64
- Create GitHub Release with binary assets
- Update `install.sh` download URLs

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
9. `just build-plugin`

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

Large artifacts are blocked unless allowlisted in `scripts/blob-size-allowlist.txt`. Plugin binaries are expected artifacts and are allowlisted.

## Release artifact distribution

Version tags (`v*`) trigger the release workflow, which builds release binaries and attaches them to the GitHub Release. The release workflow must **not** push generated binaries back to `main`. Local `just dist` / `cargo xtask dist` recipes are operator conveniences for preparing artifacts — they are not a CI write-back path.

Binary naming convention: `<service>-<version>-<arch>-unknown-linux-musl.tar.gz` (e.g. `rustarr-v0.2.0-x86_64-unknown-linux-musl.tar.gz`).

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
- Thin plugin setup hook plus binary-owned setup/repair
- Claude Code plugin with userConfig
```

Update `[Unreleased]` with every meaningful change. The release workflow promotes it to a versioned section on tag.

See `docs/PATTERNS.md` §21, §24, §29, §31, §34 for release artifacts, nextest, taplo, GitHub workflow, and changelog patterns.
