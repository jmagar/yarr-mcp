//! Typed upstream response contracts — one module per supported `ServiceKind`.
//!
//! yarr forwards upstream payloads as [`serde_json::Value`]; this module makes
//! the *shape* of each service's API explicit as typed, `Deserialize`-able Rust.
//! Unlike the old slim subsets, these are **complete contracts** sourced from each
//! service's authoritative definition:
//!
//!   * **OpenAPI** — Sonarr/Radarr/Prowlarr (`openapi.json`), Overseerr
//!     (`overseerr-api.yml`), Jellyfin (`api.jellyfin.org`), Plex
//!     (`LukeHagar/plex-api-spec`).
//!   * **Published docs** — Tautulli, SABnzbd, qBittorrent, Bazarr, Tracearr.
//!
//! Each module models the service's media-automation surface (resources,
//! profiles, queue/history, commands, health, system status, search/lookup,
//! sessions, libraries, requests, …) with full field coverage.
//!
//! Design rules (faithful + drift-tolerant):
//!   * **Every field is optional or defaulted.** These APIs are loose and
//!     version-dependent; a missing field must never fail deserialization.
//!     `Option<T>` fields default to `None`, `Vec<T>` fields carry
//!     `#[serde(default)]`. (OpenAPI `required` is a server promise, not a
//!     deserializer demand.)
//!   * **Unknown fields are ignored** (serde default), so an upstream that adds
//!     keys still decodes cleanly.
//!   * **Casing mirrors the wire** via `#[serde(rename_all)]` + per-field renames
//!     (e.g. `type` → `kind`, SABnzbd's string-encoded numerics, Plex's mixed
//!     PascalCase containers / camelCase scalars).
//!   * Models derive [`schemars::JsonSchema`] so a machine-readable schema can be
//!     emitted per type — this is what feeds the Code Mode `api.<service>` client's
//!     type hints.
//!
//! These are an available, tested typing layer alongside the `Value`+`slim()`
//! forwarding path. Colocated `*_tests.rs` deserialize representative fixtures to
//! prove each model matches the real wire shape.

// The 6 spec-backed services (sonarr, radarr, prowlarr, overseerr, jellyfin, plex)
// no longer have hand-written models: their types are generated from the vendored
// OpenAPI specs into `src/openapi/generated/` and surfaced via Code Mode
// `describe`. Only the 5 doc-based services keep hand-modeled contracts here.
pub mod bazarr;
pub mod qbittorrent;
pub mod sabnzbd;
pub mod tautulli;
pub mod tracearr;

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
