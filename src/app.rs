//! Business service layer.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::{
    config::{RustarrConfig, ServiceConfig, ServiceKind},
    rustarr::{validate_safe_path, RustarrClient},
};

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct RustarrService {
    client: RustarrClient,
    services: Vec<ServiceConfig>,
}

impl RustarrService {
    pub fn new(client: RustarrClient, config: RustarrConfig) -> Self {
        Self {
            client,
            services: config.services,
        }
    }

    pub fn integrations(&self) -> Value {
        let configured: Vec<Value> = self
            .services
            .iter()
            .map(|service| {
                json!({
                    "name": service.name,
                    "kind": service.kind.as_str(),
                    "base_url_configured": !service.base_url.trim().is_empty(),
                    "api_key_configured": service.api_key.is_some(),
                    "token_configured": service.token.is_some(),
                    "username_configured": service.username.is_some(),
                    "password_configured": service.password.is_some(),
                })
            })
            .collect();
        json!({
            "supported": ServiceKind::ALL.map(ServiceKind::as_str),
            "configured": configured,
        })
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

    pub async fn api_post(
        &self,
        service: &str,
        path: &str,
        body: Value,
        confirm: bool,
    ) -> Result<Value> {
        if !confirm {
            anyhow::bail!("api_post requires confirm=true because it can mutate upstream services");
        }
        validate_safe_path(path)?;
        self.client
            .post_json(self.service(service)?, path, body)
            .await
    }

    pub async fn api_put(
        &self,
        service: &str,
        path: &str,
        body: Value,
        confirm: bool,
    ) -> Result<Value> {
        if !confirm {
            anyhow::bail!("api_put requires confirm=true because it can mutate upstream services");
        }
        validate_safe_path(path)?;
        self.client
            .put_json(self.service(service)?, path, body)
            .await
    }

    pub async fn api_delete(
        &self,
        service: &str,
        path: &str,
        body: Option<Value>,
        confirm: bool,
    ) -> Result<Value> {
        if !confirm {
            anyhow::bail!(
                "api_delete requires confirm=true because it can mutate upstream services"
            );
        }
        validate_safe_path(path)?;
        self.client
            .delete_json(self.service(service)?, path, body)
            .await
    }

    /// Resolve a configured service by name/kind and verify its capability
    /// matches `cap`. Curated command handlers (later beads) use this so a
    /// command targeting one capability cannot run against an unrelated kind.
    ///
    /// F1 scaffolding: no caller yet (the curated registry is empty), so it is
    /// `allow(dead_code)` until the first capability bead lands a handler.
    #[allow(dead_code)]
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

    fn service(&self, name: &str) -> Result<&ServiceConfig> {
        let needle = name.trim().to_ascii_lowercase();
        if needle.is_empty() {
            anyhow::bail!("service is required");
        }
        let service = self
            .services
            .iter()
            .find(|service| service.name == needle || service.kind.as_str() == needle)
            .ok_or_else(|| anyhow!("unknown rustarr service: {name}"))?;
        if service.base_url.trim().is_empty() {
            anyhow::bail!("{} base_url is not configured", service.name);
        }
        Ok(service)
    }
}
