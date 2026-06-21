//! Destructive-delete elicitation gate (MCP-only).
//!
//! After the write-confirm removal, the ONLY actions still gated are destructive
//! deletes ([`crate::actions::action_is_destructive`]). On the MCP surface that
//! gate is satisfied by *elicitation* (rmcp 1.7 [`Peer::elicit`]): before a
//! destructive action runs, the server asks the client to confirm. The CLI has
//! no elicitation channel and uses `--confirm` instead; either way the app-layer
//! method is the final enforcement point (it refuses to mutate without
//! `confirm=true`).
//!
//! This lives in the MCP protocol layer (not `tools.rs` / the app layer) because
//! elicitation needs the client [`Peer`], exactly like the scope checks in
//! `rmcp_server.rs`. `tools.rs` stays a thin dispatcher.

use rmcp::{
    RoleServer,
    service::{ElicitationError, Peer},
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

/// Structured payload requested from the user for a destructive delete. A single
/// boolean: the client renders a confirm prompt from the generated schema.
/// `Accept` with `confirm=true` proceeds; anything else aborts.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DeleteConfirmation {
    /// Set true to confirm and run this destructive delete.
    pub confirm: bool,
}

rmcp::elicit_safe!(DeleteConfirmation);

/// How a destructive action should be handled on the MCP surface.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DeleteGate {
    /// Confirm is established — run the action (caller injects `confirm=true`).
    Proceed,
    /// The user declined/cancelled (or the prompt failed) — do NOT run it.
    Declined,
    /// The client cannot elicit and no explicit `confirm` was passed — let
    /// dispatch run so the app layer returns its needs-confirm response (the
    /// caller can then re-issue with `confirm=true`).
    NoConfirmChannel,
}

/// Whether the caller already authorized the destructive action by passing
/// `confirm=true` (covers automation and the trusted-gateway path). Pure.
pub(crate) fn preauthorized(args: &Value) -> bool {
    args.get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// The elicitation prompt shown to the user before a destructive delete.
pub(crate) fn confirm_message(action: &str, service: &str) -> String {
    format!(
        "Confirm destructive action '{action}' on service '{service}'. This permanently \
         deletes data and cannot be undone. Approve to proceed."
    )
}

/// Map an [`Peer::elicit`] result to a [`DeleteGate`]. Factored out (pure over the
/// `Ok` arms) so the accept/decline decision is unit-testable without a live
/// peer. An accepted-but-`confirm:false`, an empty response, an explicit
/// decline/cancel, or any service/parse error all fail safe to `Declined` — a
/// destructive op never runs on an ambiguous signal. Only a missing capability
/// falls back to `NoConfirmChannel`.
fn interpret(result: Result<Option<DeleteConfirmation>, ElicitationError>) -> DeleteGate {
    match result {
        Ok(Some(DeleteConfirmation { confirm: true })) => DeleteGate::Proceed,
        Ok(Some(DeleteConfirmation { confirm: false })) | Ok(None) => DeleteGate::Declined,
        Err(ElicitationError::CapabilityNotSupported) => DeleteGate::NoConfirmChannel,
        Err(_) => DeleteGate::Declined,
    }
}

/// Gate a destructive `action` targeting `service` on the MCP surface.
///
/// 1. `confirm=true` already present → [`DeleteGate::Proceed`].
/// 2. Client can't elicit → [`DeleteGate::NoConfirmChannel`].
/// 3. Otherwise ask the user and interpret the response ([`interpret`]).
pub(crate) async fn gate_destructive(
    peer: &Peer<RoleServer>,
    action: &str,
    service: &str,
    args: &Value,
) -> DeleteGate {
    if preauthorized(args) {
        return DeleteGate::Proceed;
    }
    if peer.supported_elicitation_modes().is_empty() {
        return DeleteGate::NoConfirmChannel;
    }
    interpret(
        peer.elicit::<DeleteConfirmation>(confirm_message(action, service))
            .await,
    )
}

#[cfg(test)]
#[path = "elicit_tests.rs"]
mod tests;
