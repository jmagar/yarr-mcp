//! The [`AuroraFormatter`] and its `FormatEvent` implementation: assembles a
//! log line from timestamp, level, message, and structured fields.

use std::fmt as stdfmt;

use tracing::{Event, Subscriber};
use tracing_subscriber::{
    fmt::{
        format::{FormatEvent, FormatFields, Writer},
        FmtContext,
    },
    registry::LookupSpan,
};

use crate::logging::aurora;

use super::style::{
    ansi256_bold, ansi_dim, format_field_value, sanitize_field_value, should_skip_field,
    style_value, write_level, EventFieldCollector,
};

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
/// 14:32:01  INFO  starting  bind=0.0.0.0:3000  auth=bearer  service=rustarr-mcp
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
