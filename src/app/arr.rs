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
//!     and the apply-summary builder; unit-tested directly.
//!   * [`write`](mod@self::write)   — editor-based WRITE command methods (C2):
//!     set_quality, monitor/unmonitor, add (immediate, count-capped) and the
//!     destructive `delete` (confirm-gated + count-capped).
//!   * [`command`](mod@self::command)  — async `/command`-intent methods
//!     (search/refresh) and their started-job helper — split out of `write` to
//!     keep each file under cap.
//!
//! The curated-command *descriptors* (registry table) live in
//! `src/actions/commands/arr.rs`, not here — this module only holds logic.

pub mod command;
pub mod editor;
pub mod read;
pub mod resolve;
pub mod write;

#[cfg(test)]
#[path = "arr_tests.rs"]
mod tests;
