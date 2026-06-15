//! Action layer facade.
//!
//! The action surface is split into submodules to keep every file <500 LOC and
//! to make the curated-command registry data-driven (so each capability bead
//! appends a const slice rather than editing a giant enum):
//!
//!   - [`model`]    — `RustarrAction`, `ActionSpec`, `ValidationError`, scopes
//!   - [`registry`] — `ACTION_SPECS`, name/scope lookups, the `CommandDescriptor`
//!     table, and `action_allowed_for_kind` validation
//!   - [`parse`]    — shared param extractors + `RustarrAction` construction
//!   - [`dispatch`] — `execute_service_action`
//!   - [`help`]     — `rest_help`
//!
//! All previously top-level items are re-exported here so `crate::actions::*`
//! imports across `cli.rs` and `mcp/*` resolve unchanged.

pub mod dispatch;
pub mod help;
pub mod model;
pub mod parse;
pub mod registry;

// ── re-exports: stable `crate::actions::` surface ───────────────────────────────

pub use dispatch::execute_service_action;
pub use help::rest_help;
pub use model::{
    is_validation_error, scopes_satisfy, ActionSpec, ActionTransport, RustarrAction,
    ValidationError, DENY_SCOPE, READ_SCOPE, WRITE_SCOPE,
};
pub use parse::{bool_arg, optional_string, string_arg};
pub use registry::{
    action_allowed_for_kind, action_names, curated_command, is_known_action, is_rest_action,
    mcp_only_action_names, required_scope_for_action, rest_action_names, valid_actions_for_kind,
    CommandDescriptor, CommandFuture, CommandHandler, ACTION_SPECS, CURATED_COMMANDS,
};
