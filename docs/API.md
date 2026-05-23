# rustarr API

`rustarr` exposes one MCP tool named `rustarr`, one REST action endpoint at `/v1/rustarr`, and equivalent CLI commands. All three surfaces call `RustarrService`.

## MCP Tool

Tool name: `rustarr`

Arguments:

| Field | Type | Required | Notes |
|---|---|---:|---|
| `action` | string | yes | One of `integrations`, `service_status`, `api_get`, `api_post`, `help` |
| `service` | string | action-dependent | Configured service name such as `sonarr` or `radarr` |
| `path` | string | action-dependent | Relative upstream API path |
| `body` | object | `api_post` | JSON body forwarded to the upstream service |

Examples:

```json
{"action":"integrations"}
{"action":"service_status","service":"radarr"}
{"action":"api_get","service":"sonarr","path":"/api/v3/system/status"}
{"action":"api_post","service":"radarr","path":"/api/v3/command","body":{"name":"RefreshMovie"}}
```

## CLI Parity

```bash
rustarr integrations
rustarr status --service radarr
rustarr get --service sonarr --path /api/v3/system/status
rustarr post --service radarr --path /api/v3/command --body '{"name":"RefreshMovie"}'
rustarr help
```

## REST Endpoint

`POST /v1/rustarr`

```json
{
  "action": "api_get",
  "params": {
    "service": "sonarr",
    "path": "/api/v3/system/status"
  }
}
```

The REST endpoint uses the same auth policy as the HTTP MCP endpoint. Loopback no-auth is for local development only.

## Security Rules

- `help` is public.
- Read actions require `rustarr:read`.
- `api_post` requires `rustarr:write`.
- `rustarr:write` satisfies read.
- Paths with traversal or embedded query-string secrets are rejected.
- Responses are capped by the shared token-limit layer before being returned to MCP clients.
