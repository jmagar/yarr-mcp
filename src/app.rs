//! Business service layer.

use anyhow::{Result, anyhow};
use serde_json::Value;

use crate::{
    config::{ServiceConfig, ServiceKind, YarrConfig},
    yarr::{YarrClient, validate_safe_path},
};

pub mod codemode;
pub mod download;
pub mod openapi_ops;
pub mod stats;
pub mod subtitles;
pub mod trace;

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct YarrService {
    client: YarrClient,
    services: Vec<ServiceConfig>,
    /// Root dir for Code Mode `writeArtifact` output. `None` disables artifacts
    /// (the default; the binary sets it from the data dir). Per-run subdirs are
    /// created under this root.
    data_dir: Option<std::path::PathBuf>,
    /// Code Mode `codemode.search()` semantic-scoring cache — catalog embeddings
    /// and the failure cooldown, shared (via `Arc`) across every clone of this
    /// service for the process's lifetime, so it's computed at most once, not
    /// per script run and not per clone. See [`crate::codemode::semantic`].
    ///
    /// Fully qualified as `crate::codemode` (not the bare `codemode` module
    /// path) because this struct's own `pub mod codemode;` (this file, above)
    /// would otherwise shadow the top-level engine module of the same name.
    semantic_cache: std::sync::Arc<crate::codemode::SemanticCache>,
}

impl YarrService {
    pub fn new(client: YarrClient, config: YarrConfig) -> Self {
        Self {
            client,
            services: config.services,
            data_dir: None,
            semantic_cache: std::sync::Arc::new(crate::codemode::SemanticCache::new()),
        }
    }

    /// Enable Code Mode `writeArtifact` by setting the artifacts root (typically
    /// the resolved data dir). Builder so the `new(client, config)` signature and
    /// its call sites stay unchanged.
    pub fn with_data_dir(mut self, root: std::path::PathBuf) -> Self {
        self.data_dir = Some(root);
        self
    }

    /// The configured artifacts root, if Code Mode `writeArtifact` is enabled.
    pub(crate) fn data_dir(&self) -> Option<&std::path::Path> {
        self.data_dir.as_deref()
    }

    /// The shared semantic-search cache for `codemode.search()` — see
    /// [`crate::codemode::semantic`].
    pub(crate) fn semantic_cache(&self) -> &std::sync::Arc<crate::codemode::SemanticCache> {
        &self.semantic_cache
    }

    /// Configured `(name, kind)` pairs, in declaration order. Drives the Code Mode
    /// per-service callable namespaces (`<service>.<verb>()`) and the discovery
    /// catalog — the service is baked into each callable, so a script never passes
    /// a `service` param and never needs to enumerate services.
    pub(crate) fn configured_service_kinds(&self) -> Vec<(String, ServiceKind)> {
        self.services
            .iter()
            .map(|service| (service.name.clone(), service.kind))
            .collect()
    }

    pub fn configured_service_count(&self) -> usize {
        self.services
            .iter()
            .filter(|service| !service.base_url.trim().is_empty())
            .count()
    }

    pub async fn service_status(&self, service: &str) -> Result<Value> {
        let service = self.service(service)?;
        self.client
            .get_json(service, service.kind.default_status_path())
            .await
    }

    pub async fn api_get(&self, service: &str, path: &str) -> Result<Value> {
        validate_safe_path(path)?;
        self.client.get_json(self.service(service)?, path).await
    }

    /// POST passthrough. Mutating but NOT destructive, so it runs immediately —
    /// no confirm gate (the write-confirm gate is reserved for destructive
    /// deletes; see [`api_delete`](Self::api_delete)).
    pub async fn api_post(&self, service: &str, path: &str, body: Value) -> Result<Value> {
        validate_safe_path(path)?;
        self.client
            .post_json(self.service(service)?, path, body)
            .await
    }

    /// PUT passthrough. Mutating but not destructive — runs immediately (see
    /// [`api_post`](Self::api_post)).
    pub async fn api_put(&self, service: &str, path: &str, body: Value) -> Result<Value> {
        validate_safe_path(path)?;
        self.client
            .put_json(self.service(service)?, path, body)
            .await
    }

    /// DELETE passthrough — the one destructive generic verb, so it stays gated:
    /// `confirm=true` is required to actually issue the request. On the MCP
    /// surface that confirm is obtained via elicitation; on the CLI via
    /// `--confirm`. Either way the gate is enforced here so neither shim can
    /// bypass it.
    pub async fn api_delete(
        &self,
        service: &str,
        path: &str,
        body: Option<Value>,
        confirm: bool,
    ) -> Result<Value> {
        if !confirm && !crate::config::destructive_allowed() {
            anyhow::bail!(
                "api_delete is destructive and requires confirm=true (MCP: approve the \
                 elicitation prompt; CLI: pass --confirm; or set YARR_ALLOW_DESTRUCTIVE \
                 on a disposable test stack)"
            );
        }
        validate_safe_path(path)?;
        self.client
            .delete_json(self.service(service)?, path, body)
            .await
    }

    /// Resolve a configured service by name/kind and verify its capability
    /// matches `cap`. Curated command handlers use this so a command targeting one
    /// capability cannot run against an unrelated kind.
    pub(crate) fn service_of_capability(
        &self,
        name: &str,
        cap: crate::capability::Capability,
    ) -> Result<&ServiceConfig> {
        let service = self.service(name)?;
        if service.kind.capability() != cap {
            anyhow::bail!(
                "service {} (kind={}) does not provide the {:?} capability",
                service.name,
                service.kind.as_str(),
                cap
            );
        }
        Ok(service)
    }

    /// Resolve a configured service name/kind to its [`ServiceKind`], if known.
    ///
    /// Used by the shared action×kind validation guard so both the CLI and MCP
    /// dispatch paths reject curated commands run against an incompatible kind.
    /// Returns `None` when the name does not match any configured service (the
    /// downstream service lookup then produces the canonical "unknown service"
    /// error).
    pub(crate) fn kind_of(&self, name: &str) -> Option<ServiceKind> {
        self.find_service(name).map(|service| service.kind)
    }

    /// Single source of truth for name/kind → service resolution. Trims and
    /// lowercases `name`, then matches a configured service by its name or kind.
    /// Returns `None` for an empty name or no match. Callers that additionally
    /// require a configured `base_url` layer that check on top (see `service`).
    fn find_service(&self, name: &str) -> Option<&ServiceConfig> {
        let needle = name.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return None;
        }
        self.services
            .iter()
            .find(|service| service.name == needle || service.kind.as_str() == needle)
    }

    /// Transport-client accessor for capability submodules (e.g. `app::arr`).
    /// Keeps `client` private to `YarrService` while letting curated command
    /// logic in sibling modules issue requests through the shared transport.
    pub(crate) fn client_ref(&self) -> &YarrClient {
        &self.client
    }

    fn service(&self, name: &str) -> Result<&ServiceConfig> {
        if name.trim().is_empty() {
            anyhow::bail!("service is required");
        }
        // Shared resolution (`find_service`), then layer the empty-`base_url`
        // rejection that callers of `service()` require but `kind_of()` does not.
        let service = self
            .find_service(name)
            .ok_or_else(|| anyhow!("unknown yarr service: {name}"))?;
        if service.base_url.trim().is_empty() {
            anyhow::bail!("{} base_url is not configured", service.name);
        }
        Ok(service)
    }
}
