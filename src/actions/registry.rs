//! Action registry: the SSOT for the generic action specs and the data-driven
//! curated-command descriptor table.
//!
//! Generic (infrastructure) actions live in [`ACTION_SPECS`]. Curated commands
//! live in the [`curated_commands`] table (a runtime concat of per-capability
//! const slices) of [`CommandDescriptor`]s — NOT enum variants — so each
//! capability bead can append a const slice at one extension point without
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
    // Code Mode: run a JS script that calls rustarr actions. MCP-only (a powerful
    // surface, not a casual REST passthrough; the CLI reaches it via the infra
    // verb path). Requires write scope since the script can perform writes; the
    // app layer refuses destructive deletes inside it.
    ActionSpec {
        name: "codemode",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::McpOnly,
    },
    // Snippet store verbs — persisted reusable Code Mode scripts. MCP-only (CLI via
    // the `snippet` infra verb). `snippet_list` is read; save/run/delete are write.
    // Deletes are mutating-not-destructive (operator source, recoverable), so none
    // are confirm-gated.
    ActionSpec {
        name: "snippet_list",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::McpOnly,
    },
    ActionSpec {
        name: "snippet_save",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::McpOnly,
    },
    ActionSpec {
        name: "snippet_run",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::McpOnly,
    },
    ActionSpec {
        name: "snippet_delete",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::McpOnly,
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

#[allow(dead_code)]
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

/// The JSON type a curated-command param is advertised as in the MCP tool schema.
///
/// This is the SSOT for "what JSON type does this param accept" (P2-4). The schema
/// generator in the MCP schema properties module derives each curated param's
/// `type` (and `items` for arrays) from this enum instead of a hand-written match,
/// so a new non-string param can no longer silently fall back to `string` under
/// `additionalProperties:false`. The variants mirror the parse extractors in
/// [`crate::actions::parse`]: `String`→`string_arg`/`optional_string`,
/// `Integer`→`i64_arg`/`optional_i64`, `IntegerArray`→`i64_array_arg`,
/// `StringArray`→`string_array_arg`, `Boolean`→`bool_arg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamType {
    String,
    Integer,
    IntegerArray,
    StringArray,
    Boolean,
}

impl ParamType {
    /// The JSON Schema type fragment for this param type, as a [`serde_json::Value`].
    pub fn json_schema_type(self) -> Value {
        match self {
            ParamType::String => serde_json::json!({ "type": "string" }),
            ParamType::Integer => serde_json::json!({ "type": "integer" }),
            ParamType::Boolean => serde_json::json!({ "type": "boolean" }),
            ParamType::IntegerArray => {
                serde_json::json!({ "type": "array", "items": { "type": "integer" } })
            }
            ParamType::StringArray => {
                serde_json::json!({ "type": "array", "items": { "type": "string" } })
            }
        }
    }
}

/// Future of a curated command handler.
pub type CommandFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Value>> + Send + 'a>>;

/// Handler signature for a curated command: borrows the service + args, returns a
/// boxed future. Boxing cost is negligible for network-bound calls.
pub type CommandHandler = for<'a> fn(&'a RustarrService, &'a Value) -> CommandFuture<'a>;

/// Static description of a curated, capability-scoped command. This is the SSOT
/// from which schema fragments, USAGE/HELP text, scope, and validation are all
/// derived (LD2).
///
/// `Copy` so per-capability const slices can be concatenated into the runtime
/// [`curated_commands`] table by value without clone bookkeeping (every field is
/// `Copy`: string slices, an enum, bools, and a fn pointer).
#[derive(Clone, Copy)]
pub struct CommandDescriptor {
    pub name: &'static str,
    pub capability: Capability,
    pub description: &'static str,
    pub required_scope: &'static str,
    pub required_params: &'static [&'static str],
    pub optional_params: &'static [&'static str],
    /// Whether this command is a *destructive* delete that stays gated.
    ///
    /// After the write-confirm removal, `destructive` marks the ONLY
    /// commands still gated: destructive deletes (`delete`, `download_remove`,
    /// `stats_delete_image_cache`). Plain mutating writes have
    /// `mutates: true, destructive: false` and run immediately.
    ///
    /// For a gated command the app-layer method refuses to mutate without
    /// `confirm=true` (returning a preview or erroring). Each surface obtains
    /// that confirm differently: the MCP layer elicits it
    /// ([`action_is_destructive`]), the CLI requires `--confirm`. The flag also
    /// drives schema/help annotations and is the SSOT for
    /// [`action_is_destructive`].
    pub destructive: bool,
    pub mutates: bool,
    /// The advertised JSON type of every param this command accepts
    /// (both required and optional), as `(param_name, ParamType)`.
    ///
    /// This is kept as a SEPARATE typed list rather than re-typing
    /// `required_params`/`optional_params` (which stay `&[&str]`) because the
    /// dispatch-time required-param enforcement in `actions::parse` iterates
    /// those slices as plain `&[&str]`. The
    /// schema generator derives each curated param's JSON type from this list
    /// (P2-4), so a non-string param can no longer silently fall back to
    /// `string`. Every name in `required_params`/`optional_params` (except the
    /// always-string `service`, which is declared globally) MUST appear here with
    /// the type matching its parse extractor — `tests::typed_params_cover_declared_params`
    /// in `registry_tests.rs` enforces that, and a `properties_tests.rs` test
    /// asserts the advertised schema type matches.
    pub typed_params: &'static [(&'static str, ParamType)],
    pub handler: CommandHandler,
}

/// THE single extension point for curated commands.
///
/// Each capability bead defines a per-capability const slice of
/// [`CommandDescriptor`]s under `src/actions/commands/<cap>.rs` and appends it to
/// the `concat` list below. This is the ONLY place to touch when adding a
/// capability's commands — every consumer (lookup, names, scope, schema, help,
/// validation, dispatch) flows through `curated_commands()`.
///
/// All six capability slices are registered (see the `registries` array in the
/// body): `ARR_COMMANDS`, `INDEXER_COMMANDS`, `DOWNLOAD_COMMANDS`,
/// `MEDIA_COMMANDS`, `REQUEST_COMMANDS`, and `STATS_COMMANDS`. A new capability
/// adds its slice to that array and nowhere else.
fn build_curated_commands() -> Vec<CommandDescriptor> {
    use crate::actions::commands::{
        ARR_COMMANDS, DOWNLOAD_COMMANDS, INDEXER_COMMANDS, MEDIA_COMMANDS, REQUEST_COMMANDS,
        STATS_COMMANDS,
    };

    // ── capability beads append their const slice here ───────────────────────
    let registries: &[&[CommandDescriptor]] = &[
        ARR_COMMANDS,
        INDEXER_COMMANDS,
        DOWNLOAD_COMMANDS,
        MEDIA_COMMANDS,
        REQUEST_COMMANDS,
        STATS_COMMANDS,
    ];

    registries
        .iter()
        .flat_map(|slice| slice.iter().copied())
        .collect()
}

/// All curated commands, concatenated from every capability slice once. The
/// data-driven equivalent of the F1 empty curated const, now non-empty.
pub fn curated_commands() -> &'static [CommandDescriptor] {
    static CURATED: std::sync::OnceLock<Vec<CommandDescriptor>> = std::sync::OnceLock::new();
    CURATED.get_or_init(build_curated_commands)
}

/// Lookup a curated command by name.
pub fn curated_command(name: &str) -> Option<&'static CommandDescriptor> {
    curated_commands().iter().find(|cmd| cmd.name == name)
}

/// Names of all curated commands (in registry order).
pub fn curated_command_names() -> Vec<&'static str> {
    curated_commands().iter().map(|cmd| cmd.name).collect()
}

/// The full action enum advertised to clients and the CLI: generic action specs
/// followed by every curated command name. This is the single materialization of
/// "all actions" used by the schema, help, and validation.
pub fn all_action_names() -> Vec<&'static str> {
    let mut names = action_names();
    names.extend(curated_command_names());
    names
}

/// The union of every parameter declared by any curated command
/// (`required_params` + `optional_params`), de-duplicated in first-seen order.
///
/// The MCP schema's property set is this union plus the always-present generic
/// params (`action`/`service`/`path`/`body`/`confirm`/`verbose`/`fields`), so
/// `additionalProperties:false` can stay strict while every descriptor's params
/// remain valid.
pub fn curated_param_names() -> Vec<&'static str> {
    let mut params: Vec<&'static str> = Vec::new();
    for cmd in curated_commands() {
        for p in cmd.required_params.iter().chain(cmd.optional_params) {
            if !params.contains(p) {
                params.push(p);
            }
        }
    }
    params
}

/// The advertised JSON [`ParamType`] for a curated param by name, derived from
/// the descriptor `typed_params` lists (first-seen wins, matching
/// [`curated_param_names`] dedup order). Returns `None` for params no curated
/// command declares a type for (the schema generator falls back to string).
///
/// The same param name MUST carry a consistent type across every command that
/// declares it; `tests::typed_params_are_consistent_across_commands` in
/// `registry_tests.rs` enforces that, so first-seen is unambiguous.
pub fn curated_param_type(param: &str) -> Option<ParamType> {
    for cmd in curated_commands() {
        for (name, ty) in cmd.typed_params {
            if *name == param {
                return Some(*ty);
            }
        }
    }
    None
}

/// Actions whose descriptor declares `param` as either required or optional.
///
/// This is used by MCP schema generation to annotate lifted top-level params
/// with the actions that actually consume them. Generic action params are
/// intentionally not included here; they are documented by the generic action
/// metadata and conditional required fragments.
pub fn actions_for_curated_param(param: &str) -> Vec<&'static str> {
    curated_commands()
        .iter()
        .filter(|cmd| cmd.required_params.contains(&param) || cmd.optional_params.contains(&param))
        .map(|cmd| cmd.name)
        .collect()
}

/// Required params for an action: curated commands declare them in their
/// descriptor; generic actions have their requirements encoded by
/// `generic_required_params`. Returns an empty slice when the action takes no
/// required params (or is unknown).
pub fn required_params_for_action(action: &str) -> Vec<&'static str> {
    if let Some(cmd) = curated_command(action) {
        return cmd.required_params.to_vec();
    }
    generic_required_params(action)
}

/// Required params for the generic/infra actions, mirrored into the schema
/// conditionals so MCP clients see the same contract the parser enforces.
///
/// `confirm` is NOT a required param for any generic action: plain writes
/// (`api_post`/`api_put`) no longer take it, and the destructive `api_delete`
/// obtains confirmation out-of-band (MCP elicitation / CLI `--confirm`) so the
/// schema only forces `service` + `path`.
fn generic_required_params(action: &str) -> Vec<&'static str> {
    match action {
        "service_status" => vec!["service"],
        "api_get" | "api_post" | "api_put" | "api_delete" => vec!["service", "path"],
        "codemode" => vec!["code"],
        "snippet_save" => vec!["name", "code"],
        "snippet_run" | "snippet_delete" => vec!["name"],
        _ => Vec::new(),
    }
}

/// True iff `action` is a *destructive* delete — the only actions that remain
/// gated after the write-confirm removal. Destructive curated commands carry
/// `destructive: true` in their descriptor; the generic `api_delete`
/// passthrough is destructive too. The MCP layer gates these via elicitation and
/// the CLI gates them via `--confirm`; the app layer is the final enforcement
/// point (it refuses to mutate without `confirm=true`).
pub fn action_is_destructive(action: &str) -> bool {
    if action == "api_delete" {
        return true;
    }
    curated_command(action)
        .map(|cmd| cmd.destructive)
        .unwrap_or(false)
}

/// Service kinds an action may target, by `as_str()` name. Infra actions are
/// valid for every kind; a curated command is valid for the kinds whose
/// capability matches the command's. Used to emit schema conditionals so the
/// action×kind contract is documented (server-side validation is authoritative).
#[allow(dead_code)]
pub fn allowed_kind_names_for_action(action: &str) -> Vec<&'static str> {
    if is_infra_action(action) {
        return ServiceKind::ALL.iter().map(|k| k.as_str()).collect();
    }
    match curated_command(action) {
        Some(cmd) => ServiceKind::ALL
            .iter()
            .filter(|k| k.capability() == cmd.capability)
            .map(|k| k.as_str())
            .collect(),
        None => Vec::new(),
    }
}

/// A compact, registry-derived digest of curated capabilities for embedding in
/// the tool description and help (AN-1/AN-3). Renders one line per capability
/// that has curated commands, e.g.
/// `arr(sonarr,radarr): list,set_quality | media_server(plex,jellyfin): sessions`.
///
/// Returns `None` when no curated commands are registered so callers can omit the
/// section entirely rather than print an empty digest.
pub fn capability_digest() -> Option<String> {
    use crate::capability::Capability;

    // Capabilities in a stable display order.
    const ORDER: &[(Capability, &str)] = &[
        (Capability::ArrManager, "arr"),
        (Capability::Indexer, "indexer"),
        (Capability::DownloadClient, "download_client"),
        (Capability::MediaServer, "media_server"),
        (Capability::Requests, "requests"),
        (Capability::Stats, "stats"),
    ];

    let mut segments: Vec<String> = Vec::new();
    for (cap, label) in ORDER {
        let commands: Vec<&str> = curated_commands()
            .iter()
            .filter(|cmd| cmd.capability == *cap)
            .map(|cmd| cmd.name)
            .collect();
        if commands.is_empty() {
            continue;
        }
        let kinds: Vec<&str> = ServiceKind::ALL
            .iter()
            .filter(|k| k.capability() == *cap)
            .map(|k| k.as_str())
            .collect();
        segments.push(format!(
            "{label}({}): {}",
            kinds.join(","),
            commands.join(",")
        ));
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments.join(" | "))
    }
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
        curated_commands()
            .iter()
            .filter(|cmd| cmd.capability == cap)
            .map(|cmd| cmd.name),
    );
    names
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
