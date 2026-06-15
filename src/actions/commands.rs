//! Curated command descriptor slices, one per capability.
//!
//! Each capability bead owns one module here that exposes a
//! `pub const <CAP>_COMMANDS: &[CommandDescriptor]` slice. The registry
//! concatenates every slice at its single extension point
//! ([`crate::actions::registry::curated_commands`]) — so adding a capability's
//! commands is: add a module here, export its slice, and append the slice at that
//! one extension point. No other module changes.

pub mod arr;
pub mod indexer;

pub use arr::ARR_COMMANDS;
pub use indexer::INDEXER_COMMANDS;
