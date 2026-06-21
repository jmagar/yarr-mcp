//! Destructive-delete elicitation gate (MCP-only).
//!
//! After the write-confirm removal, the ONLY actions still gated are destructive
//! deletes ([`crate::actions::action_is_destructive`]). On the MCP surface that
//! gate is satisfied by *elicitation* (rmcp 1.7 [`Peer::elicit_with_timeout`]):
//! before a destructive action runs, the server asks the client to confirm. The
//! CLI has no elicitation channel and uses `--confirm` instead; either way the
//! app-layer method is the final enforcement point (it refuses to mutate without
//! `confirm=true`).
//!
//! This lives in the MCP protocol layer (not `tools.rs` / the app layer) because
//! elicitation needs the client [`Peer`], exactly like the scope checks in
//! `rmcp_server.rs`. `tools.rs` stays a thin dispatcher.

use std::time::Duration;

use rmcp::{
    RoleServer,
    service::{ElicitationError, Peer},
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

/// Max time to wait for the user to answer a destructive-delete prompt. On expiry
/// the elicit call returns a timeout error which `normalize` treats as `Refused`
/// — a stuck prompt fails safe (no delete) instead of holding the request open
/// indefinitely.
const ELICIT_TIMEOUT: Duration = Duration::from_secs(300);

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
    /// The client cannot elicit and no explicit `confirm` was passed — abstain
    /// and let dispatch run so the app layer returns its needs-confirm response
    /// (the caller can then re-issue with `confirm=true`).
    Abstain,
}

/// The outcome of an elicitation round-trip, normalized away from rmcp's
/// `Result<Option<_>, ElicitationError>`. This intermediate exists so the gate
/// decision ([`classify`]) is a pure, fully unit-testable function: rmcp's
/// `ElicitationError` is `#[non_exhaustive]` and cannot be constructed by a
/// downstream crate, so the error arms of [`normalize`] are not directly
/// testable — but everything downstream of this enum is.
#[derive(Debug, PartialEq, Eq)]
enum ElicitOutcome {
    /// User explicitly approved (`confirm = true`).
    Confirmed,
    /// Fail-safe bucket: user declined/cancelled, sent `confirm=false`/no content,
    /// or the prompt failed for any reason (parse/transport/timeout).
    Refused,
    /// The client does not support elicitation at all.
    Unsupported,
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

/// Pure decision: map a normalized [`ElicitOutcome`] to a [`DeleteGate`]. Fully
/// unit-testable (no `Peer`, no rmcp error types).
fn classify(outcome: ElicitOutcome) -> DeleteGate {
    match outcome {
        ElicitOutcome::Confirmed => DeleteGate::Proceed,
        ElicitOutcome::Refused => DeleteGate::Declined,
        ElicitOutcome::Unsupported => DeleteGate::Abstain,
    }
}

/// Normalize an rmcp elicit result to an [`ElicitOutcome`]. The non-`Confirmed`
/// collapse is the fail-safe default: anything other than an explicit
/// `confirm=true` (decline, cancel, empty, parse/transport error, timeout)
/// becomes `Refused`; only a declared missing capability is `Unsupported`. The
/// `Ok` arms are unit-tested; the `Err` arms cannot be (non-constructible
/// `#[non_exhaustive]` error), so they are kept to a trivial, obviously-safe
/// match.
fn normalize(result: Result<Option<DeleteConfirmation>, ElicitationError>) -> ElicitOutcome {
    match result {
        Ok(Some(DeleteConfirmation { confirm: true })) => ElicitOutcome::Confirmed,
        Ok(Some(DeleteConfirmation { confirm: false })) | Ok(None) => ElicitOutcome::Refused,
        Err(ElicitationError::CapabilityNotSupported) => ElicitOutcome::Unsupported,
        Err(_) => ElicitOutcome::Refused,
    }
}

/// Gate a destructive `action` targeting `service` on the MCP surface.
///
/// 1. `confirm=true` already present → [`DeleteGate::Proceed`].
/// 2. Client can't elicit → [`DeleteGate::Abstain`].
/// 3. Otherwise prompt (with a timeout) and map the outcome ([`normalize`] +
///    [`classify`]).
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
        return DeleteGate::Abstain;
    }
    let result = peer
        .elicit_with_timeout::<DeleteConfirmation>(
            confirm_message(action, service),
            Some(ELICIT_TIMEOUT),
        )
        .await;
    classify(normalize(result))
}

#[cfg(test)]
#[path = "elicit_tests.rs"]
mod tests;
