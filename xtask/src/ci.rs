//! Required local equivalents of the GitHub CI jobs.

use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::{check_test_siblings, patterns, run_cargo, run_cmd, tool_docs};

const REQUIRED_TOOLS: &[&str] = &["cargo-nextest", "taplo", "cargo-deny", "actionlint", "npm"];

pub(super) fn run() -> Result<()> {
    for tool in REQUIRED_TOOLS {
        require_command(tool)?;
    }

    println!("==> [1/13] cargo fmt --check");
    run_cargo(&["fmt", "--all", "--", "--check"]).context("fmt failed — run `cargo fmt` to fix")?;

    println!("==> [2/13] cargo clippy");
    run_cargo(&["clippy", "--all-targets", "--", "-D", "warnings"]).context("clippy failed")?;

    println!("==> [3/13] cargo doc (deny warnings)");
    run_with_env(
        "cargo",
        &["doc", "--no-deps", "--document-private-items"],
        &[("RUSTDOCFLAGS", "-D warnings")],
    )?;

    println!("==> [4/13] cargo nextest run --profile ci");
    run_cargo(&["nextest", "run", "--profile", "ci"]).context("nextest failed")?;

    println!("==> [5/13] taplo check");
    run_cmd("taplo", &["check"]).context("taplo check failed — run `taplo format` to fix")?;

    println!("==> [6/13] actionlint");
    run_cmd("actionlint", &[]).context("GitHub workflow validation failed")?;

    println!("==> [7/13] cargo xtask patterns");
    patterns::run(patterns::PatternOptions::default())
        .context("PATTERNS.md contract check failed")?;

    println!("==> [8/13] cargo xtask check-test-siblings");
    check_test_siblings().context("test sibling check failed")?;

    println!("==> [9/13] generated schema docs");
    run_cmd("python3", &["scripts/check-schema-docs.py", "--check"])?;

    println!("==> [10/13] plugin and feature contracts");
    run_cmd("bash", &["scripts/validate-plugin-layout.sh"])?;
    run_cmd("bash", &["scripts/test-template-features.sh"])?;

    println!("==> [11/13] npm package tests and checks");
    run_cmd("npm", &["test", "--prefix", "packages/yarr-mcp"])?;
    run_cmd("npm", &["run", "check", "--prefix", "packages/yarr-mcp"])?;
    run_cmd(
        "npm",
        &["pack", "--dry-run", "--json", "./packages/yarr-mcp"],
    )?;

    println!("==> [12/13] generated tool docs");
    tool_docs::run(&["--check".to_owned()])?;

    println!("==> [13/13] cargo deny check --all-features");
    run_cmd("bash", &["scripts/check-security-exceptions.sh"])
        .context("dependency-security exception expired")?;
    run_cargo(&["deny", "--all-features", "check"])
        .context("cargo deny found dependency-policy violations")?;

    println!("==> All CI checks passed!");
    Ok(())
}

fn run_with_env(program: &str, args: &[&str], env: &[(&str, &str)]) -> Result<()> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .envs(env.iter().copied());
    let status = command
        .status()
        .with_context(|| format!("Failed to spawn `{program}`"))?;
    if !status.success() {
        bail!("`{program} {}` exited with status {status}", args.join(" "));
    }
    Ok(())
}

fn require_command(name: &str) -> Result<()> {
    let installed = Command::new(name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if installed {
        Ok(())
    } else {
        bail!("required CI tool `{name}` is not installed; install the global mise toolset")
    }
}

#[cfg(test)]
#[path = "ci_tests.rs"]
mod tests;
