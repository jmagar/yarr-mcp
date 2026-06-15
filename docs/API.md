# rustarr API

`rustarr` exposes one MCP tool named `rustarr` and equivalent CLI commands. Both surfaces call `RustarrService`.

## MCP Tool

Tool name: `rustarr`

Arguments:

| Field | Type | Required | Notes |
|---|---|---:|---|
| `action` | string | yes | A generic action (`integrations`, `service_status`, `api_get`, `api_post`, `api_put`, `api_delete`, `help`) or any curated command name from the registry (see `rustarr help`) |
| `service` | string | action-dependent | Configured service name such as `sonarr` or `radarr` |
| `path` | string | action-dependent | Relative upstream API path (generic passthrough actions) |
| `body` | object | no | JSON body forwarded to the upstream service for `api_post`/`api_put`; defaults to `{}` |
| `confirm` | boolean | mutating actions | Must be `true` for `api_post`/`api_put`/`api_delete` and other mutating commands |

Curated commands take their own typed params (e.g. `query`, `from`, `to`, `id`,
`hash`, `media_type`, `media_id`). The action set is **registry-derived** and
broader than the generic actions — run the `help` action (or `rustarr help`) for
the current full list and per-action params.

Examples:

```json
{"action":"integrations"}
{"action":"service_status","service":"radarr"}
{"action":"api_get","service":"sonarr","path":"/api/v3/system/status"}
{"action":"api_post","service":"radarr","path":"/api/v3/command","body":{"name":"RefreshMovie"},"confirm":true}
{"action":"api_put","service":"sonarr","path":"/api/v3/series/editor","body":{"seriesIds":[1],"qualityProfileId":4},"confirm":true}
{"action":"api_delete","service":"radarr","path":"/api/v3/movie/12","confirm":true}
{"action":"list","service":"sonarr"}
{"action":"set_quality","service":"sonarr","from":"Ultra-HD","to":"HD-1080p","confirm":true}
{"action":"stats_activity","service":"tautulli"}
```

## CLI Parity

The CLI is **service-grouped** (`rustarr <service> <command> [flags]`); there is
no `--service` flag. Infra commands (`integrations`, `help`) are service-less.

```bash
rustarr integrations
rustarr radarr status
rustarr sonarr get --path /api/v3/system/status
rustarr radarr post --path /api/v3/command --body '{"name":"RefreshMovie"}' --confirm
rustarr sonarr put --path /api/v3/series/editor --body '{"seriesIds":[1],"qualityProfileId":4}' --confirm
rustarr radarr delete --path /api/v3/movie/12 --confirm
rustarr help

# curated commands
rustarr sonarr list
rustarr sonarr set-quality --from "Ultra-HD" --to "HD-1080p" --confirm
rustarr tautulli activity
```

CLI ↔ MCP parity is mechanically enforced by `tests/parity.rs`; every curated
command is reachable from both surfaces.

## Security Rules

- `help` has no action scope, but mounted HTTP transports still require bearer/OAuth transport auth.
- Read actions require `rustarr:read`.
- `api_get`, `api_post`, `api_put`, and `api_delete` require `rustarr:write` because generic credentialed upstream calls can mutate some services.
- `rustarr:write` satisfies read.
- Paths with traversal or embedded query-string secrets are rejected.
- Responses are capped by the shared token-limit layer before being returned to MCP clients.
