# Integrations

## Vendored OpenAPI metadata

`specs/` contains Sonarr, Radarr, Prowlarr, Overseerr, Plex, and Jellyfin specs.
Regenerate runtime tables with:

```bash
cargo xtask gen-openapi
cargo test openapi
```

Generated modules export `OPERATIONS` and `TYPES` tables. They do not export a
Rust function per upstream operation. `src/app/openapi_ops.rs` is the one
generic executor.

The current metadata preserves method/path and path, query, header, and cookie
parameters with requiredness, schema, and OpenAPI style/explode serialization.
It also preserves supported JSON, form, multipart, text/XML, and binary request
representations and negotiates successful JSON, text, and binary responses.
Operations that cannot be represented losslessly are omitted and reported by
the runtime-derived matrix in `docs/TOOLS_ACTIONS_ENDPOINTS.md`. See
`docs/API.md` for the exact boundary.

## Repository automation

Actual xtask commands are:

```text
dist, ci, symlink-docs, check-env, gen-openapi, live, patterns,
tool-docs, check-test-siblings
```

There is no `cargo xtask live-contracts`. Use
`cargo xtask live --suite contract` or `--suite mcporter`.

## MCP clients

Local clients should spawn `yarr mcp`. Shared HTTP clients connect to `/mcp`
and authenticate with a read-only static bearer token or OAuth. Destructive
HTTP MCP dispatch requires `yarr:write` plus elicitation approval; a client
without elicitation support is denied.

## Plugins

The full `yarr` plugin and 11 service-specific fallback plugins are distribution
packages, not an extension API for adding runtime Rust actions. Plugin hooks
translate strict manifest-approved settings into mode-0600 per-service config
JSON and invoke the npm launcher; they do not add arbitrary middleware, auth
providers, or Code Mode functions. See `docs/PLUGINS.md` and executable plugin
manifests for the current host-specific contract.

## Reverse proxies

Proxy `/mcp` without stripping Streamable HTTP semantics and pass the external
Host expected by `YARR_MCP_ALLOWED_HOSTS`. If a trusted gateway performs all
auth and authorization, set `YARR_NOAUTH=true` only when direct access to yarr
is blocked. Public probes/metrics need their own network policy.
