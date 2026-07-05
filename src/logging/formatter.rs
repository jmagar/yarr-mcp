//! Aurora console log formatter — pretty, colored, human-readable.
//!
//! # Shared formatter
//!
//! This is Yarr's Aurora log formatter, shared with the rmcp server family.
//! It mirrors `lab/crates/lab/src/log_fmt/formatter.rs` exactly so that
//! all servers in the family produce identically-formatted console logs.
//!
//! Adjust `style_value()` in `logging/formatter/style.rs` if Yarr gains
//! additional semantic tracing field names.
//!
//! # Log format produced
//!
//! ```text
//! HH:MM:SS  INFO  starting  bind=0.0.0.0:3000  auth=bearer
//! HH:MM:SS  INFO  tool call  action=codemode  elapsed_ms=12
//! HH:MM:SS  WARN  upstream slow  action=service_status  elapsed_ms=3200
//! HH:MM:SS ERROR  upstream failed  action=api_get  error="connection refused"
//! ```
//!
//! Columns:
//! - `HH:MM:SS` — local time, dim grey
//! - `LEVEL   ` — 5 chars wide; ERROR=bold red, WARN=bold amber, INFO=plain, DEBUG/TRACE=dim
//! - `message`  — first token in pink+bold, inline `key=val` tokens get dim key
//! - `key=val`  — priority fields first, then alphabetical; keys dim, values semantic-colored
//!
//! # Why a custom formatter instead of tracing_subscriber::fmt defaults?
//!
//! The default tracing subscriber writes structured fields in a format like:
//! ```text
//! 2026-05-13T14:32:01.123456Z  INFO yarr: starting  bind="0.0.0.0:3000"
//! ```
//!
//! Problems with the default:
//! - Full ISO timestamp is verbose (our HH:MM:SS is sufficient for dev logs)
//! - Module path (`yarr:`) adds noise
//! - String values are always quoted (our formatter only quotes whitespace-containing values)
//! - No semantic coloring for field values
//!
//! The `AuroraFormatter` fixes all of these while staying compatible with
//! tracing's `FormatEvent` trait so it slots into the standard subscriber stack.
//!
//! This module is a facade: the styling primitives live in `formatter::style`
//! and the `FormatEvent` impl in `formatter::render`, re-exported here so the
//! public surface stays at `crate::logging::formatter::*`.

mod render;
mod style;

pub use render::AuroraFormatter;

// Bring the value helpers into this module's scope so the colocated tests can
// reach them via `super::*` / `super::name`. They are `pub(crate)` in `style`.
#[cfg(test)]
pub(crate) use style::{format_field_value, sanitize_field_value, should_skip_field};

#[cfg(test)]
#[path = "formatter_tests.rs"]
mod tests;
