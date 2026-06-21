//! MediaServer capability: business-layer methods for Plex (C6) and Jellyfin
//! (C6).
//!
//! The two servers share a verb set (`sessions`, `libraries`, `search`, `scan`)
//! but their APIs diverge *completely*: Plex is an XML-by-default HTTP API authed
//! by an `X-Plex-Token` query param that REQUIRES `Accept: application/json` on
//! every call to negotiate a JSON body, while Jellyfin is a JSON REST API authed
//! by a `MediaBrowser Token="…"` header whose item queries MUST always pass
//! `includeItemTypes` (everything is a `BaseItemDto`). Per the C5 pattern the
//! per-server split is **UNCONDITIONAL** — each public method on
//! [`RustarrService`] resolves the service's
//! [`KindDescriptor`](crate::capability::KindDescriptor) and dispatches to the
//! [`plex`] or [`jellyfin`] impl by [`AuthStyle`]
//! rather than matching the kind ad-hoc inside the method body.
//!
//! Scope split (locked in the bead): `sessions`, `libraries`, and `search` are
//! READ; `scan` triggers a library refresh, so it is WRITE — but it is not
//! destructive, so it runs immediately (no confirm gate).
//!
//! The Plex JSON/XML negotiation belongs in TRANSPORT, not here — the impls only
//! *choose* `accept_mime = "application/json"` and pass it to
//! [`send_get`](crate::rustarr::RustarrClient::send_get); no XML parsing lives in
//! the app layer (architecture decision C6-a).
//!
//! The curated-command *descriptors* (registry table) live in
//! `src/actions/commands/media_server.rs`, not here — this module only holds
//! logic.

pub mod jellyfin;
pub mod plex;

use anyhow::Result;
use serde_json::Value;

use crate::app::RustarrService;
use crate::capability::{AuthStyle, Capability};
use crate::config::ServiceConfig;

impl RustarrService {
    /// Resolve a MediaServer service and verify its capability. Central helper so
    /// every media method shares one capability-checked resolution path; a
    /// non-media kind (e.g. sonarr) is rejected here before any request is built.
    fn media_context<'a>(&'a self, service: &str) -> Result<&'a ServiceConfig> {
        self.service_of_capability(service, Capability::MediaServer)
    }

    /// True when the resolved config is a Plex service (vs Jellyfin). Dispatch
    /// keys on auth style — the SSOT topology field that already distinguishes the
    /// two media kinds — rather than re-matching `ServiceKind`.
    fn is_plex(config: &ServiceConfig) -> bool {
        config.kind.descriptor().auth_style == AuthStyle::PlexToken
    }

    /// List the active streaming sessions, slimmed. READ.
    ///
    /// Dispatches by auth style: Plex (`GET /status/sessions`, JSON-negotiated) vs
    /// Jellyfin (`GET /Sessions`).
    pub async fn media_sessions(&self, service: &str) -> Result<Value> {
        let config = self.media_context(service)?;
        if Self::is_plex(config) {
            plex::sessions(self, config).await
        } else {
            jellyfin::sessions(self, config).await
        }
    }

    /// List the libraries, slimmed. READ.
    ///
    /// Plex (`GET /library/sections`) vs Jellyfin (`GET /Library/VirtualFolders`).
    pub async fn media_libraries(&self, service: &str) -> Result<Value> {
        let config = self.media_context(service)?;
        if Self::is_plex(config) {
            plex::libraries(self, config).await
        } else {
            jellyfin::libraries(self, config).await
        }
    }

    /// Search the library for `query`, slimmed. READ.
    ///
    /// Plex (`GET /library/search?query=`) vs Jellyfin
    /// (`GET /Items?searchTerm=&includeItemTypes=…&recursive=true`).
    pub async fn media_search(&self, service: &str, query: &str) -> Result<Value> {
        let config = self.media_context(service)?;
        if Self::is_plex(config) {
            plex::search(self, config, query).await
        } else {
            jellyfin::search(self, config, query).await
        }
    }

    /// Trigger a library scan/refresh. Mutating but not destructive — runs
    /// immediately, no confirm gate.
    ///
    /// Plex (`GET /library/sections/{library}/refresh`, `library` required) vs
    /// Jellyfin (`POST /Library/Refresh`, server-wide; `library` is ignored).
    pub async fn media_scan(&self, service: &str, library: Option<&str>) -> Result<Value> {
        let config = self.media_context(service)?;
        if Self::is_plex(config) {
            let library = library.ok_or_else(|| {
                anyhow::anyhow!("plex scan requires --library (a library section id)")
            })?;
            // S6: a Plex section id is numeric. Parse it instead of interpolating
            // raw text into the request path, so no injection / traversal value
            // can reach the upstream URL.
            let section_id = library
                .trim()
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("plex --library must be a numeric section id"))?;
            plex::scan(self, config, section_id).await
        } else {
            jellyfin::scan(self, config).await
        }
    }
}

#[cfg(test)]
#[path = "media_server_tests.rs"]
mod tests;
