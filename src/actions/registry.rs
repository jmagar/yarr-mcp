//! Action registry: the SSOT for the generic action specs and the data-driven
//! curated-command descriptor table.
//!
//! Generic (infrastructure) actions live in [`ACTION_SPECS`]. Curated commands
//! live in [`CURATED_COMMANDS`] as a data table of [`CommandDescriptor`]s — NOT
//! enum variants — so each capability bead can append a const slice without
//! editing a giant match/enum (keeps every file <500 LOC and avoids merge
//! collisions between parallel beads).

use serde_json::Value;

use super::model::{ActionSpec, ActionTransport, DENY_SCOPE, READ_SCOPE, WRITE_SCOPE};
use crate::app::RustarrService;
use crate::capability::Capability;
use crate::config::ServiceKind;

// ── generic action specs ────────────────────────────────────────────────────────

pub const ACTION_SPECS: &[ActionSpec] = &[
    ActionSpec {
        name: "integrations",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "service_status",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "api_get",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "api_post",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "api_put",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "api_delete",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "help",
        required_scope: None,
        transport: ActionTransport::Any,
    },
];

pub fn action_names() -> Vec<&'static str> {
    ACTION_SPECS.iter().map(|spec| spec.name).collect()
}

pub fn is_known_action(action: &str) -> bool {
    ACTION_SPECS.iter().any(|spec| spec.name == action) || curated_command(action).is_some()
}

pub fn rest_action_names() -> Vec<&'static str> {
    ACTION_SPECS
        .iter()
        .filter(|spec| spec.transport == ActionTransport::Any)
        .map(|spec| spec.name)
        .collect()
}

pub fn is_rest_action(action: &str) -> bool {
    action_spec(action)
        .map(|spec| spec.transport == ActionTransport::Any)
        .unwrap_or(false)
}

pub fn mcp_only_action_names() -> Vec<&'static str> {
    ACTION_SPECS
        .iter()
        .filter(|spec| spec.transport == ActionTransport::McpOnly)
        .map(|spec| spec.name)
        .collect()
}

pub fn required_scope_for_action(action: &str) -> Option<&'static str> {
    if let Some(spec) = action_spec(action) {
        return spec.required_scope;
    }
    if let Some(cmd) = curated_command(action) {
        return Some(cmd.required_scope);
    }
    Some(DENY_SCOPE)
}

pub(super) fn action_spec(action: &str) -> Option<&'static ActionSpec> {
    ACTION_SPECS.iter().find(|spec| spec.name == action)
}

// ── curated command descriptor table (data-driven, not an enum) ──────────────────

/// Future of a curated command handler.
pub type CommandFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Value>> + Send + 'a>>;

/// Handler signature for a curated command: borrows the service + args, returns a
/// boxed future. Boxing cost is negligible for network-bound calls.
pub type CommandHandler = for<'a> fn(&'a RustarrService, &'a Value) -> CommandFuture<'a>;

/// Static description of a curated, capability-scoped command. This is the SSOT
/// from which schema fragments, USAGE/HELP text, scope, and validation are all
/// derived (LD2).
pub struct CommandDescriptor {
    pub name: &'static str,
    pub capability: Capability,
    pub description: &'static str,
    pub required_scope: &'static str,
    pub required_params: &'static [&'static str],
    pub optional_params: &'static [&'static str],
    pub confirm_required: bool,
    pub mutates: bool,
    pub handler: CommandHandler,
}

/// THE single extension point for curated commands.
///
/// Later capability beads append their per-capability const slices here, e.g.
/// `concat_slices!(ARR_READ_COMMANDS, INDEXER_COMMANDS, ...)`. Empty for F1 —
/// the scaffolding exists and is wired through validation/scope/lookup, ready
/// to receive commands without touching any other module.
pub const CURATED_COMMANDS: &[CommandDescriptor] = &[];

/// Lookup a curated command by name.
pub fn curated_command(name: &str) -> Option<&'static CommandDescriptor> {
    CURATED_COMMANDS.iter().find(|cmd| cmd.name == name)
}

// ── action × kind validation (LD4, fail-closed) ─────────────────────────────────

/// All generic/infra actions are valid for every kind.
fn is_infra_action(action: &str) -> bool {
    action_spec(action).is_some()
}

/// True iff `action` may be run against a service of `kind`.
///
/// Infra/generic actions are allowed for ALL kinds. A curated command is allowed
/// iff its capability matches the kind's capability. Unknown actions fail closed.
pub fn action_allowed_for_kind(action: &str, kind: ServiceKind) -> bool {
    if is_infra_action(action) {
        return true;
    }
    match curated_command(action) {
        Some(cmd) => cmd.capability == kind.capability(),
        None => false,
    }
}

/// The set of action names valid for a given kind: all infra actions plus any
/// curated commands whose capability matches the kind.
pub fn valid_actions_for_kind(kind: ServiceKind) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = action_names();
    let cap = kind.capability();
    names.extend(
        CURATED_COMMANDS
            .iter()
            .filter(|cmd| cmd.capability == cap)
            .map(|cmd| cmd.name),
    );
    names
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
