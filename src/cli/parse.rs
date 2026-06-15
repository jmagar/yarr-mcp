//! Shared CLI flag parsers.
//!
//! These are pure argument-shape parsers: they turn `&[String]` token slices
//! into typed values and produce clear errors. They contain **no business
//! logic** (the thin-shim rule) — no upstream calls, no name→id resolution, no
//! dry-run planning. Capability parse modules under `src/cli/commands/<cap>.rs`
//! reuse these helpers (notably the selector parsers `--from`/`--to`/`--title`/
//! `--id`) when later beads add curated commands.

use anyhow::{anyhow, Result};

/// Error if any tokens remain — used by argument-less commands.
pub fn reject_args(args: &[String], command: &str) -> Result<()> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(anyhow!("{command} does not accept argument `{}`", args[0]))
    }
}

/// Parse a bare boolean flag (e.g. `--json`, `--no-repair`). Rejects unknown
/// tokens and duplicates.
pub fn parse_bool_flag(args: &[String], command: &str, flag: &str) -> Result<bool> {
    let mut found = false;
    for arg in args {
        if arg == flag {
            if found {
                return Err(anyhow!("{command} received duplicate {flag}"));
            }
            found = true;
        } else {
            return Err(anyhow!("{command} does not accept argument `{arg}`"));
        }
    }
    Ok(found)
}

/// Parse an optional `--flag VALUE` from a slice that may contain only that flag.
pub fn parse_optional_value_flag(
    args: &[String],
    command: &str,
    flag: &str,
) -> Result<Option<String>> {
    match args {
        [] => Ok(None),
        [found_flag, value] if found_flag == flag => {
            if value.starts_with("--") {
                Err(anyhow!("{command} requires a value after {flag}"))
            } else {
                Ok(Some(value.clone()))
            }
        }
        [found_flag] if found_flag == flag => {
            Err(anyhow!("{command} requires a value after {flag}"))
        }
        [found_flag, value, rest @ ..] if found_flag == flag => {
            if value.starts_with("--") {
                Err(anyhow!("{command} requires a value after {flag}"))
            } else if rest.iter().any(|arg| arg == flag) {
                Err(anyhow!("{command} received duplicate {flag}"))
            } else {
                Err(anyhow!("{command} does not accept argument `{}`", rest[0]))
            }
        }
        [unexpected, ..] => Err(anyhow!("{command} does not accept argument `{unexpected}`")),
    }
}

/// Parse a required `--flag VALUE`; returns `None` when the flag is absent so
/// callers can raise a command-specific "requires" error.
pub fn parse_required_value_flag(
    args: &[String],
    command: &str,
    flag: &str,
) -> Result<Option<String>> {
    parse_optional_value_flag(args, command, flag)
}

/// Parsed shape of the generic passthrough flags (`--path`, `--body`, confirm).
///
/// The service is positional under the new grammar (`rustarr <service> get …`),
/// so it is **not** parsed here — the router supplies it.
#[derive(Debug)]
pub struct PassthroughFlags {
    pub path: String,
    pub body: Option<serde_json::Value>,
    pub confirm: bool,
}

/// Parse `--path P [--body JSON] [--confirm]` for the generic passthrough verbs.
///
/// `--yes` is accepted as an alias for `--confirm` (skip-confirm). `--confirm`
/// remains the canonical spelling.
pub fn parse_passthrough_flags(
    args: &[String],
    command: &str,
    require_body: bool,
    allow_confirm: bool,
) -> Result<PassthroughFlags> {
    let mut path = None;
    let mut body = None;
    let mut confirm = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                i += 1;
                path = Some(
                    args.get(i)
                        .cloned()
                        .ok_or_else(|| anyhow!("{command} requires a value after --path"))?,
                );
            }
            "--body" => {
                i += 1;
                let raw = args
                    .get(i)
                    .ok_or_else(|| anyhow!("{command} requires a value after --body"))?;
                body = Some(serde_json::from_str(raw)?);
            }
            "--confirm" | "--yes" if allow_confirm => {
                confirm = true;
            }
            other => return Err(anyhow!("{command} does not accept argument `{other}`")),
        }
        i += 1;
    }
    let path = path.ok_or_else(|| anyhow!("{command} requires --path"))?;
    if require_body && body.is_none() {
        return Err(anyhow!("{command} requires --body"));
    }
    Ok(PassthroughFlags {
        path,
        body,
        confirm,
    })
}

/// Parse `watch` flags: `[--url URL] [--interval N]`.
pub fn parse_watch_flags(args: &[String]) -> Result<(Option<String>, Option<String>)> {
    let mut url = None;
    let mut interval = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let target = match flag {
            "--url" => &mut url,
            "--interval" => &mut interval,
            _ => return Err(anyhow!("watch does not accept argument `{flag}`")),
        };
        if target.is_some() {
            return Err(anyhow!("watch received duplicate {flag}"));
        }
        let Some(value) = args.get(index + 1) else {
            return Err(anyhow!("watch requires a value after {flag}"));
        };
        if value.starts_with("--") {
            return Err(anyhow!("watch requires a value after {flag}"));
        }
        *target = Some(value.clone());
        index += 2;
    }
    Ok((url, interval))
}

// ── shared selector parser (parse-only) ─────────────────────────────────────────

/// Shared selector flags (`--id`, `--title`, `--from`, `--to`). Currently unused
/// by the command modules (each `cli/commands/<cap>.rs` parses the specific flags
/// it needs directly), but kept and unit-tested as the shared selector helper for
/// command modules that want one common implementation.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Selectors {
    /// `--id VALUE` — address a resource by id.
    pub id: Option<String>,
    /// `--title VALUE` — address/search a resource by title.
    pub title: Option<String>,
    /// `--from VALUE` — range/source selector.
    pub from: Option<String>,
    /// `--to VALUE` — range/target selector.
    pub to: Option<String>,
}

/// Parse the common selector flags (`--id`, `--title`, `--from`, `--to`),
/// ignoring (returning errors for) anything else. Each flag may appear once.
pub fn parse_selectors(args: &[String], command: &str) -> Result<Selectors> {
    let mut out = Selectors::default();
    let mut i = 0;
    while i < args.len() {
        let target = match args[i].as_str() {
            "--id" => &mut out.id,
            "--title" => &mut out.title,
            "--from" => &mut out.from,
            "--to" => &mut out.to,
            other => return Err(anyhow!("{command} does not accept argument `{other}`")),
        };
        if target.is_some() {
            return Err(anyhow!("{command} received duplicate {}", args[i]));
        }
        i += 1;
        let value = args
            .get(i)
            .filter(|v| !v.starts_with("--"))
            .cloned()
            .ok_or_else(|| anyhow!("{command} requires a value after the preceding flag"))?;
        *target = Some(value);
        i += 1;
    }
    Ok(out)
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
