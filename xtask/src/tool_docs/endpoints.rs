//! Endpoint mappings for the curated capabilities that still ship hand-written
//! commands: Stats (tautulli), DownloadClient (sabnzbd/qbittorrent), Subtitles
//! (bazarr), and Trace (tracearr). The 6
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
        notes: "Runs immediately; destructive, so MCP elicits the connected client for confirmation before dispatch.",
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
        notes: "qBittorrent uses form `POST /api/v2/torrents/delete` with `hashes=<hash>` and `deleteFiles={true|false}`. Runs immediately; destructive, so MCP elicits the connected client for confirmation before dispatch.",
    },
];

pub(super) const SUBTITLES_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "subtitles_status",
        tools: "bazarr",
        endpoint: "`GET /api/system/status`",
        notes: "",
    },
    EndpointRow {
        action: "subtitles_movies",
        tools: "bazarr",
        endpoint: "`GET /api/movies[?start=&length=]`",
        notes: "",
    },
    EndpointRow {
        action: "subtitles_episodes",
        tools: "bazarr",
        endpoint: "`GET /api/episodes[?start=&length=]`",
        notes: "",
    },
    EndpointRow {
        action: "subtitles_wanted_episodes",
        tools: "bazarr",
        endpoint: "`GET /api/episodes/wanted[?start=&length=]`",
        notes: "",
    },
    EndpointRow {
        action: "subtitles_wanted_movies",
        tools: "bazarr",
        endpoint: "`GET /api/movies/wanted[?start=&length=]`",
        notes: "",
    },
    EndpointRow {
        action: "subtitles_providers",
        tools: "bazarr",
        endpoint: "`GET /api/providers`",
        notes: "",
    },
    EndpointRow {
        action: "subtitles_languages",
        tools: "bazarr",
        endpoint: "`GET /api/system/languages`",
        notes: "",
    },
];

pub(super) const TRACE_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "trace_health",
        tools: "tracearr",
        endpoint: "`GET /api/v1/public/health`",
        notes: "",
    },
    EndpointRow {
        action: "trace_stats",
        tools: "tracearr",
        endpoint: "`GET /api/v1/public/stats`",
        notes: "",
    },
    EndpointRow {
        action: "trace_today",
        tools: "tracearr",
        endpoint: "`GET /api/v1/public/stats/today[?timezone=]`",
        notes: "",
    },
    EndpointRow {
        action: "trace_activity",
        tools: "tracearr",
        endpoint: "`GET /api/v1/public/activity[?period=]`",
        notes: "",
    },
    EndpointRow {
        action: "trace_streams",
        tools: "tracearr",
        endpoint: "`GET /api/v1/public/streams[?summary=true]`",
        notes: "",
    },
    EndpointRow {
        action: "trace_users",
        tools: "tracearr",
        endpoint: "`GET /api/v1/public/users[?page=&pageSize=]`",
        notes: "",
    },
    EndpointRow {
        action: "trace_violations",
        tools: "tracearr",
        endpoint: "`GET /api/v1/public/violations[?page=&pageSize=]`",
        notes: "",
    },
    EndpointRow {
        action: "trace_history",
        tools: "tracearr",
        endpoint: "`GET /api/v1/public/history[?page=&pageSize=]`",
        notes: "",
    },
    EndpointRow {
        action: "trace_terminate_stream",
        tools: "tracearr",
        endpoint: "`POST /api/v1/public/streams/{id}/terminate`",
        notes: "Optional JSON `reason`; destructive, so MCP elicits the connected client for confirmation before dispatch.",
    },
];
