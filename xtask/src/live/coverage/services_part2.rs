//! Live-coverage data table — part 2 of 2 (Tracearr through Jellyfin).
//!
//! See [`super::services_part1`] for the first half and `coverage::services()`
//! for the assembled matrix.

use super::{EndpointCoverage, ServiceCoverage};

pub(super) const SERVICES: &[ServiceCoverage] = &[
    ServiceCoverage {
        name: "Tracearr",
        rows: &[
            EndpointCoverage {
                endpoint: "/health",
                implementation: "`service_status`; generic `api_get`; generic `api_post` unconfirmed upstream-error probe",
                checks: &[
                    "cli status tracearr",
                    "api_get tracearr /health",
                    "api_post unconfirmed upstream error tracearr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v1/public/health",
                implementation: "generic confirmed upstream-error probe through `api_post`",
                checks: &["api_post confirmed upstream error tracearr"],
            },
            EndpointCoverage {
                endpoint: "/api/v1/public/stats",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/public/stats/today",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/public/activity",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/public/streams",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/public/streams/{id}/terminate",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/public/users",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/public/violations",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/public/history",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/debug/sessions",
                implementation: "seeded generic `api_delete` debug-session lifecycle",
                checks: &["mcporter confirmed write tracearr debug sessions delete"],
            },
            EndpointCoverage {
                endpoint: "/api/v1/debug/violations",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/debug/rules",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/debug/library",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/debug/users",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/debug/servers",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/v1/debug/reset",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/health and /api/v1 allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error tracearr api_post",
                    "mcporter confirmed generic error tracearr api_put",
                    "mcporter confirmed generic error tracearr api_delete",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "SABnzbd",
        rows: &[
            EndpointCoverage {
                endpoint: "/api?mode=version&output=json",
                implementation: "`service_status`; generic `api_get`; generic `api_post` unconfirmed and confirmed upstream-error probes",
                checks: &[
                    "cli status sabnzbd",
                    "api_get sabnzbd /api?mode=version&output=json",
                    "api_post unconfirmed upstream error sabnzbd",
                    "api_post confirmed upstream error sabnzbd",
                ],
            },
            EndpointCoverage {
                endpoint: "/api?mode=queue&output=json",
                implementation: "`download_queue`; generic `api_get`",
                checks: &[
                    "mcporter sabnzbd download_queue",
                    "api_get sabnzbd /api?mode=queue&output=json",
                ],
            },
            EndpointCoverage {
                endpoint: "/api?mode=addurl&name=<url>&output=json",
                implementation: "`download_add`",
                checks: &["mcporter confirmed write sabnzbd download lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api?mode=queue&name=pause&value=<id>&output=json",
                implementation: "`download_pause` for one job",
                checks: &["mcporter confirmed write sabnzbd download lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api?mode=pause&output=json",
                implementation: "`download_pause` for all jobs",
                checks: &["mcporter confirmed write sabnzbd download lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api?mode=queue&name=resume&value=<id>&output=json",
                implementation: "`download_resume` for one job",
                checks: &["mcporter confirmed write sabnzbd download lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api?mode=resume&output=json",
                implementation: "`download_resume` for all jobs",
                checks: &["mcporter confirmed write sabnzbd download lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api?mode=queue&name=delete&value=<id>[&del_files=1]&output=json",
                implementation: "`download_remove`",
                checks: &["mcporter confirmed write sabnzbd download lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api and /api/v2 allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error sabnzbd api_post",
                    "mcporter confirmed generic error sabnzbd api_put",
                    "mcporter confirmed generic error sabnzbd api_delete",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "qBittorrent",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v2/app/version",
                implementation: "`service_status`; generic `api_get`; generic `api_post` unconfirmed and confirmed upstream-error probes",
                checks: &[
                    "cli status qbittorrent",
                    "api_get qbittorrent /api/v2/app/version",
                    "api_post unconfirmed upstream error qbittorrent",
                    "api_post confirmed upstream error qbittorrent",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v2/torrents/info",
                implementation: "`download_queue`; generic `api_get`",
                checks: &[
                    "mcporter qbittorrent download_queue",
                    "api_get qbittorrent /api/v2/torrents/info",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v2/torrents/add",
                implementation: "`download_add`",
                checks: &["mcporter confirmed write lifecycle qbittorrent torrent"],
            },
            EndpointCoverage {
                endpoint: "/api/v2/torrents/stop",
                implementation: "`download_pause`",
                checks: &["mcporter confirmed write lifecycle qbittorrent torrent"],
            },
            EndpointCoverage {
                endpoint: "/api/v2/torrents/start",
                implementation: "`download_resume`",
                checks: &["mcporter confirmed write lifecycle qbittorrent torrent"],
            },
            EndpointCoverage {
                endpoint: "/api/v2/torrents/delete",
                implementation: "`download_remove`",
                checks: &["mcporter confirmed write lifecycle qbittorrent torrent"],
            },
            EndpointCoverage {
                endpoint: "/api/v2 allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error qbittorrent api_post",
                    "mcporter confirmed generic error qbittorrent api_put",
                    "mcporter confirmed generic error qbittorrent api_delete",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Plex",
        rows: &[
            EndpointCoverage {
                endpoint: "/identity",
                implementation: "`service_status`; generic `api_get`; generic `api_post` unconfirmed and confirmed upstream-error probes",
                checks: &[
                    "cli status plex",
                    "api_get plex /identity",
                    "api_post unconfirmed upstream error plex",
                    "api_post confirmed upstream error plex",
                ],
            },
            EndpointCoverage {
                endpoint: "/status/sessions",
                implementation: "`media_sessions`",
                checks: &["mcporter plex media_sessions"],
            },
            EndpointCoverage {
                endpoint: "/library/sections",
                implementation: "`media_libraries`",
                checks: &["mcporter plex media_libraries"],
            },
            EndpointCoverage {
                endpoint: "/library/search?query=...",
                implementation: "`media_search`",
                checks: &["mcporter plex media_search"],
            },
            EndpointCoverage {
                endpoint: "/library/sections/{library}/refresh",
                implementation: "`media_scan`",
                checks: &["mcporter confirmed write plex media_scan"],
            },
            EndpointCoverage {
                endpoint: "/identity, /library, /status, /servers allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error plex api_post",
                    "mcporter confirmed generic error plex api_put",
                    "mcporter confirmed generic error plex api_delete",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Jellyfin",
        rows: &[
            EndpointCoverage {
                endpoint: "/System/Info/Public",
                implementation: "`service_status`; generic `api_get`; generic `api_post` unconfirmed and confirmed upstream-error probes",
                checks: &[
                    "cli status jellyfin",
                    "api_get jellyfin /System/Info/Public",
                    "api_post unconfirmed upstream error jellyfin",
                    "api_post confirmed upstream error jellyfin",
                ],
            },
            EndpointCoverage {
                endpoint: "/Sessions",
                implementation: "`media_sessions`",
                checks: &["mcporter jellyfin media_sessions"],
            },
            EndpointCoverage {
                endpoint: "/Library/VirtualFolders",
                implementation: "`media_libraries`",
                checks: &["mcporter jellyfin media_libraries"],
            },
            EndpointCoverage {
                endpoint: "/Items?searchTerm=...&includeItemTypes=Movie,Series,Episode&recursive=true",
                implementation: "`media_search`",
                checks: &["mcporter jellyfin media_search"],
            },
            EndpointCoverage {
                endpoint: "/Library/Refresh",
                implementation: "`media_scan`",
                checks: &["mcporter confirmed write jellyfin media_scan"],
            },
            EndpointCoverage {
                endpoint: "/System, /Items, /Users, /Library, /Sessions allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error jellyfin api_post",
                    "mcporter confirmed generic error jellyfin api_put",
                    "mcporter confirmed generic error jellyfin api_delete",
                ],
            },
        ],
    },
];
