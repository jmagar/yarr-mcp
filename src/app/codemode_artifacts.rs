//! Code Mode artifact persistence, quotas, and retention.

use std::{
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

use serde_json::{Value, json};

use super::{ARTIFACT_WRITE_LOCK, CODEMODE_RUN_SEQ};
use crate::codemode::{
    self, CODEMODE_ARTIFACT_GLOBAL_BYTES, CODEMODE_ARTIFACT_MIN_FREE_BYTES,
    CODEMODE_ARTIFACT_RETENTION, CODEMODE_ARTIFACTS_SUBDIR, CODEMODE_MAX_ARTIFACT_BYTES,
};

pub(super) fn write_codemode_artifact(
    run: Option<&(String, PathBuf)>,
    path: &str,
    content: &str,
    options_json: &str,
) -> Result<String, String> {
    let (_, dir) = run.ok_or_else(|| {
        "writeArtifact is unavailable: no artifacts root is configured for this server".to_owned()
    })?;
    if content.len() > CODEMODE_MAX_ARTIFACT_BYTES {
        return Err(format!(
            "writeArtifact content is {} bytes; the per-artifact limit is {CODEMODE_MAX_ARTIFACT_BYTES}",
            content.len()
        ));
    }
    let relative = codemode::artifact::validate_artifact_path(path)?;
    let full = codemode::artifact::resolve_under_root(dir, &relative)?;
    let options: Value = serde_json::from_str(options_json).unwrap_or(Value::Null);
    let content_type = codemode::artifact::content_type_for(
        &relative,
        options.get("contentType").and_then(Value::as_str),
    );

    let _guard = ARTIFACT_WRITE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = dir
        .parent()
        .ok_or_else(|| "writeArtifact run directory has no artifact root".to_owned())?;
    if directory_bytes(root).saturating_add(content.len() as u64) > CODEMODE_ARTIFACT_GLOBAL_BYTES {
        return Err(format!(
            "writeArtifact global quota exceeded ({CODEMODE_ARTIFACT_GLOBAL_BYTES} retained bytes)"
        ));
    }
    std::fs::create_dir_all(root)
        .map_err(|error| format!("writeArtifact could not create artifact root: {error}"))?;
    let available = fs2::available_space(root)
        .map_err(|error| format!("writeArtifact could not inspect free disk space: {error}"))?;
    if available.saturating_sub(content.len() as u64) < CODEMODE_ARTIFACT_MIN_FREE_BYTES {
        return Err(format!(
            "writeArtifact refused to consume reserved disk space ({CODEMODE_ARTIFACT_MIN_FREE_BYTES} bytes must remain free)"
        ));
    }
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("writeArtifact could not create directory: {error}"))?;
    }
    let sequence = CODEMODE_RUN_SEQ.fetch_add(1, Ordering::Relaxed);
    let temporary = full.with_extension(format!("yarr-{sequence}.tmp"));
    let result = (|| -> Result<(), String> {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("writeArtifact could not create temporary file: {error}"))?;
        file.write_all(content.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("writeArtifact could not persist temporary file: {error}"))?;
        std::fs::rename(&temporary, &full)
            .map_err(|error| format!("writeArtifact could not commit file: {error}"))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result?;
    Ok(json!({
        "path": relative.to_string_lossy(),
        "bytes": content.len(),
        "contentType": content_type,
    })
    .to_string())
}

fn directory_bytes(root: &Path) -> u64 {
    let mut total = 0_u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.metadata() {
                Ok(metadata) if metadata.is_dir() => pending.push(path),
                Ok(metadata) if metadata.is_file() => total = total.saturating_add(metadata.len()),
                _ => {}
            }
        }
    }
    total
}

pub(super) fn prune_artifact_runs(data_dir: &Path) {
    let root = data_dir.join(CODEMODE_ARTIFACTS_SUBDIR);
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        let expired = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > CODEMODE_ARTIFACT_RETENTION);
        if expired && path.is_dir() {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}
