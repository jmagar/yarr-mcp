//! Per-capability CLI parse modules for curated commands.
//!
//! Each capability bead adds one module here that exposes a `parse(kind, verb,
//! rest) -> Result<Option<Command>>` function. The router's
//! [`parse_capability_command`](crate::cli::router::parse_capability_command)
//! hook dispatches to the right module by capability, falling through to its
//! generic-verb handling when a module returns `Ok(None)`.

pub mod arr;
pub mod download;
pub mod indexer;
pub mod media_server;
pub mod requests;
pub mod stats;

use crate::capability::Capability;

/// Per-capability friendly verb tables, paired with their [`Capability`]. Each
/// module's `VERBS` const is the SSOT for `(friendly CLI verb → registry action
/// name)`; this table joins them with their owning capability so the USAGE
/// renderer can group friendly verbs by service and the mechanical CLI↔MCP parity
/// test (`tests/parity.rs`) can assert every curated descriptor is CLI-reachable.
pub fn capability_verb_tables() -> &'static [(Capability, &'static [(&'static str, &'static str)])]
{
    &[
        (Capability::ArrManager, arr::VERBS),
        (Capability::Indexer, indexer::VERBS),
        (Capability::DownloadClient, download::VERBS),
        (Capability::MediaServer, media_server::VERBS),
        (Capability::Requests, requests::VERBS),
        (Capability::Stats, stats::VERBS),
    ]
}

/// The friendly CLI verb that maps to a curated registry `action` name, if any
/// capability declares one. Used by USAGE to render the friendly verb (e.g.
/// `activity`) instead of the snake/kebab of the action name (`stats-activity`).
pub fn cli_verb_for_action(action: &str) -> Option<&'static str> {
    capability_verb_tables()
        .iter()
        .flat_map(|(_, verbs)| verbs.iter())
        .find(|(_, act)| *act == action)
        .map(|(verb, _)| *verb)
}
