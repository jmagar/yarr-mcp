//! Styling primitives for the Aurora console formatter: raw ANSI helpers,
//! field collection, value sanitization, and semantic field coloring.

use std::collections::BTreeMap;
use std::fmt as stdfmt;

use tracing::field::{Field, Visit};
use tracing_subscriber::fmt::format::Writer;

use crate::logging::aurora;

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
pub(super) fn ansi256(n: u8, text: &str) -> String {
    format!("\x1b[38;5;{n}m{text}\x1b[0m")
}

/// Apply ANSI 256 foreground color + bold to `text`.
pub(super) fn ansi256_bold(n: u8, text: &str) -> String {
    format!("\x1b[1;38;5;{n}m{text}\x1b[0m")
}

/// Apply ANSI dim (low intensity) to `text`.
pub(super) fn ansi_dim(text: &str) -> String {
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
pub(super) struct EventFieldCollector {
    pub(super) fields: BTreeMap<&'static str, String>,
}

impl EventFieldCollector {
    fn insert(&mut self, field: &Field, value: String) {
        self.fields.insert(field.name(), value);
    }

    /// Remove and return a field by key (used to extract priority fields first).
    pub(super) fn take(&mut self, key: &str) -> Option<String> {
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
/// # TEMPLATE: Injection attack rustarr
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
/// add them here. Rustarr for a Gotify server:
/// ```rust,ignore
/// "app_id" | "app_token" => ansi256(aurora::ACCENT_PRIMARY, value),
/// "priority" if value == "10" => ansi256(aurora::ERROR, value),
/// "priority" if value >= "7" => ansi256(aurora::WARN, value),
/// ```
pub(super) fn style_value(key: &str, value: &str, level: tracing::Level) -> String {
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
pub(super) fn write_level(
    writer: &mut Writer<'_>,
    level: tracing::Level,
    ansi: bool,
) -> stdfmt::Result {
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
