//! Live-coverage data table — part 1 of 2 (Sonarr through Bazarr).
//!
//! Pure data, split out of `super` (coverage) so each file stays under the
//! module-size limit. Assembled back into the full matrix by
//! `coverage::services()`. The rendering/IO logic lives in `super`.
//!
//! Check names must match `report.pass(...)` names produced by the live suites:
//!   - `cli`      → `cli status <svc>`, `cli get <svc> <path>`, `cli post unconfirmed upstream error <svc>`
//!   - `services` → `service_status <svc>`, `api_get <svc> <path>`, `api_post {unconfirmed,confirmed} upstream error <svc>`
//!   - `contract` → `contract <svc>` (every generated OpenAPI op, schema-validated; spec-backed kinds only)
//!   - `mcp`      → `mcp yarr service_status sonarr`, `mcp api_post confirmed upstream error`

use super::{EndpointCoverage, ServiceCoverage};

pub(super) const SERVICES: &[ServiceCoverage] = &[
    ServiceCoverage {
        name: "Sonarr",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v3/system/status",
                implementation: "`service_status` + generic `api_get`; also the representative MCP transport round-trip through the `yarr` tool",
                checks: &[
                    "cli status sonarr",
                    "service_status sonarr",
                    "api_get sonarr /api/v3/system/status",
                    "cli get sonarr /api/v3/system/status",
                    "mcp yarr service_status sonarr",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v3/series",
                implementation: "generic `api_get` probe (matrix-backed)",
                checks: &[
                    "api_get sonarr /api/v3/series",
                    "cli get sonarr /api/v3/series",
                ],
            },
            EndpointCoverage {
                endpoint: "generated OpenAPI operations (op action)",
                implementation: "every spec operation dispatched and schema-validated by the `contract` suite",
                checks: &["contract sonarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3 allowlist (generic passthrough)",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough; unconfirmed + confirmed upstream-error probes (CLI and MCP)",
                checks: &[
                    "api_post unconfirmed upstream error sonarr",
                    "api_post confirmed upstream error sonarr",
                    "cli post unconfirmed upstream error sonarr",
                    "mcp api_post confirmed upstream error",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Radarr",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v3/system/status",
                implementation: "`service_status` + generic `api_get`",
                checks: &[
                    "cli status radarr",
                    "service_status radarr",
                    "api_get radarr /api/v3/system/status",
                    "cli get radarr /api/v3/system/status",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v3/movie",
                implementation: "generic `api_get` probe (matrix-backed)",
                checks: &[
                    "api_get radarr /api/v3/movie",
                    "cli get radarr /api/v3/movie",
                ],
            },
            EndpointCoverage {
                endpoint: "generated OpenAPI operations (op action)",
                implementation: "every spec operation dispatched and schema-validated by the `contract` suite",
                checks: &["contract radarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v3 allowlist (generic passthrough)",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough; unconfirmed + confirmed upstream-error probes",
                checks: &[
                    "api_post unconfirmed upstream error radarr",
                    "api_post confirmed upstream error radarr",
                    "cli post unconfirmed upstream error radarr",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Prowlarr",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v1/system/status",
                implementation: "`service_status` + generic `api_get`",
                checks: &[
                    "cli status prowlarr",
                    "service_status prowlarr",
                    "api_get prowlarr /api/v1/system/status",
                    "cli get prowlarr /api/v1/system/status",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v1/indexer",
                implementation: "generic `api_get` probe (matrix-backed)",
                checks: &[
                    "api_get prowlarr /api/v1/indexer",
                    "cli get prowlarr /api/v1/indexer",
                ],
            },
            EndpointCoverage {
                endpoint: "generated OpenAPI operations (op action)",
                implementation: "every spec operation dispatched and schema-validated by the `contract` suite",
                checks: &["contract prowlarr"],
            },
            EndpointCoverage {
                endpoint: "/api/v1 allowlist (generic passthrough)",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough; unconfirmed + confirmed upstream-error probes",
                checks: &[
                    "api_post unconfirmed upstream error prowlarr",
                    "api_post confirmed upstream error prowlarr",
                    "cli post unconfirmed upstream error prowlarr",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Tautulli",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v2?cmd=get_server_info",
                implementation: "`service_status` + generic `api_get` (query-API kind)",
                checks: &[
                    "cli status tautulli",
                    "service_status tautulli",
                    "api_get tautulli /api/v2?cmd=get_server_info",
                    "cli get tautulli /api/v2?cmd=get_server_info",
                ],
            },
            EndpointCoverage {
                endpoint: "/api/v2 allowlist (generic passthrough)",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough; unconfirmed + confirmed upstream-error probes. Curated `stats_*` reads/maintenance are not yet re-homed on the live stack.",
                checks: &[
                    "api_post unconfirmed upstream error tautulli",
                    "api_post confirmed upstream error tautulli",
                    "cli post unconfirmed upstream error tautulli",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Overseerr",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/v1/status",
                implementation: "`service_status` + generic `api_get`",
                checks: &[
                    "cli status overseerr",
                    "service_status overseerr",
                    "api_get overseerr /api/v1/status",
                    "cli get overseerr /api/v1/status",
                ],
            },
            EndpointCoverage {
                endpoint: "generated OpenAPI operations (op action)",
                implementation: "every spec operation dispatched and schema-validated by the `contract` suite",
                checks: &["contract overseerr"],
            },
            EndpointCoverage {
                endpoint: "/api/v1 allowlist (generic passthrough)",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough; unconfirmed + confirmed upstream-error probes",
                checks: &[
                    "api_post unconfirmed upstream error overseerr",
                    "api_post confirmed upstream error overseerr",
                    "cli post unconfirmed upstream error overseerr",
                ],
            },
        ],
    },
    ServiceCoverage {
        name: "Bazarr",
        rows: &[
            EndpointCoverage {
                endpoint: "/api/system/status",
                implementation: "`service_status` + generic `api_get` (GenericOnly kind)",
                checks: &[
                    "cli status bazarr",
                    "service_status bazarr",
                    "api_get bazarr /api/system/status",
                    "cli get bazarr /api/system/status",
                ],
            },
            EndpointCoverage {
                endpoint: "/api allowlist (generic passthrough)",
                implementation: "`api_post`/`api_put`/`api_delete` generic passthrough; unconfirmed + confirmed upstream-error probes. Seeded `api_delete` blacklist cleanup is not yet re-homed on the live stack.",
                checks: &[
                    "api_post unconfirmed upstream error bazarr",
                    "api_post confirmed upstream error bazarr",
                    "cli post unconfirmed upstream error bazarr",
                ],
            },
        ],
    },
];
