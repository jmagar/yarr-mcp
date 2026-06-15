//! ArrManager capability: business-layer methods for Sonarr/Radarr (C1) and the
//! shared name→id resolvers reused by write/intent commands (C2).
//!
//! Split per the architecture C2-a pattern so each sub-bead appends to its own
//! file instead of re-splitting mid-epic:
//!   * [`read`]    — READ commands (C1): quality_profiles, list, wanted, queue,
//!     history, rootfolders, health.
//!   * [`resolve`] — name→id resolution (e.g. quality-profile name → id) shared
//!     by C2 write commands. Pre-created now.
//!
//! Write/intent commands (C2) will add a sibling `write.rs` here. The
//! curated-command *descriptors* (registry table) live in
//! `src/actions/commands/arr.rs`, not here — this module only holds logic.

pub mod read;
pub mod resolve;
