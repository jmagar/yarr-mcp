//! Business service layer.

use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::{
    config::{RustarrConfig, ServiceConfig, ServiceKind},
    rustarr::{RustarrClient, validate_safe_path},
};

pub mod arr;
pub mod codemode;
pub mod download;
pub mod indexer;
pub mod media_server;
pub mod requests;
pub mod stats;

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct RustarrService {
    client: RustarrClient,
    services: Vec<ServiceConfig>,
    /// Root dir for Code Mode `writeArtifact` output. `None` disables artifacts
    /// (the default; the binary sets it from the data dir). Per-run subdirs are
    /// created under this root.
    data_dir: Option<std::path::PathBuf>,
}

impl RustarrService {
    pub fn new(client: RustarrClient, config: RustarrConfig) -> Self {
        Self {
            client,
            services: config.services,
            data_dir: None,
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

    /// Configured service names, in declaration order. Drives the Code Mode
    /// `api.<service>` client (one entry per configured service).
    pub(crate) fn configured_service_names(&self) -> Vec<String> {
        self.services
            .iter()
            .map(|service| service.name.clone())
            .collect()
    }

    /// Build the introspection / "catalog" payload describing configured and
    /// supported services plus the curated-command capability digest.
    ///
    /// LAYERING NOTE (P2-3): this method DELIBERATELY reads up into the `actions`
    /// registry layer (`actions::valid_actions_for_kind`,
    /// `actions::capability_digest`). The normal dependency direction is
    /// actions → app; this catalog method is the one intentional exception so the
    /// registry stays the single source of truth for "what actions a kind exposes"
    /// rather than duplicating that table here. The crossing is explicit and scoped
    /// to introspection — do NOT add business logic that depends on `actions` to
    /// other `app` methods.
    pub fn integrations(&self) -> Value {
        let configured: Vec<Value> = self
            .services
            .iter()
            .map(|service| {
                let kind = service.kind;
                json!({
                    "name": service.name,
                    "kind": kind.as_str(),
                    // Per-service capability + the actions valid for this kind so
                    // an agent can pick the right action on the first try (AN-3).
                    "capability": format!("{:?}", kind.capability()),
                    "available_actions": crate::actions::valid_actions_for_kind(kind),
                    "base_url_configured": !service.base_url.trim().is_empty(),
                    "api_key_configured": service.api_key.is_some(),
                    "token_configured": service.token.is_some(),
                    "username_configured": service.username.is_some(),
                    "password_configured": service.password.is_some(),
                })
            })
            .collect();
        // Supported kinds with their capability class, registry-derived so the
        // catalog can't drift from the capability map.
        let supported: Vec<Value> = ServiceKind::ALL
            .iter()
            .map(|kind| {
                json!({
                    "kind": kind.as_str(),
                    "capability": format!("{:?}", kind.capability()),
                })
            })
            .collect();
        let mut payload = json!({
            "supported": supported,
            "configured": configured,
        });
        // Compact capability digest of curated commands (AN-1); omitted entirely
        // when no curated commands are registered so the field can't be empty.
        if let Some(digest) = crate::actions::capability_digest() {
            payload["capability_digest"] = Value::String(digest);
        }
        payload
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
        if !confirm {
            anyhow::bail!(
                "api_delete is destructive and requires confirm=true (MCP: approve the \
                 elicitation prompt; CLI: pass --confirm)"
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
    /// Keeps `client` private to `RustarrService` while letting curated command
    /// logic in sibling modules issue requests through the shared transport.
    pub(crate) fn client_ref(&self) -> &RustarrClient {
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
            .ok_or_else(|| anyhow!("unknown rustarr service: {name}"))?;
        if service.base_url.trim().is_empty() {
            anyhow::bail!("{} base_url is not configured", service.name);
        }
        Ok(service)
    }
}
