//! Plugin setup and repair commands.
//!
//! These are operational commands that check and mutate appdata, write .env
//! files, and validate auth/port configuration before the server starts.
//! Business logic stays in `app.rs`; this module is allowed to touch the
//! filesystem and network only for diagnostics and setup purposes.
//!
//! This module is a facade: the implementation is split by concern into the
//! `setup/` submodules and re-exported here so existing import paths
//! (`crate::cli::setup::*`) keep working unchanged.

mod plugin;
mod wizard;

pub use plugin::apply_plugin_options;
pub use wizard::{run_setup, SetupCommand};

// Re-export internal items used by the colocated tests via `super::*`/`super::`.
#[cfg(test)]
pub(crate) use wizard::{dotenv_assignment, setup_check, setup_repair, SetupFailure, SetupReport};

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
