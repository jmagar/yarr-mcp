//! Dual-output logging — console (colored) + file (JSON).
//!
//! # Why dual logging?
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
//! use yarr::{init_logging, resolve_data_dir};
//!
//! if serve_mode {
//!     // HTTP server: dual logging (pretty console + JSON file under
//!     // {data_dir}/logs/yarr.log). Falls back to the stderr-only subscriber
//!     // below if the data dir / log file is unavailable.
//!     if resolve_data_dir().and_then(|d| init_logging(&d, "yarr")).is_ok() {
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
//! # Log file location
//!
//! Logs are written to `{data_dir}/logs/{service}.log`.
//! For the yarr service this resolves to `~/.yarr/logs/yarr.log`.
//!
//! File writes are queued to a dedicated worker and rotated while the process
//! is running. Four files are retained (the current file plus three backups),
//! each capped at 10 MiB.

pub mod aurora;
pub mod formatter;

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{SyncSender, sync_channel};

use anyhow::{Context, Result};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use formatter::AuroraFormatter;

/// Initialise dual logging: pretty console (stderr) + JSON file.
///
/// # Arguments
///
/// - `data_dir` — service data directory (e.g. `~/.yarr`). Logs go into
///   `{data_dir}/logs/{service_name}.log`.
/// - `service_name` — used as the log file name (e.g. `"yarr"`).
///
/// # Errors
///
/// Returns an error if the log directory cannot be created or the log file
/// cannot be opened for writing.
///
/// # EnvFilter precedence
///
/// Log levels are controlled by `RUST_LOG`. If unset, defaults to `"info"`.
/// Examples:
/// - `RUST_LOG=debug` — show all debug logs
/// - `RUST_LOG=info,rmcp=warn` — info level, suppress rmcp crate noise
/// - `RUST_LOG=yarr=trace` — trace this crate only
///
/// Both the console and file writers share the same `EnvFilter`, so they
/// always emit the same set of events.
pub fn init(data_dir: &Path, service_name: &str) -> Result<()> {
    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("failed to create log directory: {}", log_dir.display()))?;

    let log_path = log_dir.join(format!("{service_name}.log"));

    let log_writer = non_blocking_rotating_writer(log_path.clone())?;

    let console_ansi = should_colorize();

    // Subscriber stack
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
            // Console layer configuration
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
            // File layer configuration
            // - `.json()` — emit one JSON object per log line (NDJSON format)
            // - `.with_ansi(false)` — never emit ANSI codes to the file
            // - `.with_writer(log_file)` — write to the log file we opened above
            //
            // JSON format yarr:
            // {"timestamp":"2026-05-13T14:32:01.123Z","level":"INFO","fields":{"message":"starting","bind":"0.0.0.0:3000"}}
            tracing_subscriber::fmt::layer()
                .json()
                .with_ansi(false)
                .with_writer(move || log_writer.clone()),
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

/// Maximum size of each retained log file.
const LOG_FILE_MAX_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB
const LOG_BACKUPS: usize = 3;
const LOG_QUEUE_CAPACITY: usize = 8_192;

#[derive(Clone)]
struct NonBlockingLogWriter {
    sender: SyncSender<Vec<u8>>,
    buffer: Vec<u8>,
}

impl Write for NonBlockingLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for NonBlockingLogWriter {
    fn drop(&mut self) {
        if !self.buffer.is_empty() {
            // Logging must never hold up request processing. A saturated queue
            // drops one complete event, never a fragment that corrupts NDJSON.
            if let Err(error) = self.sender.try_send(std::mem::take(&mut self.buffer)) {
                let reason = match error {
                    std::sync::mpsc::TrySendError::Full(_) => "queue_full",
                    std::sync::mpsc::TrySendError::Disconnected(_) => "writer_disconnected",
                };
                axum_prometheus::metrics::counter!(
                    "yarr_log_events_dropped_total",
                    "reason" => reason
                )
                .increment(1);
            }
        }
    }
}

struct RotatingFile {
    path: PathBuf,
    file: Option<std::fs::File>,
    bytes: u64,
}

impl RotatingFile {
    fn open(path: PathBuf) -> Result<Self> {
        if path.metadata().map(|metadata| metadata.len()).unwrap_or(0) >= LOG_FILE_MAX_BYTES {
            rotate_files(&path)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("failed to open log file: {}", path.display()))?;
        let bytes = file.metadata()?.len();
        Ok(Self {
            path,
            file: Some(file),
            bytes,
        })
    }

    fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if self.bytes.saturating_add(bytes.len() as u64) > LOG_FILE_MAX_BYTES {
            if let Some(file) = self.file.as_mut() {
                file.flush()?;
            }
            // Windows cannot rename an open file. Drop the active handle before
            // rotating so the same implementation is portable.
            self.file.take();
            rotate_files(&self.path).map_err(std::io::Error::other)?;
            self.file = Some(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.path)?,
            );
            self.bytes = 0;
        }
        // A single malformed giant event cannot defeat the disk bound.
        let bytes = if bytes.len() as u64 > LOG_FILE_MAX_BYTES {
            &bytes[bytes.len() - LOG_FILE_MAX_BYTES as usize..]
        } else {
            bytes
        };
        self.file
            .as_mut()
            .ok_or_else(|| std::io::Error::other("log file is unavailable"))?
            .write_all(bytes)?;
        self.bytes += bytes.len() as u64;
        Ok(())
    }
}

fn backup_path(path: &Path, generation: usize) -> PathBuf {
    PathBuf::from(format!("{}.{}", path.display(), generation))
}

fn rotate_files(path: &Path) -> Result<()> {
    let oldest = backup_path(path, LOG_BACKUPS);
    if oldest.exists() {
        std::fs::remove_file(&oldest)?;
    }
    for generation in (1..LOG_BACKUPS).rev() {
        let source = backup_path(path, generation);
        if source.exists() {
            std::fs::rename(source, backup_path(path, generation + 1))?;
        }
    }
    if path.exists() {
        std::fs::rename(path, backup_path(path, 1))?;
    }
    for generation in 1..=LOG_BACKUPS {
        let backup = backup_path(path, generation);
        if backup
            .metadata()
            .is_ok_and(|metadata| metadata.len() > LOG_FILE_MAX_BYTES)
        {
            std::fs::OpenOptions::new()
                .write(true)
                .open(&backup)?
                .set_len(LOG_FILE_MAX_BYTES)?;
        }
    }
    Ok(())
}

fn non_blocking_rotating_writer(path: PathBuf) -> Result<NonBlockingLogWriter> {
    let mut writer = RotatingFile::open(path)?;
    let (sender, receiver) = sync_channel::<Vec<u8>>(LOG_QUEUE_CAPACITY);
    std::thread::Builder::new()
        .name("yarr-log-writer".into())
        .spawn(move || {
            while let Ok(bytes) = receiver.recv() {
                if let Err(error) = writer.append(&bytes) {
                    eprintln!("WARN  file logging disabled after write failure: {error}");
                    break;
                }
            }
        })
        .context("failed to start log writer thread")?;
    Ok(NonBlockingLogWriter {
        sender,
        buffer: Vec::new(),
    })
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
/// # Docker containers
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
/// # CI/CD pipelines
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
