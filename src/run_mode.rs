//! Binary launch-mode classification.
//!
//! A pure mapping from process arguments to the mode `main` dispatches on. It
//! lives in the library crate (not `main.rs`) so the arg→mode rules — which are
//! ordering-sensitive and easy to break — can be unit-tested without spawning
//! the binary.

/// The mode the binary runs in, derived from its CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// HTTP MCP server — `yarr`, `yarr serve`, or `yarr serve mcp`.
    Serve,
    /// Stdio MCP transport — `yarr mcp`.
    Stdio,
    /// One-shot CLI command — anything else (`help`, `sonarr list`, …).
    Cli,
}

impl RunMode {
    /// Classify the process arguments (already stripped of `argv[0]`).
    ///
    /// `Serve` and `Stdio` only match their exact arg shapes; any extra or
    /// unexpected argument falls through to `Cli`, where the CLI parser reports
    /// a precise error.
    #[must_use]
    pub fn classify(args: &[String]) -> Self {
        if args.is_empty()
            || matches!(args, [c] if c == "serve")
            || matches!(args, [a, b] if a == "serve" && b == "mcp")
        {
            RunMode::Serve
        } else if matches!(args, [c] if c == "mcp") {
            RunMode::Stdio
        } else {
            RunMode::Cli
        }
    }

    /// Whether this mode runs the long-lived HTTP server.
    #[must_use]
    pub fn is_serve(self) -> bool {
        matches!(self, RunMode::Serve)
    }
}

#[cfg(test)]
#[path = "run_mode_tests.rs"]
mod tests;
