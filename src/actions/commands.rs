//! Curated command descriptor slices, one per capability.
//!
//! Each capability bead owns one module here that exposes a
//! `pub const <CAP>_COMMANDS: &[CommandDescriptor]` slice. The registry
//! concatenates every slice at its single extension point
//! ([`crate::actions::registry::curated_commands`]) — so adding a capability's
//! commands is: add a module here, export its slice, and append the slice at that
//! one extension point. No other module changes.

// Only the doc-based capabilities keep curated commands. The 4 spec-backed
// capabilities (ArrManager, Indexer, Requests, MediaServer) are fully served by
// generated OpenAPI operations via Code Mode — no curated commands.
pub mod download;
pub mod stats;

pub use download::DOWNLOAD_COMMANDS;
pub use stats::STATS_COMMANDS;

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
