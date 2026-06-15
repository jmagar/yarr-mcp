//! Dispatch: map a parsed `RustarrAction` to the corresponding service method.

use anyhow::Result;
use serde_json::Value;

use super::help::rest_help;
use super::model::RustarrAction;
use crate::app::RustarrService;

pub async fn execute_service_action(
    service: &RustarrService,
    action: &RustarrAction,
) -> Result<Value> {
    match action {
        RustarrAction::Integrations => Ok(service.integrations()),
        RustarrAction::ServiceStatus { service: name } => service.service_status(name).await,
        RustarrAction::ApiGet {
            service: name,
            path,
        } => service.api_get(name, path).await,
        RustarrAction::ApiPost {
            service: name,
            path,
            body,
            confirm,
        } => service.api_post(name, path, body.clone(), *confirm).await,
        RustarrAction::ApiPut {
            service: name,
            path,
            body,
            confirm,
        } => service.api_put(name, path, body.clone(), *confirm).await,
        RustarrAction::ApiDelete {
            service: name,
            path,
            body,
            confirm,
        } => service.api_delete(name, path, body.clone(), *confirm).await,
        RustarrAction::Help => Ok(rest_help()),
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
