---
name: rustarr
description: Use when the user wants to inspect or automate the Arr/media stack through rustarr, including Sonarr, Radarr, Prowlarr, Tautulli, Overseerr, Bazarr, Tracearr, Lidarr, Readarr, SABnzbd, qBittorrent, Wizarr, Notifiarr, Plex, or Jellyfin.
---

# rustarr

Use the `rustarr` MCP tool for media automation stack operations. Prefer high-level read actions first, and avoid passing secrets in paths or tool arguments.

## Actions

| Action | Use When | Required Arguments |
|---|---|---|
| `integrations` | The user asks what media services are supported or configured | none |
| `service_status` | The user asks whether a configured service is reachable or healthy | `service` |
| `api_get` | The user asks to inspect upstream data from a service | `service`, `path` |
| `api_post` | The user asks to trigger an upstream command or mutation | `service`, `path`, `body` |
| `help` | The user asks what rustarr can do | none |

## Examples

```text
mcp__rustarr__rustarr(action="integrations")
mcp__rustarr__rustarr(action="service_status", service="radarr")
mcp__rustarr__rustarr(action="api_get", service="sonarr", path="/api/v3/system/status")
mcp__rustarr__rustarr(action="api_post", service="radarr", path="/api/v3/command", body={"name":"RefreshMovie"})
```

## Safety

- Do not include API keys, tokens, or passwords in `path`.
- Use configured service names, not arbitrary URLs.
- Use `api_get` for inspection before `api_post` for mutation.
- If an upstream API needs a destructive command, explain the likely effect before calling it.
