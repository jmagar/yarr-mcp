# Vendored OpenAPI specs

Authoritative upstream API specs, vendored as the **source of truth** for the
Code Mode typed operation client codegen (see `xtask` generator + `src/codemode`).
Re-fetch with the URLs below when bumping.

| File | Service | Source |
|------|---------|--------|
| `sonarr.openapi.json`   | Sonarr (v3)   | github.com/Sonarr/Sonarr `src/Sonarr.Api.V3/openapi.json` (develop) |
| `radarr.openapi.json`   | Radarr (v3)   | github.com/Radarr/Radarr `src/Radarr.Api.V3/openapi.json` (develop) |
| `prowlarr.openapi.json` | Prowlarr (v1) | github.com/Prowlarr/Prowlarr `src/Prowlarr.Api.V1/openapi.json` (develop) |
| `overseerr.openapi.yml` | Overseerr     | github.com/sct/overseerr `overseerr-api.yml` (develop) |
| `jellyfin.openapi.json` | Jellyfin      | api.jellyfin.org `jellyfin-openapi-stable.json` |
| `plex.openapi.yml`      | Plex          | github.com/LukeHagar/plex-api-spec `plex-api-spec.yaml` (main) |

The remaining 5 services (Tautulli, SABnzbd, qBittorrent, Bazarr, Tracearr) have no
machine-readable spec; their operations are derived from the endpoint annotations in
`src/models/<svc>.rs` doc comments / published docs.
