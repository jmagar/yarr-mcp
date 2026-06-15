//! Per-capability CLI parse modules for curated commands.
//!
//! Each capability bead adds one module here that exposes a `parse(kind, verb,
//! rest) -> Result<Option<Command>>` function. The router's
//! [`parse_capability_command`](crate::cli::router::parse_capability_command)
//! hook dispatches to the right module by capability, falling through to its
//! generic-verb handling when a module returns `Ok(None)`.

pub mod arr;
