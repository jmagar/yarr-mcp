//! DownloadClient capability: business-layer methods for SABnzbd (C5) and
//! qBittorrent (C5).
//!
//! The two clients share a verb set (`queue`, `add`, `pause`, `resume`,
//! `remove`) but their APIs diverge *completely*: SABnzbd is a `?mode=` query API
//! authed by an `apikey` query param, while qBittorrent is a `/api/v2` REST API
//! authed by a username/password cookie session. Per the locked bead decision the
//! per-client split is **UNCONDITIONAL** — each public method on
//! [`RustarrService`] resolves the service's
//! [`KindDescriptor`](crate::capability::KindDescriptor) and dispatches to the
//! [`sab`] or [`qbit`] impl by `query_api` flag rather than matching the kind
//! ad-hoc inside the method body.
//!
//! Scope split (locked in the bead): `queue` is READ; `add`, `pause`, `resume`,
//! and `remove` mutate, so they are WRITE + confirm-gated. `remove` defaults
//! `delete_files` to `false` (opt-in via `--delete-files` / `delete_files=true`).
//!
//! The curated-command *descriptors* (registry table) live in
//! `src/actions/commands/download.rs`, not here — this module only holds logic.

pub mod qbit;
pub mod sab;

use anyhow::Result;
use serde_json::Value;

use crate::app::RustarrService;
use crate::capability::Capability;
use crate::config::ServiceConfig;

impl RustarrService {
    /// Resolve a DownloadClient service and verify its capability. Central helper
    /// so every download method shares one capability-checked resolution path; a
    /// non-download kind (e.g. plex) is rejected here before any request is built.
    fn download_context<'a>(&'a self, service: &str) -> Result<&'a ServiceConfig> {
        self.service_of_capability(service, Capability::DownloadClient)
    }

    /// List the active downloads, slimmed. READ.
    ///
    /// Dispatches by `query_api`: SABnzbd (`?mode=queue`) vs qBittorrent
    /// (`/api/v2/torrents/info`).
    pub async fn download_queue(&self, service: &str) -> Result<Value> {
        let config = self.download_context(service)?;
        if config.kind.descriptor().query_api {
            sab::queue(self, config).await
        } else {
            qbit::queue(self, config).await
        }
    }

    /// Add a download by URL/magnet. WRITE + confirm-gated.
    pub async fn download_add(&self, service: &str, url: &str, confirm: bool) -> Result<Value> {
        if !confirm {
            anyhow::bail!(
                "download add queues a new download (write); pass confirm=true (CLI --confirm) to run it"
            );
        }
        let config = self.download_context(service)?;
        if config.kind.descriptor().query_api {
            sab::add(self, config, url).await
        } else {
            qbit::add(self, config, url).await
        }
    }

    /// Pause downloads (all, or a specific id/hash). WRITE + confirm-gated.
    pub async fn download_pause(
        &self,
        service: &str,
        id: Option<&str>,
        confirm: bool,
    ) -> Result<Value> {
        if !confirm {
            anyhow::bail!(
                "download pause changes download state (write); pass confirm=true (CLI --confirm) to run it"
            );
        }
        let config = self.download_context(service)?;
        if config.kind.descriptor().query_api {
            sab::pause(self, config, id).await
        } else {
            qbit::pause(self, config, id).await
        }
    }

    /// Resume downloads (all, or a specific id/hash). WRITE + confirm-gated.
    pub async fn download_resume(
        &self,
        service: &str,
        id: Option<&str>,
        confirm: bool,
    ) -> Result<Value> {
        if !confirm {
            anyhow::bail!(
                "download resume changes download state (write); pass confirm=true (CLI --confirm) to run it"
            );
        }
        let config = self.download_context(service)?;
        if config.kind.descriptor().query_api {
            sab::resume(self, config, id).await
        } else {
            qbit::resume(self, config, id).await
        }
    }

    /// Remove a download. `delete_files` (default false) also deletes the
    /// downloaded data. WRITE + confirm-gated.
    pub async fn download_remove(
        &self,
        service: &str,
        id: &str,
        delete_files: bool,
        confirm: bool,
    ) -> Result<Value> {
        if !confirm {
            anyhow::bail!(
                "download remove deletes a download (write); pass confirm=true (CLI --confirm) to run it"
            );
        }
        let config = self.download_context(service)?;
        if config.kind.descriptor().query_api {
            sab::remove(self, config, id, delete_files).await
        } else {
            qbit::remove(self, config, id, delete_files).await
        }
    }
}

#[cfg(test)]
#[path = "download_tests.rs"]
mod tests;
