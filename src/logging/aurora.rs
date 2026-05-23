//! Aurora color palette — ANSI 256 constants for log formatting.
//!
//! # TEMPLATE: Why a separate color module?
//!
//! These constants are the single source of truth for color values across
//! ALL rendering surfaces in this server:
//!   - Console log formatting (`formatter.rs`)
//!   - CLI output theming (if any)
//!   - Future UI surfaces
//!
//! **Do not inline ANSI codes elsewhere.** Always reference `aurora::CONSTANT`.
//! This makes palette changes a one-line edit.
//!
//! # TEMPLATE: Copy these constants EXACTLY
//!
//! The values below match `lab/crates/lab/src/output/theme.rs` exactly.
//! When adapting this template for your service, copy this file unchanged.
//! The aurora palette is shared across the entire rmcp server family:
//! unrust, rustify, rustifi, rustscale, apprise-mcp, and rustarr.
//!
//! # ANSI 256 vs TrueColor
//!
//! The log formatter uses ANSI 256 (not TrueColor) because:
//! - ANSI 256 is supported by virtually every terminal emulator
//! - Docker's `docker compose logs` strips TrueColor but keeps ANSI 256
//! - The `console` crate used in lab only supports ANSI 256
//!
//! The RGB values shown in comments are the closest-matching TrueColor
//! equivalents for documentation purposes only.

/// Pink — used for service names, first token of log messages.
///
/// RGB equivalent: (255, 175, 215) — soft pink
///
/// Used for:
/// - The first word of every log message (the "action verb")
/// - The `service` structured field value
///
/// # Visual rustarr (approximate)
/// ```text
/// HH:MM:SS  INFO  starting  bind=0.0.0.0:3000  auth=bearer
///           ────  ────────
///           plain  aurora::SERVICE_NAME (pink, bold)
/// ```
pub const SERVICE_NAME: u8 = 211;

/// Bright blue — used for primary action/route/tool identifiers.
///
/// RGB equivalent: (41, 182, 246) — sky blue
///
/// Used for structured field values where the value identifies the
/// primary action being taken:
/// - `action=greet` → "greet" in blue
/// - `tool=rustarr` → "rustarr" in blue
/// - `route=/health` → "/health" in blue
/// - `addr=0.0.0.0:3000` → "0.0.0.0:3000" in blue
pub const ACCENT_PRIMARY: u8 = 39;

/// Light grey — used for secondary metadata and muted text.
///
/// RGB equivalent: (167, 188, 201) — cool grey
///
/// Used for:
/// - Subsystem names: `subsystem=mcp`
/// - Phase names: `phase=startup`
/// - Transport names: `transport=streamable-http`
/// - Operation names: `operation=list`
pub const TEXT_MUTED: u8 = 250;

/// Teal — used for success states and HTTP 2xx status codes.
///
/// RGB equivalent: (125, 211, 199) — seafoam teal
///
/// Used for:
/// - `status=200` → "200" in teal
/// - `status=201` → "201" in teal
/// - Any HTTP status < 300
pub const SUCCESS: u8 = 115;

/// Amber — used for warnings and HTTP 4xx status codes.
///
/// RGB equivalent: (198, 163, 107) — warm amber
///
/// Used for:
/// - `WARN` level label (bold)
/// - `status=404` → "404" in amber
/// - `status=429` → "429" in amber
/// - Any HTTP status 300–499
/// - `kind` field on WARN/ERROR events
pub const WARN: u8 = 180;

/// Muted red — used for errors and HTTP 5xx status codes.
///
/// RGB equivalent: (199, 132, 144) — rose/muted red
///
/// Used for:
/// - `ERROR` level label (bold)
/// - `error=<message>` field value
/// - `status=500` → "500" in muted red
/// - Any HTTP status >= 500
///
/// # Why muted red instead of bright red?
///
/// Bright red (\x1b[31m) is harsh and hard to read in log streams.
/// Aurora uses muted red (ANSI 174) for better readability while still
/// clearly indicating error state. It's noticeable without being alarming.
pub const ERROR: u8 = 174;

#[cfg(test)]
#[path = "aurora_tests.rs"]
mod tests;
