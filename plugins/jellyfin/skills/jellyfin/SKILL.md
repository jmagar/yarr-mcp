---
name: jellyfin
description: "This skill should be used when the user asks about Jellyfin media server administration or troubleshooting — libraries, users, active sessions, playback/transcoding issues, scheduled tasks, plugins, or general server health. Triggers include: \"check Jellyfin\", \"Jellyfin library\", \"who's watching on Jellyfin\", \"active Jellyfin sessions\", \"add a Jellyfin user\", \"refresh Jellyfin library\", \"Jellyfin won't scan new episodes\", \"fix Jellyfin transcoding\", \"Jellyfin container down\", \"Jellyfin metadata\", \"Jellyfin health\", \"Jellyfin scheduled tasks\", \"Jellyfin plugins\", or any mention of Jellyfin media server management."
---

# Jellyfin

Use this skill for Jellyfin media server workflows. This plugin is
skills-only — it ships no MCP server. If some other installed plugin or
gateway happens to expose Jellyfin via MCP, prefer that; otherwise work
through the Jellyfin HTTP API via `scripts/jellyfin-api.sh`, Docker/container
inspection, or server logs.

## Workflow

1. Identify the target server and scope of change. `scripts/jellyfin-api.sh`
   already sources `JELLYFIN_URL`/`JELLYFIN_API_KEY` from
   `~/.config/lab-jellyfin/config.env` (populated by this plugin's setup hook)
   or `~/.lab/.env` — only ask the user for credentials if both sources come
   up empty, instead of searching broadly.
2. For read-only checks, use the Jellyfin API or available MCP tools to inspect
   server info, libraries, users, active sessions, scheduled tasks, devices, and
   logs.
3. For library or metadata problems, collect the library id/name, affected item
   ids, provider ids, scan/task status, and recent Jellyfin log lines before
   recommending refreshes or metadata edits.
4. For playback or transcoding problems, gather client, stream type, codec,
   container, bitrate, subtitle mode, hardware acceleration settings, and the
   relevant transcode/log output.
5. For user/account work, confirm the exact user and requested permission
   change before applying writes.

## API Notes

- Prefer `scripts/jellyfin-api.sh` for repeatable API checks. It loads
  `JELLYFIN_URL` and `JELLYFIN_API_KEY` from `~/.config/lab-jellyfin/config.env`
  or `~/.lab/.env`, keeps the token out of output, and provides commands
  such as `info`, `users`, `sessions`, `libraries`, `search`, `item`, `tasks`,
  and `devices` (all read-only), plus `refresh` — a **write** (POST) that
  triggers a library/metadata refresh. Delete and user-permission changes
  aren't implemented by the script — use the Jellyfin API or admin console
  directly for those.
- Common REST roots are `/System/Info`, `/Users`, `/Sessions`,
  `/Library/VirtualFolders`, `/Items`, `/ScheduledTasks`, and `/Devices`.
- `scripts/jellyfin-api.sh` always authenticates via the `X-Emby-Token` header.
  Jellyfin's API also accepts `Authorization: MediaBrowser Token="<token>"`
  for some client paths, but the script doesn't use or fall back to that
  form — if `X-Emby-Token` is rejected by a given deployment, that's a
  server-side config issue to troubleshoot, not something to work around by
  hand-rolling a different auth header.
- Treat delete, metadata rewrite, the `refresh` library/metadata scan, and
  user-permission changes as writes. Confirm with the user and summarize the
  intended object ids before executing them.

## Operational Checks

- If Jellyfin is containerized, inspect the container status, mounted config and
  media paths, network reachability, and recent logs before changing settings.
- For database, plugin, or upgrade issues, back up or identify the Jellyfin
  config/data path first.
- When no MCP or authenticated API path is available, provide the exact API or
  admin-console action the user can run rather than inventing a local command.
