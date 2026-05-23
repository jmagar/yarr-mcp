//! Aurora console log formatter — pretty, colored, human-readable.
//!
//! # TEMPLATE: Reference implementation
//!
//! This is the canonical log formatter for the rmcp server family.
//! It mirrors `lab/crates/lab/src/log_fmt/formatter.rs` exactly so that
//! all servers in the family produce identically-formatted console logs.
//!
//! When adapting this template for your service:
//! 1. Copy this file unchanged — it needs no service-specific edits
//! 2. Adjust `style_value()` if you have additional semantic field names
//!
//! # Log format produced
//!
//! ```text
//! HH:MM:SS  INFO  starting  bind=0.0.0.0:3000  auth=bearer
//! HH:MM:SS  INFO  tool call  action=greet  elapsed_ms=12
//! HH:MM:SS  WARN  upstream slow  action=status  elapsed_ms=3200
//! HH:MM:SS ERROR  upstream failed  action=echo  error="connection refused"
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
//! 2026-05-13T14:32:01.123456Z  INFO rmcp_template: starting  bind="0.0.0.0:3000"
//! ```
//!
//! Problems with the default:
//! - Full ISO timestamp is verbose (our HH:MM:SS is sufficient for dev logs)
//! - Module path (`rmcp_template:`) adds noise
//! - String values are always quoted (our formatter only quotes whitespace-containing values)
//! - No semantic coloring for field values
//!
//! The `AuroraFormatter` fixes all of these while staying compatible with
//! tracing's `FormatEvent` trait so it slots into the standard subscriber stack.

use std::collections::BTreeMap;
use std::fmt as stdfmt;

use tracing::{
    field::{Field, Visit},
    Event, Subscriber,
};
use tracing_subscriber::{
    fmt::{
        format::{FormatEvent, FormatFields, Writer},
        FmtContext,
    },
    registry::LookupSpan,
};

use super::aurora;

// ── Raw ANSI helpers ──────────────────────────────────────────────────────────
//
// TEMPLATE: We use raw ANSI codes rather than the `console` crate because:
// 1. `console::colors_enabled()` checks stdout's TTY state, ignoring our
//    ANSI flag (which is set based on stderr's TTY state)
// 2. We want ANSI 256, not TrueColor — ANSI 256 survives `docker compose logs`
//
// These helpers are intentionally private — callers use `aurora::CONSTANT`
// for the color values and call these helpers for the formatting.

/// Apply ANSI 256 foreground color to `text`.
fn ansi256(n: u8, text: &str) -> String {
    format!("\x1b[38;5;{n}m{text}\x1b[0m")
}

/// Apply ANSI 256 foreground color + bold to `text`.
fn ansi256_bold(n: u8, text: &str) -> String {
    format!("\x1b[1;38;5;{n}m{text}\x1b[0m")
}

/// Apply ANSI dim (low intensity) to `text`.
fn ansi_dim(text: &str) -> String {
    format!("\x1b[2m{text}\x1b[0m")
}

// ── Field collection ──────────────────────────────────────────────────────────

/// Collects all structured fields from a tracing `Event` into a `BTreeMap`.
///
/// Using `BTreeMap` gives us two properties:
/// 1. Deterministic iteration order (alphabetical) for remaining fields
/// 2. O(log n) `take()` for extracting priority fields in a fixed order
///
/// # TEMPLATE: Adding custom field types
///
/// If you add custom tracing fields with types beyond str/bool/i64/u64/f64,
/// override `record_debug` — tracing calls it for any type implementing `Debug`.
#[derive(Default)]
struct EventFieldCollector {
    fields: BTreeMap<&'static str, String>,
}

impl EventFieldCollector {
    fn insert(&mut self, field: &Field, value: String) {
        self.fields.insert(field.name(), value);
    }

    /// Remove and return a field by key (used to extract priority fields first).
    fn take(&mut self, key: &str) -> Option<String> {
        self.fields.remove(key)
    }
}

impl Visit for EventFieldCollector {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert(field, value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field, value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.insert(field, value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.insert(field, format!("{value:?}"));
    }
}

// ── ANSI injection prevention ─────────────────────────────────────────────────

/// Strip Unicode control characters from upstream-controlled field values.
///
/// # Why this matters
///
/// Field values come from external sources (API responses, user input, etc.).
/// A malicious upstream could inject ANSI escape sequences into field values,
/// causing the log output to color arbitrary text or hide log entries.
///
/// # TEMPLATE: Injection attack example
///
/// Without sanitization, this log line:
/// ```text
/// info!(error = "\x1b[31mFAKE\x1b[0m", "upstream failed")
/// ```
/// would inject red "FAKE" text into adjacent log output.
///
/// # What we strip
///
/// All Unicode control characters EXCEPT:
/// - Tab (`\t` / 0x09) — preserved, valid in field values
/// - Newline (`\n` / 0x0A) — preserved, valid in multiline values
///
/// ESC (`\x1b` / 0x1B) — stripped, replaced with Unicode replacement char (U+FFFD)
/// Other C0 controls — stripped, replaced with U+FFFD
///
/// # Performance
///
/// Returns `Cow::Borrowed` when the input is clean (zero allocation).
/// Only allocates when control characters are found.
pub(crate) fn sanitize_field_value(value: &str) -> std::borrow::Cow<'_, str> {
    if value
        .chars()
        .any(|c| c.is_control() && c != '\t' && c != '\n')
    {
        std::borrow::Cow::Owned(
            value
                .chars()
                .map(|c| {
                    if c.is_control() && c != '\t' && c != '\n' {
                        '\u{FFFD}'
                    } else {
                        c
                    }
                })
                .collect(),
        )
    } else {
        std::borrow::Cow::Borrowed(value)
    }
}

/// Quote values that contain whitespace (otherwise leave bare).
///
/// This matches Rust's Debug format for strings: only quote when necessary.
/// `"hello"` → `hello` (no quotes)
/// `"hello world"` → `"hello world"` (quoted)
pub(crate) fn format_field_value(value: &str) -> String {
    if value.contains(char::is_whitespace) {
        format!("{value:?}")
    } else {
        value.to_string()
    }
}

/// Return true if this field+value should be suppressed in output.
///
/// # TEMPLATE: Noise suppression
///
/// Some boolean fields are always emitted even when false. Suppressing
/// `field=false` reduces noise for fields that only matter when true.
///
/// Add additional suppression rules here as needed for your service.
pub(crate) fn should_skip_field(key: &str, value: &str) -> bool {
    // Suppress boolean flags that are always present — only show when true
    matches!((key, value), ("subject_scoped" | "destructive", "false"))
}

// ── Semantic field coloring ───────────────────────────────────────────────────

/// Apply aurora palette colors to structured field values based on field name.
///
/// # TEMPLATE: Semantic coloring rules
///
/// The color applied depends on the field's *semantic role*, not its value.
/// This gives operators an immediate visual hierarchy:
///
/// | Color  | Role        | Fields                                          |
/// |--------|-------------|------------------------------------------------|
/// | Pink   | Identity    | `service`                                       |
/// | Blue   | Action/path | `action`, `tool`, `route`, `addr`, etc.         |
/// | Grey   | Metadata    | `subsystem`, `phase`, `transport`, `operation`  |
/// | Teal   | Success     | `status` 2xx                                    |
/// | Amber  | Warning     | `status` 3xx–4xx, `kind` on WARN/ERROR          |
/// | Red    | Error       | `error`, `status` 5xx                           |
///
/// # TEMPLATE: Adding field colors for your service
///
/// If your service has additional domain-specific fields with semantic meaning,
/// add them here. Example for a Gotify server:
/// ```rust,ignore
/// "app_id" | "app_token" => ansi256(aurora::ACCENT_PRIMARY, value),
/// "priority" if value == "10" => ansi256(aurora::ERROR, value),
/// "priority" if value >= "7" => ansi256(aurora::WARN, value),
/// ```
fn style_value(key: &str, value: &str, level: tracing::Level) -> String {
    match key {
        // Pink: service name (identity)
        "service" => ansi256(aurora::SERVICE_NAME, value),

        // Blue: primary action/route/resource identifiers
        "tool" | "prompt" | "resource_uri" | "upstream" | "route" | "action" | "addr"
        | "instance" | "target" | "capability" => ansi256(aurora::ACCENT_PRIMARY, value),

        // Grey: secondary metadata
        "subsystem" | "phase" | "transport" | "operation" => ansi256(aurora::TEXT_MUTED, value),

        // HTTP status: semantic color by range
        "status" => {
            if let Ok(n) = value.parse::<u16>() {
                let color = if n < 300 {
                    aurora::SUCCESS
                } else if n < 500 {
                    aurora::WARN
                } else {
                    aurora::ERROR
                };
                ansi256(color, value)
            } else {
                value.to_string()
            }
        }

        // Red: error messages
        "error" => ansi256(aurora::ERROR, value),

        // Amber: `kind` field on warning/error events
        "kind" if matches!(level, tracing::Level::WARN | tracing::Level::ERROR) => {
            ansi256(aurora::WARN, value)
        }

        // Everything else: no color (plain)
        _ => value.to_string(),
    }
}

// ── Level rendering ───────────────────────────────────────────────────────────

/// Write the log level to the output writer, 5 chars wide.
///
/// Level formatting:
/// - `ERROR` → aurora::ERROR (muted red), bold, no leading space
/// - ` WARN` → aurora::WARN (amber), bold, 1 leading space for alignment
/// - ` INFO` → plain, 1 leading space
/// - `DEBUG` → dim
/// - `TRACE` → dim
///
/// Two trailing spaces separate the level from the message.
fn write_level(writer: &mut Writer<'_>, level: tracing::Level, ansi: bool) -> stdfmt::Result {
    let s = if ansi {
        match level {
            tracing::Level::ERROR => ansi256_bold(aurora::ERROR, "ERROR"),
            tracing::Level::WARN => ansi256_bold(aurora::WARN, " WARN"),
            tracing::Level::INFO => " INFO".to_string(),
            tracing::Level::DEBUG => ansi_dim("DEBUG"),
            tracing::Level::TRACE => ansi_dim("TRACE"),
        }
    } else {
        match level {
            tracing::Level::ERROR => "ERROR".to_string(),
            tracing::Level::WARN => " WARN".to_string(),
            tracing::Level::INFO => " INFO".to_string(),
            tracing::Level::DEBUG => "DEBUG".to_string(),
            tracing::Level::TRACE => "TRACE".to_string(),
        }
    };
    write!(writer, "{s}  ")
}

// ── AuroraFormatter ───────────────────────────────────────────────────────────

/// The Aurora console log formatter.
///
/// Implements tracing_subscriber's [`FormatEvent`] trait so it can be used
/// as a drop-in replacement for the default formatter:
///
/// ```rust,ignore
/// use tracing_subscriber::fmt;
/// use crate::logging::formatter::AuroraFormatter;
///
/// fmt::layer()
///     .with_ansi(should_colorize())
///     .with_writer(std::io::stderr)
///     .event_format(AuroraFormatter)
/// ```
///
/// # Output anatomy
///
/// ```text
/// 14:32:01  INFO  starting  bind=0.0.0.0:3000  auth=bearer  service=example-mcp
/// ────────  ────  ─────────  ─────────────────────────────────────────────────
///   dim      level  message    structured fields (priority order, then alphabetical)
/// ```
///
/// # Thread safety
///
/// `AuroraFormatter` is `Clone + Copy` with no mutable state. It is safe to
/// share across threads without any synchronization.
#[derive(Clone, Copy)]
pub struct AuroraFormatter;

impl<S, N> FormatEvent<S, N> for AuroraFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> stdfmt::Result {
        let ansi = writer.has_ansi_escapes();

        // ── 1. Collect all fields ─────────────────────────────────────────────
        let mut fields = EventFieldCollector::default();
        event.record(&mut fields);

        let level = *event.metadata().level();
        let message = fields
            .take("message")
            .map(|m| sanitize_field_value(&m).into_owned())
            .unwrap_or_default();

        // ── 2. Timestamp: HH:MM:SS (local time, dim) ─────────────────────────
        //
        // TEMPLATE: We use local HH:MM:SS rather than UTC ISO 8601 because:
        // - Development logs are easier to read in local time
        // - The file log (JSON) records full UTC timestamps for analysis
        // - HH:MM:SS is compact; ISO 8601 adds 15 chars of noise per line
        let now = chrono::Local::now();
        let ts = now.format("%H:%M:%S").to_string();
        if ansi {
            write!(writer, "{}  ", ansi_dim(&ts))?;
        } else {
            write!(writer, "{ts}  ")?;
        }

        // ── 3. Level ──────────────────────────────────────────────────────────
        write_level(&mut writer, level, ansi)?;

        // ── 4. Message: first token pink+bold, inline key=val tokens get dim ─
        //
        // TEMPLATE: Message token coloring convention:
        //
        // The first word of the message becomes the visual "action verb" of the
        // log line. Making it pink+bold helps operators scan log streams quickly:
        // the eye jumps to the pink word to understand what happened.
        //
        // Inline `key=val` tokens within the message body (e.g. written as part
        // of the message string rather than as structured fields) get dim key+eq
        // treatment so they don't visually conflict with structured fields.
        if ansi && !message.is_empty() {
            for (i, token) in message.split_whitespace().enumerate() {
                if i > 0 {
                    write!(writer, " ")?;
                }
                if i == 0 {
                    // First token: pink + bold (action verb)
                    write!(writer, "{}", ansi256_bold(aurora::SERVICE_NAME, token))?;
                } else if let Some(eq) = token.find('=') {
                    // Inline key=val: dim key, dim equals, plain value
                    write!(
                        writer,
                        "{}{}{}",
                        ansi_dim(&token[..eq]),
                        ansi_dim("="),
                        &token[eq + 1..],
                    )?;
                } else {
                    // Normal word: plain
                    write!(writer, "{token}")?;
                }
            }
        } else {
            write!(writer, "{message}")?;
        }

        // ── 5. Structured fields: priority first, then alphabetical ───────────
        //
        // TEMPLATE: Priority field order
        //
        // High-priority fields appear first, left-to-right, so the most useful
        // information is immediately visible without horizontal scrolling.
        // Alphabetical ordering of remaining fields ensures deterministic output.
        //
        // Add service-specific high-priority fields to this list.
        // Fields not in the list still appear — just after the priority ones.
        let priority = [
            "kind",
            "request_id",
            "tool",
            "prompt",
            "resource_uri",
            "upstream",
            "route",
            "instance",
            "addr",
            "method",
            "status",
            "operation",
            "capability",
            "transport",
            "response_bytes",
            "elapsed_ms",
            "error",
        ];

        // Closure: write one key=val pair with appropriate styling
        let write_kv = |writer: &mut Writer<'_>, key: &str, raw: &str| -> stdfmt::Result {
            let safe = sanitize_field_value(raw);
            let formatted = format_field_value(&safe);
            if ansi {
                write!(
                    writer,
                    "  {}{}{}",
                    ansi_dim(key),
                    ansi_dim("="),
                    style_value(key, &formatted, level),
                )
            } else {
                write!(writer, "  {key}={formatted}")
            }
        };

        // Write priority fields in declared order (skipping missing ones)
        for key in priority {
            if let Some(val) = fields.take(key) {
                if should_skip_field(key, &val) {
                    continue;
                }
                write_kv(&mut writer, key, &val)?;
            }
        }

        // Write remaining fields in alphabetical order (BTreeMap guarantees this)
        let remaining: Vec<_> = fields.fields.iter().map(|(k, v)| (*k, v.clone())).collect();
        for (key, val) in remaining {
            if should_skip_field(key, &val) {
                continue;
            }
            write_kv(&mut writer, key, &val)?;
        }

        writeln!(writer)
    }
}

#[cfg(test)]
#[path = "formatter_tests.rs"]
mod tests;
