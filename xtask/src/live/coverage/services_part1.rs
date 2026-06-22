//! Live-coverage data table — part 1 of 2 (Sonarr through Bazarr).
//!
//! Pure data, split out of `super` (coverage) so each file stays under the
//! module-size limit. Assembled back into the full matrix by
//! `coverage::services()`. The rendering/IO logic lives in `super`.

use super::{EndpointCoverage, ServiceCoverage};

pub(super) const SERVICES: &[ServiceCoverage] = &[
    ServiceCoverage {
        name: "Sonarr",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v3/system/status",
                implementation: "`service_status`; generic `api_get`; generic `api_post` confirm guard and confirmed upstream-error probe",
                checks: &[
                    "cli status sonarr",
                    "api_get sonarr /api/v3/system/status",
                    "api_post blocked sonarr",
                    "api_post confirmed upstream error sonarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v3/qualityprofile",
                implementation: "`quality_profiles`; also used by `set_quality` and `add`",
                checks: &[
                    "mcporter sonarr quality_profiles",
                    "mcporter confirmed arr item lifecycle sonarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v3/series",
                implementation: "`list`; also used by `set_quality`, `monitor`, `unmonitor`, and selection flows",
                checks: &[
                    "mcporter sonarr list",
                    "mcporter confirmed arr item lifecycle sonarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v3/wanted/missing?pageSize=50",
                implementation: "`wanted`",
                checks: &["mcporter sonarr wanted"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/queue?pageSize=50",
                implementation: "`queue`",
                checks: &["mcporter sonarr queue"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/history?pageSize=50",
                implementation: "`history`",
                checks: &["mcporter sonarr history"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/rootfolder",
                implementation: "`rootfolders`; also used by `add` setup/validation",
                checks: &[
                    "mcporter sonarr rootfolders",
                    "mcporter confirmed arr item lifecycle sonarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v3/health",
                implementation: "`health`",
                checks: &["mcporter sonarr health"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/series/editor",
                implementation: "`set_quality`, `monitor`, `unmonitor`",
                checks: &["mcporter confirmed arr item lifecycle sonarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/command",
                implementation: "`search`, `refresh`",
                checks: &["mcporter confirmed arr item lifecycle sonarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/series/lookup?term=...",
                implementation: "`add`",
                checks: &["mcporter confirmed arr item lifecycle sonarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/series POST",
                implementation: "`add`",
                checks: &["mcporter confirmed arr item lifecycle sonarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/series/{id}?deleteFiles={true|false} DELETE",
                implementation: "`delete`",
                checks: &["mcporter confirmed arr item lifecycle sonarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3 allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error sonarr api_post",
                    "mcporter confirmed generic error sonarr api_put",
                    "mcporter confirmed generic error sonarr api_delete",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Radarr",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v3/system/status",
                implementation: "`service_status`; generic `api_get`; generic `api_post` confirm guard and confirmed upstream-error probe",
                checks: &[
                    "cli status radarr",
                    "api_get radarr /api/v3/system/status",
                    "api_post blocked radarr",
                    "api_post confirmed upstream error radarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v3/qualityprofile",
                implementation: "`quality_profiles`; also used by `set_quality` and `add`",
                checks: &[
                    "mcporter radarr quality_profiles",
                    "mcporter confirmed arr item lifecycle radarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v3/movie",
                implementation: "`list`; also used by `set_quality`, `monitor`, `unmonitor`, and selection flows",
                checks: &[
                    "mcporter radarr list",
                    "mcporter confirmed arr item lifecycle radarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v3/wanted/missing?pageSize=50",
                implementation: "`wanted`",
                checks: &["mcporter radarr wanted"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/queue?pageSize=50",
                implementation: "`queue`",
                checks: &["mcporter radarr queue"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/history?pageSize=50",
                implementation: "`history`",
                checks: &["mcporter radarr history"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/rootfolder",
                implementation: "`rootfolders`; also used by `add` setup/validation",
                checks: &[
                    "mcporter radarr rootfolders",
                    "mcporter confirmed arr item lifecycle radarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v3/health",
                implementation: "`health`",
                checks: &["mcporter radarr health"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/movie/editor",
                implementation: "`set_quality`, `monitor`, `unmonitor`",
                checks: &["mcporter confirmed arr item lifecycle radarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/command",
                implementation: "`search`, `refresh`",
                checks: &["mcporter confirmed arr item lifecycle radarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/movie/lookup?term=...",
                implementation: "`add`",
                checks: &["mcporter confirmed arr item lifecycle radarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/movie POST",
                implementation: "`add`",
                checks: &["mcporter confirmed arr item lifecycle radarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3/movie/{id}?deleteFiles={true|false} DELETE",
                implementation: "`delete`",
                checks: &["mcporter confirmed arr item lifecycle radarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3 allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error radarr api_post",
                    "mcporter confirmed generic error radarr api_put",
                    "mcporter confirmed generic error radarr api_delete",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Prowlarr",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v1/system/status",
                implementation: "`service_status`; generic `api_get`; generic `api_post` confirm guard and confirmed upstream-error probe",
                checks: &[
                    "cli status prowlarr",
                    "api_get prowlarr /api/v1/system/status",
                    "api_post blocked prowlarr",
                    "api_post confirmed upstream error prowlarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v1/indexer",
                implementation: "`indexers`; one-indexer lookup for `indexer_test`; generic `api_get`",
                checks: &[
                    "mcporter prowlarr indexers",
                    "api_get prowlarr /api/v1/indexer",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v1/search?query=...&type=search&limit=100[&indexerIds=...]",
                implementation: "`indexer_search`",
                checks: &["mcporter prowlarr indexer_search"],
            },
            EndpointCoverage {
                endpoint: "/api/v1/indexerstats",
                implementation: "`indexer_stats`",
                checks: &["mcporter prowlarr indexer_stats"],
            },
            EndpointCoverage {
                endpoint: "/api/v1/indexer/testall",
                implementation: "`indexer_test` for all indexers",
                checks: &["mcporter confirmed write prowlarr indexer_test"],
            },
            EndpointCoverage {
                endpoint: "/api/v1/indexer/test",
                implementation: "`indexer_test` for one indexer",
                checks: &["mcporter prowlarr indexer_test"],
            },
            EndpointCoverage {
                endpoint: "/api/v1/tag",
                implementation: "confirmed generic tag lifecycle through `api_post`, `api_put`, `api_delete`",
                checks: &["mcporter confirmed write lifecycle prowlarr tag"],
            },
            EndpointCoverage {
                endpoint: "/api/v1 allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error prowlarr api_post",
                    "mcporter confirmed generic error prowlarr api_put",
                    "mcporter confirmed generic error prowlarr api_delete",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Tautulli",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v2?cmd=get_server_info",
                implementation: "`service_status`; generic `api_get`; generic `api_post` confirm guard and confirmed upstream-error probe",
                checks: &[
                    "cli status tautulli",
                    "api_get tautulli /api/v2?cmd=get_server_info",
                    "api_post blocked tautulli",
                    "api_post confirmed upstream error tautulli",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v2?cmd=get_activity",
                implementation: "`stats_activity`",
                checks: &["mcporter tautulli stats_activity"],
            },
            EndpointCoverage {
                endpoint: "/api/v2?cmd=get_history[&start=&length=&user=]",
                implementation: "`stats_history`",
                checks: &["mcporter tautulli stats_history"],
            },
            EndpointCoverage {
                endpoint: "/api/v2?cmd=get_users",
                implementation: "`stats_users`",
                checks: &["mcporter tautulli stats_users"],
            },
            EndpointCoverage {
                endpoint: "/api/v2?cmd=get_library_names",
                implementation: "`stats_libraries`",
                checks: &["mcporter tautulli stats_libraries"],
            },
            EndpointCoverage {
                endpoint: "/api/v2?cmd=refresh_libraries_list",
                implementation: "`stats_refresh_libraries`",
                checks: &["mcporter confirmed write tautulli maintenance lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api/v2?cmd=refresh_users_list",
                implementation: "`stats_refresh_users`",
                checks: &["mcporter confirmed write tautulli maintenance lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api/v2?cmd=delete_image_cache",
                implementation: "`stats_delete_image_cache`",
                checks: &["mcporter confirmed write tautulli maintenance lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api and /api/v2 allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error tautulli api_post",
                    "mcporter confirmed generic error tautulli api_put",
                    "mcporter confirmed generic error tautulli api_delete",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Overseerr",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v1/status",
                implementation: "`service_status`; generic `api_get`; generic `api_post` confirm guard and confirmed upstream-error probe",
                checks: &[
                    "cli status overseerr",
                    "api_get overseerr /api/v1/status",
                    "api_post blocked overseerr",
                    "api_post confirmed upstream error overseerr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v1/request[?filter=&take=&skip=]",
                implementation: "`requests`",
                checks: &["mcporter overseerr requests"],
            },
            EndpointCoverage {
                endpoint: "/api/v1/request POST",
                implementation: "`request_create`",
                checks: &["mcporter confirmed write overseerr request lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api/v1/request/{id}/approve",
                implementation: "`request_approve`",
                checks: &["mcporter confirmed write overseerr request lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api/v1/request/{id}/decline",
                implementation: "`request_decline`",
                checks: &["mcporter confirmed write overseerr request lifecycle"],
            },
            EndpointCoverage {
                endpoint: "/api/v1/search?query=...",
                implementation: "`request_search`",
                checks: &["mcporter overseerr request_search"],
            },
            EndpointCoverage {
                endpoint: "/api/v1 allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error overseerr api_post",
                    "mcporter confirmed generic error overseerr api_put",
                    "mcporter confirmed generic error overseerr api_delete",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Bazarr",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/system/status",
                implementation: "`service_status`; generic `api_get`; generic `api_post` confirm guard and confirmed upstream-error probe",
                checks: &[
                    "cli status bazarr",
                    "api_get bazarr /api/system/status",
                    "api_post blocked bazarr",
                    "api_post confirmed upstream error bazarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/system/health",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/system/jobs",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/system/tasks",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/movies",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/series",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/movies/subtitles",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/episodes/subtitles",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/subtitles",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/movies/wanted",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/episodes/wanted",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/movies/history",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/episodes/history",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/movies/blacklist",
                implementation: "seeded generic `api_delete` blacklist lifecycle",
                checks: &["mcporter confirmed write bazarr blacklist delete"],
            },
            EndpointCoverage {
                endpoint: "/api/episodes/blacklist",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/providers",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/plex/oauth/pin",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/plex/oauth/logout",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api/plex/webhook/list",
                implementation: "generic passthrough allowlist",
                checks: &[],
            },
            EndpointCoverage {
                endpoint: "/api and /api/v2 allowlist",
                implementation: "generic `api_get`, `api_post`, `api_put`, `api_delete`",
                checks: &[
                    "mcporter confirmed generic error bazarr api_post",
                    "mcporter confirmed generic error bazarr api_put",
                    "mcporter confirmed generic error bazarr api_delete",
                ],
            },
        ],
    },
];
