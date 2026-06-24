//! Live-coverage data table — part 2 of 2 (Tracearr through Jellyfin).
//!
//! See [`super::services_part1`] for the first half and `coverage::services()`
//! for the assembled matrix. See part 1 for the check-name conventions.

use super::{EndpointCoverage, ServiceCoverage};

pub(super) const SERVICES: &[ServiceCoverage] = &[
    ServiceCoverage {
        name: "Tracearr",
        rows: &[
            EndpointCoverage {
                endpoint: "/health",
                implementation: "`service_status` + generic `api_get` (GenericOnly kind)",
                checks: &[
                    "cli status tracearr",
                    "service_status tracearr",
                    "api_get tracearr /health",
                    "cli get tracearr /health",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v1 allowlist (generic passthrough) + seeded debug-session delete",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough (unconfirmed + confirmed upstream-error probes); seeded `api_delete` debug-session cleanup exercised by the lifecycles suite",
                checks: &[
                    "api_post unconfirmed upstream error tracearr",
                    "api_post confirmed upstream error tracearr",
                    "cli post unconfirmed upstream error tracearr",
                    "lifecycle tracearr debug delete",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "SABnzbd",
        rows: &[
            EndpointCoverage {
                endpoint: "/api?mode=version&output=json",
                implementation: "`service_status` + generic `api_get` (query-API kind)",
                checks: &[
                    "cli status sabnzbd",
                    "service_status sabnzbd",
                    "api_get sabnzbd /api?mode=version&output=json",
                    "cli get sabnzbd /api?mode=version&output=json",
                ],
            },
            EndpointCoverage {
                endpoint: "/api?mode=queue&output=json",
                implementation: "generic `api_get` probe (matrix-backed)",
                checks: &[
                    "api_get sabnzbd /api?mode=queue&output=json",
                    "cli get sabnzbd /api?mode=queue&output=json",
                ],
            },
            EndpointCoverage {
                endpoint: "/api allowlist (generic passthrough) + download_* lifecycle",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough (unconfirmed + confirmed upstream-error probes); curated `download_*` add/pause/resume/remove lifecycle exercised against a fixture NZB by the lifecycles suite",
                checks: &[
                    "api_post unconfirmed upstream error sabnzbd",
                    "api_post confirmed upstream error sabnzbd",
                    "cli post unconfirmed upstream error sabnzbd",
                    "lifecycle sabnzbd download",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "qBittorrent",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v2/app/version",
                implementation: "`service_status` + generic `api_get` (cookie-auth kind)",
                checks: &[
                    "cli status qbittorrent",
                    "service_status qbittorrent",
                    "api_get qbittorrent /api/v2/app/version",
                    "cli get qbittorrent /api/v2/app/version",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v2/torrents/info",
                implementation: "generic `api_get` probe (matrix-backed)",
                checks: &[
                    "api_get qbittorrent /api/v2/torrents/info",
                    "cli get qbittorrent /api/v2/torrents/info",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v2 allowlist (generic passthrough) + download_* lifecycle",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough (unconfirmed + confirmed upstream-error probes); curated `download_*` add/pause/resume/remove lifecycle exercised against a test magnet by the lifecycles suite",
                checks: &[
                    "api_post unconfirmed upstream error qbittorrent",
                    "api_post confirmed upstream error qbittorrent",
                    "cli post unconfirmed upstream error qbittorrent",
                    "lifecycle qbittorrent download",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Plex",
        rows: &[
            EndpointCoverage {
                endpoint: "/identity",
                implementation: "`service_status` + generic `api_get` (token-auth kind)",
                checks: &[
                    "cli status plex",
                    "service_status plex",
                    "api_get plex /identity",
                    "cli get plex /identity",
                ],
            },
            EndpointCoverage {
                endpoint: "generated OpenAPI operations (op action)",
                implementation: "every spec operation dispatched and schema-validated by the `contract` suite",
                checks: &["contract plex"],
            },
            EndpointCoverage {
                endpoint: "/identity, /library, /status, /servers allowlist (generic passthrough)",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough; unconfirmed + confirmed upstream-error probes",
                checks: &[
                    "api_post unconfirmed upstream error plex",
                    "api_post confirmed upstream error plex",
                    "cli post unconfirmed upstream error plex",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Jellyfin",
        rows: &[
            EndpointCoverage {
                endpoint: "/System/Info/Public",
                implementation: "`service_status` + generic `api_get` (token-auth kind)",
                checks: &[
                    "cli status jellyfin",
                    "service_status jellyfin",
                    "api_get jellyfin /System/Info/Public",
                    "cli get jellyfin /System/Info/Public",
                ],
            },
            EndpointCoverage {
                endpoint: "generated OpenAPI operations (op action)",
                implementation: "every spec operation dispatched and schema-validated by the `contract` suite",
                checks: &["contract jellyfin"],
            },
            EndpointCoverage {
                endpoint: "/System, /Items, /Users, /Library, /Sessions allowlist (generic passthrough)",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough; unconfirmed + confirmed upstream-error probes",
                checks: &[
                    "api_post unconfirmed upstream error jellyfin",
                    "api_post confirmed upstream error jellyfin",
                    "cli post unconfirmed upstream error jellyfin",
                ],
            },
        ],
    },
];
