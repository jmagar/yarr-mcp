#[derive(Debug, Clone, Copy)]
pub(super) struct EndpointRow {
    pub(super) action: &'static str,
    pub(super) tools: &'static str,
    pub(super) endpoint: &'static str,
    pub(super) notes: &'static str,
}

pub(super) const ARR_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "quality_profiles",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/qualityprofile`",
        notes: "",
    },
    EndpointRow {
        action: "list",
        tools: "sonarr",
        endpoint: "`GET /api/v3/series`",
        notes: "Radarr uses `GET /api/v3/movie`.",
    },
    EndpointRow {
        action: "wanted",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/wanted/missing?pageSize=50`",
        notes: "",
    },
    EndpointRow {
        action: "queue",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/queue?pageSize=50`",
        notes: "",
    },
    EndpointRow {
        action: "history",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/history?pageSize=50`",
        notes: "",
    },
    EndpointRow {
        action: "rootfolders",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/rootfolder`",
        notes: "",
    },
    EndpointRow {
        action: "health",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/health`",
        notes: "",
    },
    EndpointRow {
        action: "set_quality",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/qualityprofile`, `GET /api/v3/{series|movie}`, then `PUT /api/v3/{series|movie}/editor`",
        notes: "No write without `confirm=true`.",
    },
    EndpointRow {
        action: "search",
        tools: "sonarr/radarr",
        endpoint: "`POST /api/v3/command`",
        notes: "Radarr can batch ids; Sonarr fans out one command per id.",
    },
    EndpointRow {
        action: "refresh",
        tools: "sonarr/radarr",
        endpoint: "`POST /api/v3/command`",
        notes: "Radarr can batch ids; Sonarr fans out one command per id.",
    },
    EndpointRow {
        action: "monitor",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/{series|movie}`, then `PUT /api/v3/{series|movie}/editor`",
        notes: "Sets `monitored=true`.",
    },
    EndpointRow {
        action: "unmonitor",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/{series|movie}`, then `PUT /api/v3/{series|movie}/editor`",
        notes: "Sets `monitored=false`.",
    },
    EndpointRow {
        action: "add",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/{series|movie}/lookup?term=...`, `GET /api/v3/qualityprofile`, then `POST /api/v3/{series|movie}`",
        notes: "No write without `confirm=true`.",
    },
    EndpointRow {
        action: "delete",
        tools: "sonarr/radarr",
        endpoint: "`DELETE /api/v3/{series|movie}/{id}?deleteFiles={true|false}`",
        notes: "No delete without `confirm=true`.",
    },
];

pub(super) const INDEXER_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "indexers",
        tools: "prowlarr",
        endpoint: "`GET /api/v1/indexer`",
        notes: "",
    },
    EndpointRow {
        action: "indexer_search",
        tools: "prowlarr",
        endpoint: "`GET /api/v1/search?query=...&type=search&limit=100[&indexerIds=...]`",
        notes: "",
    },
    EndpointRow {
        action: "indexer_stats",
        tools: "prowlarr",
        endpoint: "`GET /api/v1/indexerstats`",
        notes: "",
    },
    EndpointRow {
        action: "indexer_test",
        tools: "prowlarr",
        endpoint: "all: `POST /api/v1/indexer/testall`; one: `GET /api/v1/indexer/{id}` then `POST /api/v1/indexer/test`",
        notes: "Requires `confirm=true`.",
    },
];

pub(super) const REQUEST_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "requests",
        tools: "overseerr",
        endpoint: "`GET /api/v1/request[?filter=&take=&skip=]`",
        notes: "",
    },
    EndpointRow {
        action: "request_search",
        tools: "overseerr",
        endpoint: "`GET /api/v1/search?query=...`",
        notes: "",
    },
    EndpointRow {
        action: "request_create",
        tools: "overseerr",
        endpoint: "`POST /api/v1/request`",
        notes: "Body `{mediaType, mediaId, seasons?}`. Requires `confirm=true`.",
    },
    EndpointRow {
        action: "request_approve",
        tools: "overseerr",
        endpoint: "`POST /api/v1/request/{id}/approve`",
        notes: "Requires `confirm=true` and `MANAGE_REQUESTS`.",
    },
    EndpointRow {
        action: "request_decline",
        tools: "overseerr",
        endpoint: "`POST /api/v1/request/{id}/decline`",
        notes: "Requires `confirm=true` and `MANAGE_REQUESTS`.",
    },
];

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
        notes: "Requires `confirm=true`.",
    },
    EndpointRow {
        action: "stats_refresh_users",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=refresh_users_list`",
        notes: "Requires `confirm=true`.",
    },
    EndpointRow {
        action: "stats_delete_image_cache",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=delete_image_cache`",
        notes: "Requires `confirm=true`.",
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
        notes: "qBittorrent uses form `POST /api/v2/torrents/add` with `urls=<url>`. Requires `confirm=true`.",
    },
    EndpointRow {
        action: "download_pause",
        tools: "sabnzbd",
        endpoint: "one: `GET /api?mode=queue&name=pause&value=<id>&output=json`; all: `GET /api?mode=pause&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/stop` with `hashes=<hash-or-all>`. Requires `confirm=true`.",
    },
    EndpointRow {
        action: "download_resume",
        tools: "sabnzbd",
        endpoint: "one: `GET /api?mode=queue&name=resume&value=<id>&output=json`; all: `GET /api?mode=resume&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/start` with `hashes=<hash-or-all>`. Requires `confirm=true`.",
    },
    EndpointRow {
        action: "download_remove",
        tools: "sabnzbd",
        endpoint: "`GET /api?mode=queue&name=delete&value=<id>[&del_files=1]&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/delete` with `hashes=<hash>` and `deleteFiles={true|false}`. Requires `confirm=true`.",
    },
];

pub(super) const MEDIA_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "media_sessions",
        tools: "plex",
        endpoint: "`GET /status/sessions`",
        notes: "Jellyfin uses `GET /Sessions`.",
    },
    EndpointRow {
        action: "media_libraries",
        tools: "plex",
        endpoint: "`GET /library/sections`",
        notes: "Jellyfin uses `GET /Library/VirtualFolders`.",
    },
    EndpointRow {
        action: "media_search",
        tools: "plex",
        endpoint: "`GET /library/search?query=...`",
        notes: "Jellyfin uses `GET /Items?searchTerm=...&includeItemTypes=Movie,Series,Episode&recursive=true`.",
    },
    EndpointRow {
        action: "media_scan",
        tools: "plex",
        endpoint: "`GET /library/sections/{library}/refresh`",
        notes: "Jellyfin uses `POST /Library/Refresh` with `{}`. Requires `confirm=true`.",
    },
];
