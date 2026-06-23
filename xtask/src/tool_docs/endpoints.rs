//! Endpoint mappings for the curated capabilities that still ship hand-written
//! commands: Stats (tautulli) and DownloadClient (sabnzbd/qbittorrent). The 6
//! spec-backed services (sonarr/radarr/prowlarr/overseerr/jellyfin/plex) have no
//! curated commands — their surface is the generated OpenAPI operations, reached
//! via the `op` action / `codemode.search` (see the generated tables under
//! `src/openapi/generated/`), so there is nothing to map here.

#[derive(Debug, Clone, Copy)]
pub(super) struct EndpointRow {
    pub(super) action: &'static str,
    pub(super) tools: &'static str,
    pub(super) endpoint: &'static str,
    pub(super) notes: &'static str,
}

pub(super) const STATS_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "stats_activity",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=get_activity`",
        notes: "",
    },
    EndpointRow {
        action: "stats_history",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=get_history[&start=&length=&user=]`",
        notes: "",
    },
    EndpointRow {
        action: "stats_users",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=get_users`",
        notes: "",
    },
    EndpointRow {
        action: "stats_libraries",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=get_library_names`",
        notes: "",
    },
    EndpointRow {
        action: "stats_refresh_libraries",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=refresh_libraries_list`",
        notes: "Runs immediately (not destructive).",
    },
    EndpointRow {
        action: "stats_refresh_users",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=refresh_users_list`",
        notes: "Runs immediately (not destructive).",
    },
    EndpointRow {
        action: "stats_delete_image_cache",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=delete_image_cache`",
        notes: "Destructive: gated by MCP elicitation / CLI `--confirm`.",
    },
];

pub(super) const DOWNLOAD_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "download_queue",
        tools: "sabnzbd",
        endpoint: "`GET /api?mode=queue&output=json`",
        notes: "qBittorrent uses `GET /api/v2/torrents/info`.",
    },
    EndpointRow {
        action: "download_add",
        tools: "sabnzbd",
        endpoint: "`GET /api?mode=addurl&name=<url>&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/add` with `urls=<url>`. Runs immediately.",
    },
    EndpointRow {
        action: "download_pause",
        tools: "sabnzbd",
        endpoint: "one: `GET /api?mode=queue&name=pause&value=<id>&output=json`; all: `GET /api?mode=pause&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/stop` with `hashes=<hash-or-all>`. Runs immediately.",
    },
    EndpointRow {
        action: "download_resume",
        tools: "sabnzbd",
        endpoint: "one: `GET /api?mode=queue&name=resume&value=<id>&output=json`; all: `GET /api?mode=resume&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/start` with `hashes=<hash-or-all>`. Runs immediately.",
    },
    EndpointRow {
        action: "download_remove",
        tools: "sabnzbd",
        endpoint: "`GET /api?mode=queue&name=delete&value=<id>[&del_files=1]&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/delete` with `hashes=<hash>` and `deleteFiles={true|false}`. Destructive: gated by MCP elicitation / CLI `--confirm`.",
    },
];
