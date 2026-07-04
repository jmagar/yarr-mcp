//! Shart live-stack reset support.
//!
//! The shart test stack mounts several service `/config` directories directly
//! from `backup/lab/live/golden/<service>` ZFS datasets. Rolling each dataset
//! back to `@configured-v1` gives the live harness a cheap reset point for
//! operations that intentionally rewrite config/auth state or stop a service.

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct ResetTarget {
    pub service: &'static str,
    pub container: &'static str,
    pub dataset: &'static str,
    pub snapshot: &'static str,
}

const RESET_TARGETS: &[ResetTarget] = &[
    target("sonarr"),
    target("radarr"),
    target("prowlarr"),
    target("overseerr"),
    target("plex"),
    target("tautulli"),
    target("sabnzbd"),
    target("qbittorrent"),
    target("jellyfin"),
    ResetTarget {
        service: "jellyfin",
        container: "jellyfin",
        dataset: "jellyfin-cache",
        snapshot: "configured-v1",
    },
];

const fn target(service: &'static str) -> ResetTarget {
    ResetTarget {
        service,
        container: service,
        dataset: service,
        snapshot: "configured-v1",
    }
}

pub fn target_for(service: &str) -> Option<&'static ResetTarget> {
    RESET_TARGETS
        .iter()
        .find(|target| target.service == service)
}

pub fn reset_service(service: &str) -> Result<()> {
    let targets = targets_for(service);
    if targets.is_empty() {
        bail!("no shart ZFS golden reset target for {service}");
    }
    reset_targets(&targets)
}

pub fn reset_targets(targets: &[&ResetTarget]) -> Result<()> {
    if targets.is_empty() {
        return Ok(());
    }
    let command = reset_script(targets);
    run_ssh_shart(&command)?;
    Ok(())
}

fn reset_script(targets: &[&ResetTarget]) -> String {
    let containers = targets
        .iter()
        .map(|target| target.container)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" ");
    let rollbacks = targets
        .iter()
        .map(|target| {
            format!(
                "zfs rollback -r backup/lab/live/golden/{dataset}@{snapshot}\nfind /mnt/backup/lab/live/golden/{dataset} -maxdepth 2 \\( -name '*.db-shm' -o -name '*.db-wal' -o -name '*.pid' \\) -delete\nmkdir -p /mnt/user/lab/live/golden/{dataset}\nrsync -a --delete /mnt/backup/lab/live/golden/{dataset}/ /mnt/user/lab/live/golden/{dataset}/\nfind /mnt/user/lab/live/golden/{dataset} -maxdepth 2 \\( -name '*.db-shm' -o -name '*.db-wal' -o -name '*.pid' \\) -delete",
                dataset = target.dataset,
                snapshot = target.snapshot,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"set -euo pipefail
containers="{containers}"
docker stop $containers >/dev/null 2>&1 || true
{rollbacks}
docker start $containers >/dev/null
"#
    )
}

fn targets_for(service: &str) -> Vec<&'static ResetTarget> {
    RESET_TARGETS
        .iter()
        .filter(|target| target.service == service)
        .collect()
}

pub fn wait_service_url(url: &str) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(45);
    let mut last = String::new();
    while Instant::now() < deadline {
        match ureq::get(url).call() {
            Ok(response) if response.status().as_u16() < 500 => return Ok(()),
            Ok(response) => last = format!("HTTP {}", response.status().as_u16()),
            Err(err) => last = err.to_string(),
        }
        std::thread::sleep(Duration::from_millis(750));
    }
    bail!("service did not become reachable at {url}: {last}");
}

pub fn service_url(values: &BTreeMap<String, String>, service: &str) -> Option<String> {
    let env_name: String = service
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    values.get(&format!("YARR_{env_name}_URL")).cloned()
}

fn run_ssh_shart(command: &str) -> Result<String> {
    let output = Command::new("ssh")
        .arg("shart")
        .arg(command)
        .output()
        .with_context(|| format!("failed to run ssh shart {command}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !output.status.success() {
        bail!("ssh shart reset failed: {stdout}{stderr}");
    }
    Ok(stdout)
}
