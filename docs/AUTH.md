# Authentication

This server supports two authentication mechanisms simultaneously: **static bearer tokens** and **OAuth 2.0**. They serve different audiences and can be active at the same time.

---

## Why two mechanisms?

**Bearer tokens** are for agents and automation. An agent sets `Authorization: Bearer <token>` and makes calls. No browser, no redirect flow, no session cookie — just a shared secret. Tokens are fast to issue (`just gen-token`) and easy to rotate.

**OAuth** is for humans. It runs a full browser-based Google OAuth flow, issues short-lived JWTs, and maintains refresh tokens. This is the right choice when a human user needs to grant access through a UI without ever seeing a raw token.

When both are configured, each request is accepted if it satisfies either mechanism. A human signs in via OAuth; an agent uses a token. They share the same server.

---

## Scopes

All non-trivial actions require at least `yarr:read`. Mutating actions require `yarr:write`, which also satisfies read checks. The `help` action is always public.

Static bearer tokens default to `yarr:read` only. OAuth tokens carry whatever scopes the OAuth flow issued.

---

## Configuring bearer token auth

```bash
# Generate a token
export YARR_MCP_TOKEN=$(openssl rand -hex 32)

# Or: just gen-token
```

Set `YARR_MCP_TOKEN` in your environment or `.env` file. Clients authenticate with:

```
Authorization: Bearer <token>
```

That's all. The server validates the header on every request to `/mcp`.

---

## Configuring OAuth

Set the following environment variables:

```bash
YARR_MCP_AUTH_MODE=oauth
YARR_MCP_PUBLIC_URL=https://your-server.yarr.com   # public URL for OAuth callbacks
YARR_MCP_GOOGLE_CLIENT_ID=...
YARR_MCP_GOOGLE_CLIENT_SECRET=...
YARR_MCP_AUTH_ADMIN_EMAIL=you@yarr.com
```

The server exposes standard OAuth discovery endpoints under `/mcp/.well-known/` that MCP clients can use for dynamic registration. Session cookies are disabled — all auth is via `Authorization` headers.

OAuth and bearer token can coexist: set both `YARR_MCP_TOKEN` and the OAuth variables. To disable bearer tokens while OAuth is active, set `disable_static_token_with_oauth = true` under `[mcp.auth]` in `config.toml` (this is a config file field, not an environment variable).

---

## The startup guard

**The HTTP server will refuse to start if it is binding to a non-loopback address with no authentication configured.**

This is enforced by `server::resolve_auth_policy_kind()`. The exact error:

```
Refusing to bind MCP server to 0.0.0.0 without authentication.

Choose one of:
1. Bind to loopback:    YARR_MCP_HOST=127.0.0.1
2. Set a bearer token:  YARR_MCP_TOKEN=$(openssl rand -hex 32)
3. Enable OAuth:        YARR_MCP_AUTH_MODE=oauth (+ OAuth credentials)
4. Disable auth:        YARR_MCP_HOST=127.0.0.1 YARR_MCP_NO_AUTH=true
5. Upstream gateway:    YARR_NOAUTH=true  (if a proxy handles auth)
```

The guard passes when any of the following is true:

| Condition | Variable | Notes |
|---|---|---|
| Loopback bind | `YARR_MCP_HOST=127.0.0.1` | Trust boundary is the network address |
| Bearer token set | `YARR_MCP_TOKEN=<token>` | Auth middleware enforces it |
| OAuth enabled | `YARR_MCP_AUTH_MODE=oauth` | Auth middleware enforces it |
| Auth disabled | `YARR_MCP_HOST=127.0.0.1` + `YARR_MCP_NO_AUTH=true` | Local dev — see below |
| Gateway override | `YARR_NOAUTH=true` | Upstream handles auth — see below |

---

## Local development (no auth)

For local development, disable auth entirely:

```bash
just dev
# equivalent to: YARR_MCP_HOST=127.0.0.1 YARR_MCP_NO_AUTH=true cargo run -- serve mcp
```

`YARR_MCP_NO_AUTH=true` is accepted only on a loopback bind. It sets the auth policy to `LoopbackDev`, removes the auth middleware, and requires no token for local calls.

**Do not use this in production.**

---

## Upstream gateway / MCP proxy (no server-level auth)

If you deploy behind a gateway that handles authentication for all services (e.g. an MCP proxy that validates tokens before routing to this server), you can disable auth at the server level:

```bash
YARR_NOAUTH=true         # acknowledge the startup guard that an upstream gateway handles auth
```

`YARR_NOAUTH=true` selects the explicit `TrustedGatewayUnscoped` policy. It removes the local auth middleware and scope checks, so only use it when a trusted upstream gateway enforces both authentication and authorization before traffic reaches this server.

---

## Stdio transport

The stdio transport (`yarr mcp`) bypasses all HTTP auth entirely. It is always `LoopbackDev` — the trust boundary is the OS pipe between parent and child process. Scope checks are not enforced in stdio mode. This matches the MCP spec: stdio servers are local, trusted, subprocess connections.

---

## Auth policy reference

The `AuthPolicy` enum in `src/server.rs` controls what the router does:

| Policy | When | Auth enforced? | Scope checks? |
|---|---|---|---|
| `LoopbackDev` | Loopback bind, or stdio mode. `YARR_MCP_NO_AUTH=true` also enables this policy for loopback development. | No | No |
| `TrustedGatewayUnscoped` | Non-loopback no-auth deployment with `YARR_NOAUTH=true` | No | No |
| `Mounted { auth_state: None }` | Bearer-only mode | Yes (token) | Yes |
| `Mounted { auth_state: Some(_) }` | OAuth mode (+ optional token) | Yes (OAuth / token) | Yes |

Public endpoints (`/health`, `/status`) are never gated by auth, regardless of policy. `/status` returns only local redacted runtime metadata.

---

## TEMPLATE

When you adapt this template, replace all `YARR_` prefixes with your service's prefix throughout `src/config.rs`, `src/main.rs`, and this document.
