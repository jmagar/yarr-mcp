//! Business service layer.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::{
    config::{RustarrConfig, ServiceConfig, ServiceKind},
    rustarr::{validate_safe_path, RustarrClient},
    scaffold::{build_scaffold_intent, ScaffoldIntent},
};

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct RustarrService {
    client: RustarrClient,
    services: Vec<ServiceConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElicitedNameOutcome<'a> {
    Accepted(&'a str),
    NoInput,
    Declined,
    Cancelled,
    Unsupported,
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

    pub fn elicited_name_greeting(&self, outcome: ElicitedNameOutcome<'_>) -> Value {
        match outcome {
            ElicitedNameOutcome::Accepted(name) => {
                let name = name.trim().to_owned();
                if name.is_empty() {
                    json!({
                        "greeting": "Hello, mysterious stranger!",
                        "note": "You submitted an empty name.",
                    })
                } else {
                    json!({
                        "greeting": format!("Hello, {name}! Welcome to rustarr."),
                        "name": name,
                    })
                }
            }
            ElicitedNameOutcome::NoInput => json!({
                "greeting": "Hello. No name was provided.",
            }),
            ElicitedNameOutcome::Declined => json!({
                "message": "No problem. You chose not to share your name.",
                "greeting": "Hello, anonymous user.",
            }),
            ElicitedNameOutcome::Cancelled => json!({
                "message": "Elicitation was cancelled.",
                "greeting": "Hello.",
            }),
            ElicitedNameOutcome::Unsupported => json!({
                "message": "Elicitation is not supported by this MCP client.",
                "hint": "Use a client that supports MCP elicitation, or call action=help for non-elicitation actions.",
                "fallback_greeting": "Hello.",
            }),
        }
    }

    pub fn scaffold_intent(&self, input: ScaffoldIntent) -> Result<Value> {
        build_scaffold_intent(input)
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
