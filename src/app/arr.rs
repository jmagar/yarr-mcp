//! ArrManager capability: business-layer methods for Sonarr/Radarr (C1) and the
//! shared name→id resolvers reused by write/intent commands (C2).
//!
//! Split per the architecture C2-a pattern so each sub-bead appends to its own
//! file instead of re-splitting mid-epic:
//!   * [`read`]    — READ commands (C1): quality_profiles, list, wanted, queue,
//!     history, rootfolders, health.
//!   * [`resolve`] — name→id resolution (e.g. quality-profile name → id) shared
//!     by C2 write commands.
//!   * [`editor`]  — pure (no self/network) body builders, selectors, count cap,
//!     and dry-run preview for the C2 write commands; unit-tested directly.
//!   * [`write`](mod@self::write)   — WRITE/intent command methods (C2): set_quality, search,
//!     refresh, monitor/unmonitor, add, delete — confirm-gated + count-capped.
//!
//! The curated-command *descriptors* (registry table) live in
//! `src/actions/commands/arr.rs`, not here — this module only holds logic.

pub mod editor;
pub mod read;
pub mod resolve;
pub mod write;

#[cfg(test)]
#[path = "arr_tests.rs"]
mod tests;
