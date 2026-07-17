//! Operator lifecycle commands for the dedicated shart test stack.
//!
//! These commands intentionally manage only the 11 containers declared by the
//! guarded Yarr test environment. They never start or stop the Unraid array.

use crate::live::{guard, reset};
use anyhow::{Context, Result, bail};
use std::process::Command;

const DOCKER_PREFLIGHT: &str = "if ! docker info >/dev/null 2>&1; then echo 'shart Docker is unavailable; start the Unraid array before managing the test stack' >&2; exit 1; fi";

pub fn run(args: &[String]) -> Result<()> {
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };
    if args.len() != 1 {
        bail!("usage: cargo xtask shart <start|stop|status|seed>");
    }

    match command {
        "start" => start(),
        "stop" => stop(),
        "status" => status(),
        "seed" => seed(),
        "--help" | "-h" | "help" => {
            print_help();
            Ok(())
        }
        unknown => bail!("unknown shart command: {unknown}"),
    }
}

fn start() -> Result<()> {
    let guarded = guard::load(None, false)?;
    let containers = container_names();
    run_ssh(&container_command("start", &containers))?;
    wait_for_services(&guarded)?;
    print_status(&containers)
}

fn stop() -> Result<()> {
    guard::load(None, false)?;
    let containers = container_names();
    run_ssh(&container_command("stop", &containers))?;
    print_status_allow_stopped(&containers)
}

fn status() -> Result<()> {
    guard::load(None, false)?;
    print_status(&container_names())
}

fn seed() -> Result<()> {
    let guarded = guard::load(None, false)?;
    let containers = container_names();
    run_ssh(DOCKER_PREFLIGHT)?;

    // reset_service restores the configured-v1 snapshots, clears stale SQLite
    // sidecars/PID files, and restarts each golden-backed container. Services
    // without a golden are retained and started below.
    for service in &containers {
        if reset::target_for(service).is_some() {
            reset::reset_service(service)?;
        }
    }

    run_ssh(&container_command("start", &containers))?;
    wait_for_services(&guarded)?;
    print_status(&containers)
}

fn container_names() -> Vec<String> {
    guard::required_kinds()
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn container_command(action: &str, containers: &[String]) -> String {
    format!(
        "set -eu; {DOCKER_PREFLIGHT}; docker {action} {}",
        containers.join(" ")
    )
}

fn status_script(containers: &[String], require_running: bool) -> String {
    let required_check = if require_running {
        r#"if [ "$state" != "running" ]; then failed=1; fi"#
    } else {
        ""
    };
    format!(
        r#"set -eu
{DOCKER_PREFLIGHT}
printf '%-16s %-12s %s\n' CONTAINER STATE HEALTH
failed=0
for container in {}; do
  if docker inspect "$container" >/dev/null 2>&1; then
    state=$(docker inspect --format '{{{{.State.Status}}}}' "$container")
    health=$(docker inspect --format '{{{{if .State.Health}}}}{{{{.State.Health.Status}}}}{{{{else}}}}-{{{{end}}}}' "$container")
  else
    state=missing
    health=-
    failed=1
  fi
  printf '%-16s %-12s %s\n' "$container" "$state" "$health"
  {required_check}
done
exit "$failed""#,
        containers.join(" "),
        DOCKER_PREFLIGHT = DOCKER_PREFLIGHT,
    )
}

fn print_status(containers: &[String]) -> Result<()> {
    run_ssh(&status_script(containers, true)).map(|_| ())
}

fn print_status_allow_stopped(containers: &[String]) -> Result<()> {
    run_ssh(&status_script(containers, false)).map(|_| ())
}

fn wait_for_services(guarded: &guard::GuardedEnv) -> Result<()> {
    for service in &guarded.services {
        let url = reset::service_url(&guarded.values, service)
            .with_context(|| format!("missing URL for shart service {service}"))?;
        reset::wait_service_url(&url)
            .with_context(|| format!("wait for shart service {service} at {url}"))?;
    }
    Ok(())
}

fn run_ssh(command: &str) -> Result<String> {
    let output = Command::new("timeout")
        .args(["--kill-after=5s", "180s", "ssh"])
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("ServerAliveInterval=10")
        .arg("-o")
        .arg("ServerAliveCountMax=3")
        .arg("shart")
        .arg(command)
        .output()
        .with_context(|| format!("failed to run ssh shart {command}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !stdout.is_empty() {
        println!("{stdout}");
    }
    if !output.status.success() {
        bail!("shart command failed: {stderr}");
    }
    Ok(stdout)
}

fn print_help() {
    println!("cargo xtask shart <start|stop|status|seed>");
    println!("  start   Start all guarded Yarr test containers and wait for readiness");
    println!("  stop    Stop all guarded Yarr test containers");
    println!("  status  Show container state/health; fail unless every container is running");
    println!("  seed    Restore configured-v1 golden data, start all containers, and wait");
    println!("These commands do not start or stop the shart Unraid array.");
}

#[cfg(test)]
#[path = "shart_tests.rs"]
mod tests;
