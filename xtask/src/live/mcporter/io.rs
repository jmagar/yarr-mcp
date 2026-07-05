use anyhow::{Context, Result, bail};
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::preview;

pub(super) fn mcporter_output(args: &[&str]) -> Result<Output> {
    let capture = ProcessCapture::new("mcporter-call")?;
    let mut child = Command::new("timeout")
        .args(args)
        .stdout(capture.stdout_stdio()?)
        .stderr(capture.stderr_stdio()?)
        .spawn()
        .context("failed to run mcporter")?;

    let deadline = Instant::now() + Duration::from_secs(50);
    while Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            let status = child.wait().context("failed to collect mcporter status")?;
            return capture.into_output(status);
        }
        thread::sleep(Duration::from_millis(50));
    }

    let _ = child.kill();
    let status = child
        .wait()
        .context("failed to collect timed-out mcporter status")?;
    let output = capture.into_output(status)?;
    bail!(
        "mcporter wrapper timed out after 50s; stderr: {}; stdout: {}",
        preview(String::from_utf8_lossy(&output.stderr).trim()),
        preview(String::from_utf8_lossy(&output.stdout).trim())
    );
}

struct ProcessCapture {
    stdout: PathBuf,
    stderr: PathBuf,
}

impl ProcessCapture {
    fn new(label: &str) -> Result<Self> {
        let nonce = format!(
            "{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .context("system clock before unix epoch")?
                .as_nanos()
        );
        let dir = std::env::temp_dir();
        Ok(Self {
            stdout: dir.join(format!("{nonce}.stdout")),
            stderr: dir.join(format!("{nonce}.stderr")),
        })
    }

    fn stdout_stdio(&self) -> Result<Stdio> {
        Ok(Stdio::from(File::create(&self.stdout).with_context(
            || format!("create {}", self.stdout.display()),
        )?))
    }

    fn stderr_stdio(&self) -> Result<Stdio> {
        Ok(Stdio::from(File::create(&self.stderr).with_context(
            || format!("create {}", self.stderr.display()),
        )?))
    }

    fn into_output(self, status: std::process::ExitStatus) -> Result<Output> {
        let stdout = fs::read(&self.stdout).unwrap_or_default();
        let stderr = fs::read(&self.stderr).unwrap_or_default();
        let _ = fs::remove_file(&self.stdout);
        let _ = fs::remove_file(&self.stderr);
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}
