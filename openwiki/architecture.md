# Architecture

Yarr has two user-facing execution surfaces: MCP and CLI. Both route through
the same action registry, dispatcher, `YarrService`, and `YarrClient`.

```text
CLI / MCP
  -> src/actions (registry, parsing, dispatch)
  -> src/app (business and curated capability logic)
  -> src/yarr (HTTP, auth, URL/response handling)
  -> upstream service
```

Configuration is split across `src/config/{mcp,auth,services}.rs`. The 11-kind
topology and capability/auth/path descriptors live in `src/capability.rs`.
Code Mode is implemented under `src/codemode/`; MCP protocol/auth/elicitation
is under `src/mcp/`; HTTP probe/metrics routing is under `src/server/`.

Generated OpenAPI files are static operation/type metadata tables under
`src/openapi/generated/`, produced by `xtask/src/gen_openapi.rs`. Execution is
generic in `src/app/openapi_ops.rs`; there are no hundreds of generated Rust
request functions.

HTTP exposes `/mcp`, `/health`, `/ready`, `/status`, `/metrics`, and OAuth routes
when configured. There is no local REST action API or embedded web app.

See `docs/ARCHITECTURE.md` for the current file map and invariants.
