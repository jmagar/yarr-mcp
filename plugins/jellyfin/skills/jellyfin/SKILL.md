---
name: jellyfin
description: "This skill should be used when the user asks about Jellyfin media server. Triggers include: \"check Jellyfin\", \"Jellyfin library\", \"who's watching on Jellyfin\", \"active Jellyfin sessions\", \"add a Jellyfin user\", \"Jellyfin metadata\", \"Jellyfin transcoding\", \"Jellyfin health\", \"Jellyfin scheduled tasks\", \"Jellyfin plugins\", or any mention of Jellyfin media server management."
---

# Jellyfin

Use this skill for Jellyfin media server workflows. Prefer an available Lab or
MCP Jellyfin integration when one is present; otherwise work through the
Jellyfin HTTP API, Docker/container inspection, or server logs.

## Workflow

1. Identify the target server, auth source, and scope of change. Do not assume a
   default server URL or admin token; ask for missing credentials instead of
   searching broadly.
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
  `JELLYFIN_URL` and `JELLYFIN_API_KEY` from this plugin config or
  `~/.lab/.env`, keeps the token out of output, and provides commands
  such as `info`, `users`, `sessions`, `libraries`, `search`, `item`, `tasks`,
  and `devices` (all read-only), plus `refresh` — a **write** (POST) that
  triggers a library/metadata refresh.
- Common REST roots are `/System/Info`, `/Users`, `/Sessions`,
  `/Library/VirtualFolders`, `/Items`, `/ScheduledTasks`, and `/Devices`.
- Jellyfin typically accepts API keys through `X-Emby-Token` or
  `Authorization: MediaBrowser Token="<token>"` depending on the client path.
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
