use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::guard::GuardedEnv;

pub struct RustarrProcess {
    pub binary: String,
    pub env: BTreeMap<String, String>,
}

pub struct Server {
    child: Child,
}

impl RustarrProcess {
    pub fn new(binary: String, guarded: &GuardedEnv) -> Self {
        let mut env = guarded.values.clone();
        env.insert("RUSTARR_HOME".into(), super::guard::SHART_HOME.into());
        Self { binary, env }
    }

    pub fn output(&self, args: &[&str]) -> Result<Output> {
        self.output_with_timeout(args, Duration::from_secs(30))
    }

    pub fn output_with_env(
        &self,
        args: &[&str],
        extra_env: &BTreeMap<String, String>,
    ) -> Result<Output> {
        self.output_with_env_timeout(args, extra_env, Duration::from_secs(30))
    }

    pub fn output_with_env_timeout(
        &self,
        args: &[&str],
        extra_env: &BTreeMap<String, String>,
        timeout: Duration,
    ) -> Result<Output> {
        let mut env = self.env.clone();
        env.extend(extra_env.clone());
        output_for_command(&self.binary, args, &env, None, timeout)
    }

    pub fn output_with_timeout(&self, args: &[&str], timeout: Duration) -> Result<Output> {
        output_for_command(&self.binary, args, &self.env, None, timeout)
    }

    pub fn output_with_stdin(
        &self,
        args: &[&str],
        input: &str,
        timeout: Duration,
    ) -> Result<Output> {
        output_for_command(&self.binary, args, &self.env, Some(input), timeout)
    }

    pub fn output_until_timeout(&self, args: &[&str], timeout: Duration) -> Result<Output> {
        let mut child = Command::new(&self.binary)
            .args(args)
            .envs(&self.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to run {} {}", self.binary, args.join(" ")))?;

        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if child.try_wait()?.is_some() {
                return child
                    .wait_with_output()
                    .context("failed to collect command output");
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = child.kill();
        let output = child
            .wait_with_output()
            .context("failed to collect timed-out command output")?;
        Ok(output)
    }

    pub fn json(&self, args: &[&str]) -> Result<serde_json::Value> {
        let output = self.output(args)?;
        if !output.status.success() {
            bail!("{}", String::from_utf8_lossy(&output.stderr));
        }
        serde_json::from_slice(&output.stdout).context("failed to parse Rustarr CLI JSON")
    }

    pub fn start_server(&self, port: u16) -> Result<Server> {
        self.start_server_args(&["serve", "mcp"], "127.0.0.1", port, &BTreeMap::new())
    }

    pub fn start_server_args(
        &self,
        args: &[&str],
        host: &str,
        port: u16,
        extra_env: &BTreeMap<String, String>,
    ) -> Result<Server> {
        let mut env = self.env.clone();
        env.extend(extra_env.clone());
        env.insert("RUSTARR_MCP_HOST".into(), host.into());
        env.insert("RUSTARR_MCP_PORT".into(), port.to_string());
        let child = Command::new(&self.binary)
            .args(args)
            .envs(env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to start Rustarr MCP server")?;
        Ok(Server { child })
    }
}

fn output_for_command(
    binary: &str,
    args: &[&str],
    env: &BTreeMap<String, String>,
    input: Option<&str>,
    timeout: Duration,
) -> Result<Output> {
    let mut child = Command::new(binary)
        .args(args)
        .envs(env)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to run {} {}", binary, args.join(" ")))?;

    if let Some(input) = input {
        let mut stdin = child.stdin.take().context("failed to open command stdin")?;
        stdin.write_all(input.as_bytes())?;
    }

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            return child
                .wait_with_output()
                .context("failed to collect command output");
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let output = child
        .wait_with_output()
        .context("failed to collect timed-out command output")?;
    bail!(
        "{} {} timed out after {}s; stdout={} stderr={}",
        binary,
        args.join(" "),
        timeout.as_secs(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

impl Server {
    pub fn wait_healthy(&mut self, base_url: &str) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if let Ok(response) = ureq::get(format!("{base_url}/health")).call()
                && response.status().as_u16() == 200
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        bail!("Rustarr server did not become healthy at {base_url}");
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
