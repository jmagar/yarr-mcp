//! Dual-output logging — console (colored) + file (JSON).
//!
//! # TEMPLATE: Why dual logging?
//!
//! Two simultaneous log destinations serve different audiences:
//!
//! | Destination | Format     | Audience              | Purpose                    |
//! |-------------|------------|-----------------------|----------------------------|
//! | stderr      | Pretty     | Developer / operator  | Human-readable, colorized  |
//! | File        | JSON lines | Log aggregator / AI   | Machine-parseable, indexed |
//!
//! The console output is optimized for human scanning: compact timestamps,
//! semantic colors, noise suppression. The file output preserves all fields
//! for programmatic analysis (grep, jq, log aggregators, AI agents).
//!
//! # Usage in main.rs
//!
//! This module is crate-private; the binary calls [`init`] via the
//! [`crate::init_logging`] re-export. `main.rs` wires it up best-effort — file
//! logging failures degrade to stderr rather than aborting startup:
//!
//! ```rust,ignore
//! use rustarr::{config::resolve_data_dir, init_logging};
//!
//! if serve_mode {
//!     // HTTP server: dual logging (pretty console + JSON file under
//!     // {data_dir}/logs/rustarr.log). Falls back to the stderr-only subscriber
//!     // below if the data dir / log file is unavailable.
//!     if resolve_data_dir().and_then(|d| init_logging(&d, "rustarr")).is_ok() {
//!         return;
//!     }
//! }
//! // stdio / CLI (or serve fallback): stderr only — a log file or stdout writes
//! // would corrupt the MCP JSON-RPC stream or CLI output.
//! tracing_subscriber::fmt()
//!     .with_env_filter(EnvFilter::new(if serve_mode { "info" } else { "warn" }))
//!     .with_writer(std::io::stderr)
//!     .init();
//! ```
//!
//! # TEMPLATE: Log file location
//!
//! Logs are written to `{data_dir}/logs/{service}.log`.
//! For the rustarr service this resolves to `~/.rustarr/logs/rustarr.log`.
//!
//! The file is truncated (not rotated) at **startup** if it exceeds 10MB — see
//! [`truncate_log_if_needed`]. The cap is enforced only once per process, so a
//! long-running server can grow the file past 10MB until the next restart; this
//! keeps the implementation simple and avoids log rotation. For production
//! deployments that need persistent or strictly-bounded logs, configure a log
//! aggregator (e.g. Loki, Datadog, CloudWatch) to ship from stderr instead.

pub mod aurora;
pub mod formatter;

use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Context, Result};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use formatter::AuroraFormatter;

/// Initialise dual logging: pretty console (stderr) + JSON file.
///
/// # Arguments
///
/// - `data_dir` — service data directory (e.g. `~/.rustarr`). Logs go into
///   `{data_dir}/logs/{service_name}.log`.
/// - `service_name` — used as the log file name (e.g. `"rustarr"`).
///
/// # Errors
///
/// Returns an error if the log directory cannot be created or the log file
/// cannot be opened for writing.
///
/// # TEMPLATE: EnvFilter precedence
///
/// Log levels are controlled by `RUST_LOG`. If unset, defaults to `"info"`.
/// Examples:
/// - `RUST_LOG=debug` — show all debug logs
/// - `RUST_LOG=info,rmcp=warn` — info level, suppress rmcp crate noise
/// - `RUST_LOG=rustarr=trace` — trace this crate only
///
/// Both the console and file writers share the same `EnvFilter`, so they
/// always emit the same set of events.
pub fn init(data_dir: &Path, service_name: &str) -> Result<()> {
    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("failed to create log directory: {}", log_dir.display()))?;

    let log_path = log_dir.join(format!("{service_name}.log"));

    // Truncate the log file if it has grown past the 10MB cap.
    // See `truncate_log_if_needed()` for rationale.
    truncate_log_if_needed(&log_path)?;

    // Open the log file for appending (creates it if it doesn't exist).
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("failed to open log file: {}", log_path.display()))?;

    let console_ansi = should_colorize();

    // TEMPLATE: Subscriber stack
    //
    // The stack is built as:
    //   registry()          — the base subscriber that stores span data
    //     .with(env_filter) — shared level filter for ALL layers
    //     .with(console)    — pretty, colored stderr output
    //     .with(file)       — JSON lines file output
    //
    // Both layers share the same filter. To give them independent filters,
    // see `tracing_subscriber::layer::Filtered`.
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            // Console layer: pretty, colored, human-readable
            //
            // TEMPLATE: Console layer configuration
            // - `with_ansi(console_ansi)` — enables ANSI codes only when stderr is a TTY
            //   or FORCE_COLOR is set. The AuroraFormatter reads `writer.has_ansi_escapes()`
            //   to conditionally apply colors.
            // - `with_writer(std::io::stderr)` — logs go to stderr, not stdout.
            //   stdout is reserved for CLI output and MCP JSON streams.
            // - `.event_format(AuroraFormatter)` — our custom formatter (see formatter.rs)
            tracing_subscriber::fmt::layer()
                .with_ansi(console_ansi)
                .with_writer(std::io::stderr)
                .event_format(AuroraFormatter),
        )
        .with(
            // File layer: structured JSON, machine-readable
            //
            // TEMPLATE: File layer configuration
            // - `.json()` — emit one JSON object per log line (NDJSON format)
            // - `.with_ansi(false)` — never emit ANSI codes to the file
            // - `.with_writer(log_file)` — write to the log file we opened above
            //
            // JSON format rustarr:
            // {"timestamp":"2026-05-13T14:32:01.123Z","level":"INFO","fields":{"message":"starting","bind":"0.0.0.0:3000"}}
            tracing_subscriber::fmt::layer()
                .json()
                .with_ansi(false)
                .with_writer(log_file),
        )
        .init();

    tracing::debug!(
        log_file = %log_path.display(),
        ansi = console_ansi,
        "logging initialised"
    );

    Ok(())
}

// ── Log file rotation ─────────────────────────────────────────────────────────

/// Maximum log file size in bytes before truncation.
///
/// # TEMPLATE: Why 10MB?
///
/// 10MB is large enough to contain several hours of busy server logs at INFO
/// level, but small enough that disk pressure is never a concern. The file is
/// truncated (not rotated), so disk usage is bounded at exactly one file.
///
/// If you need longer retention, configure log shipping to an external system
/// (Loki, Datadog, etc.) and keep this cap. The file is for local debugging.
const LOG_FILE_MAX_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB

/// Truncate the log file to zero if it exceeds [`LOG_FILE_MAX_BYTES`].
///
/// # TEMPLATE: Truncation vs rotation
///
/// Traditional log rotation creates `service.log.1`, `service.log.2`, etc.
/// We truncate instead because:
/// 1. Simpler — no need to manage multiple files or `logrotate` config
/// 2. Bounded at startup — the file is reset to 0 whenever a process starts and
///    finds it over the cap, so it never accumulates across restarts. (It is
///    *not* re-checked mid-run, so a single long-lived process can still grow
///    the file past the cap until its next restart.)
/// 3. Safe for agents — agents reading the log file always find a single file
///
/// The check runs before the tracing subscriber is installed, so when it
/// truncates it writes the WARN notice straight to stderr (see below) — the
/// operator still sees why the log history starts from the current process.
fn truncate_log_if_needed(path: &std::path::PathBuf) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let size = path
        .metadata()
        .with_context(|| format!("failed to stat log file: {}", path.display()))?
        .len();

    if size >= LOG_FILE_MAX_BYTES {
        std::fs::write(path, b"")
            .with_context(|| format!("failed to truncate log file: {}", path.display()))?;
        // Note: we can't use tracing here (subscriber not yet initialised).
        // Write to stderr directly so the truncation event is never lost.
        eprintln!(
            "WARN  log file exceeded {LOG_FILE_MAX_BYTES} bytes — truncated: {}",
            path.display()
        );
    }

    Ok(())
}

// ── Colorization detection ────────────────────────────────────────────────────

/// Determine whether console log output should include ANSI color codes.
///
/// Priority order (highest to lowest):
///
/// 1. `NO_COLOR` env var set → **no color** (<https://no-color.org> convention)
/// 2. `FORCE_COLOR` env var set → **force color** (useful in Docker/CI)
/// 3. `stderr` is a TTY → **color** (interactive terminal)
/// 4. `stderr` is not a TTY → **no color** (piped/redirected)
///
/// # TEMPLATE: Docker containers
///
/// Docker containers often do NOT have a TTY attached to stderr, which would
/// disable color by rule 4. But `docker compose logs` renders ANSI codes
/// correctly, so operators benefit from colors.
///
/// Set `FORCE_COLOR=1` in your `docker-compose.yml` or Dockerfile:
/// ```yaml
/// environment:
///   FORCE_COLOR: "1"
/// ```
///
/// # TEMPLATE: CI/CD pipelines
///
/// Most CI systems (GitHub Actions, GitLab CI) support ANSI codes.
/// Set `FORCE_COLOR=1` in your CI environment variables to enable color logs.
pub fn should_colorize() -> bool {
    // NO_COLOR takes precedence over everything (https://no-color.org)
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }

    // FORCE_COLOR overrides TTY detection (for Docker, CI, etc.)
    if std::env::var_os("FORCE_COLOR").is_some() {
        return true;
    }

    // Fall back to TTY detection
    std::io::stderr().is_terminal()
}

#[cfg(test)]
#[path = "logging_tests.rs"]
mod tests;
