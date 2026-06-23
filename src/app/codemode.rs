//! Code Mode orchestration (business layer).
//!
//! Bridges the synchronous JS engine ([`crate::codemode`]) to rustarr's async
//! action dispatch. The engine runs on a blocking thread; each `callTool` becomes
//! a [`ToolRequest`] sent over a channel to the async loop here, which dispatches
//! it through the shared [`execute_service_action`] path and sends the result
//! back. Destructive actions are refused (no confirmation channel mid-script), so
//! Code Mode can read and perform non-destructive writes but never deletes.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::Result;
use serde_json::{Map, Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::actions::{RustarrAction, action_is_destructive, execute_service_action};
use crate::app::RustarrService;
use crate::codemode::{
    self, CODEMODE_ARTIFACTS_SUBDIR, CODEMODE_MAX_ARTIFACT_BYTES, CODEMODE_MAX_ARTIFACTS,
    CODEMODE_MAX_CODE_BYTES, CODEMODE_MEMORY_LIMIT, CODEMODE_STACK_LIMIT, CODEMODE_TIMEOUT,
    EngineLimits, EngineOutcome,
};

/// Process-global monotonic sequence appended to each run-id. Two concurrent runs
/// in the same process could compute the same nanosecond timestamp; the sequence
/// guarantees their run-ids (and thus artifacts dirs) are always distinct.
static CODEMODE_RUN_SEQ: AtomicU64 = AtomicU64::new(0);

/// One `callTool` round-trip: the action id + JSON params, plus a one-shot channel
/// the async loop replies on (a JSON result string or an error message).
struct ToolRequest {
    id: String,
    params_json: String,
    reply: oneshot::Sender<Result<String, String>>,
}

/// One `writeArtifact` round-trip: the relative path + content + options JSON,
/// plus a one-shot channel the async loop replies on (a receipt JSON string or an
/// error message).
struct ArtifactRequest {
    path: String,
    content: String,
    options_json: String,
    reply: oneshot::Sender<Result<String, String>>,
}

impl RustarrService {
    /// Execute a Code Mode script: run `code` (a JS async-arrow expression) in the
    /// sandbox, dispatching its `callTool`/`tools.*` calls through the shared
    /// action path, and return `{ result, calls, logs }`.
    ///
    /// `result` is the script's return value, `calls` is the per-call audit log
    /// (`{action, ok, error}`), and `logs` is captured `console.*` output.
    pub async fn codemode(&self, code: &str) -> Result<Value> {
        self.run_script(code, None, false).await
    }

    /// Shared executor for a Code Mode script. `input_json` binds `globalThis.input`
    /// (for `snippet_run`); `in_snippet` is true when running a saved snippet, which
    /// refuses a further `snippet_run` so snippets can't recurse into snippets
    /// (the only nesting bound needed — max depth is 2).
    async fn run_script(
        &self,
        code: &str,
        input_json: Option<String>,
        in_snippet: bool,
    ) -> Result<Value> {
        if code.trim().is_empty() {
            anyhow::bail!("codemode requires a non-empty `code` string");
        }
        if code.len() > CODEMODE_MAX_CODE_BYTES {
            anyhow::bail!(
                "codemode `code` is {} bytes; the limit is {CODEMODE_MAX_CODE_BYTES}",
                code.len()
            );
        }

        let preamble = codemode::build_preamble(&self.configured_service_kinds());
        let code = code.to_owned();
        let (req_tx, mut req_rx) = mpsc::channel::<ToolRequest>(8);
        let (art_tx, mut art_rx) = mpsc::channel::<ArtifactRequest>(8);
        let limits = EngineLimits {
            memory_bytes: CODEMODE_MEMORY_LIMIT,
            stack_bytes: CODEMODE_STACK_LIMIT,
            deadline: Instant::now() + CODEMODE_TIMEOUT,
        };

        // Per-run artifacts dir, computed host-side (the engine never reads a clock).
        // `None` when no artifacts root is configured → `writeArtifact` errors.
        let run = self.data_dir().map(|root| {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let seq = CODEMODE_RUN_SEQ.fetch_add(1, Ordering::Relaxed);
            let run_id = format!("{nanos}-{}-{seq}", std::process::id());
            let dir = root.join(CODEMODE_ARTIFACTS_SUBDIR).join(&run_id);
            (run_id, dir)
        });

        // The engine runs on a blocking thread; `on_call`/`on_write` block it on a
        // channel round-trip to the async loop below (never the reverse, so no
        // deadlock).
        let handle = tokio::task::spawn_blocking(move || {
            let on_call: codemode::ToolCaller = Box::new(move |id: &str, params_json: &str| {
                let (reply_tx, reply_rx) = oneshot::channel();
                req_tx
                    .blocking_send(ToolRequest {
                        id: id.to_owned(),
                        params_json: params_json.to_owned(),
                        reply: reply_tx,
                    })
                    .map_err(|_| "codemode dispatcher unavailable".to_string())?;
                reply_rx
                    .blocking_recv()
                    .map_err(|_| "codemode dispatch was dropped".to_string())?
            });
            let on_write: codemode::ArtifactWriter =
                Box::new(move |path: &str, content: &str, options_json: &str| {
                    let (reply_tx, reply_rx) = oneshot::channel();
                    art_tx
                        .blocking_send(ArtifactRequest {
                            path: path.to_owned(),
                            content: content.to_owned(),
                            options_json: options_json.to_owned(),
                            reply: reply_tx,
                        })
                        .map_err(|_| "codemode artifact writer unavailable".to_string())?;
                    reply_rx
                        .blocking_recv()
                        .map_err(|_| "codemode artifact write was dropped".to_string())?
                });
            codemode::run(
                &code,
                &preamble,
                &limits,
                on_call,
                on_write,
                input_json.as_deref(),
            )
        });

        // Drive BOTH channels until each is drained to `None`. The engine drops both
        // senders together when it finishes, but buffered messages must still be
        // received — so we keep selecting (with per-branch `done` guards, never
        // breaking on the first `None`) until both are exhausted. Guarding the
        // branches also avoids busy-spinning on a closed receiver.
        let mut calls: Vec<Value> = Vec::new();
        let mut artifacts: Vec<Value> = Vec::new();
        let mut written = 0usize;
        let mut req_done = false;
        let mut art_done = false;
        loop {
            tokio::select! {
                maybe = req_rx.recv(), if !req_done => match maybe {
                    Some(req) => {
                        let started = Instant::now();
                        let outcome = self
                            .codemode_dispatch(&req.id, &req.params_json, in_snippet)
                            .await;
                        let elapsed_ms = started.elapsed().as_millis();
                        let ok = outcome.is_ok();
                        let error = outcome.as_ref().err().cloned();
                        // Record whether the script actually received the result: a
                        // failed send means the engine already abandoned this call
                        // (deadline fired mid-flight), so `ok` describes the action
                        // but the script never saw it — surface that, never hide it.
                        let delivered = req.reply.send(outcome).is_ok();
                        calls.push(json!({
                            "action": req.id, "ok": ok, "error": error,
                            "delivered": delivered, "elapsed_ms": elapsed_ms,
                        }));
                    }
                    None => req_done = true,
                },
                maybe = art_rx.recv(), if !art_done => match maybe {
                    Some(art) => {
                        let outcome = write_codemode_artifact(
                            run.as_ref(), &mut written, &art.path, &art.content, &art.options_json,
                        );
                        let ok = outcome.is_ok();
                        let error = outcome.as_ref().err().cloned();
                        // The file is already written + counted; if the receipt can't
                        // be delivered (receiver dropped), record `delivered:false` so
                        // the response doesn't claim a write the script never saw.
                        let delivered = art.reply.send(outcome).is_ok();
                        artifacts.push(json!({
                            "path": art.path, "ok": ok, "error": error, "delivered": delivered,
                        }));
                    }
                    None => art_done = true,
                },
                else => break,
            }
        }

        let outcome: EngineOutcome = handle
            .await
            .map_err(|e| anyhow::anyhow!("codemode task panicked: {e}"))?
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut response = json!({
            "result": outcome.result,
            "calls": calls,
            "logs": outcome.logs,
            "artifacts": artifacts,
        });
        if let Some((run_id, _)) = run {
            response["artifactsRunId"] = Value::String(run_id);
        }
        // Shape the envelope to a Code-Mode budget BELOW the transport cap: trim
        // logs oldest-first to preserve `result`, marker-ize an oversized `result`
        // only if dropping all logs still doesn't fit. Keeps the envelope parseable
        // instead of letting the blunt transport cap slice it mid-JSON.
        crate::codemode::truncate::fit_response(&mut response);
        Ok(response)
    }

    /// Dispatch a single in-sandbox `callTool(id, params)` to the shared action
    /// path. Returns the result as a JSON string (the engine bridge speaks JSON
    /// strings) or an error message. Destructive actions are refused.
    async fn codemode_dispatch(
        &self,
        id: &str,
        params_json: &str,
        in_snippet: bool,
    ) -> Result<String, String> {
        if id == "codemode" {
            return Err("codemode cannot invoke codemode".to_string());
        }
        if in_snippet && id == "snippet_run" {
            return Err(
                "a snippet cannot run another snippet (codemode.run is one level deep)".to_string(),
            );
        }
        let params: Value = serde_json::from_str(params_json)
            .map_err(|e| format!("invalid params for `{id}`: {e}"))?;
        let mut args: Map<String, Value> = match params {
            Value::Object(map) => map,
            _ => return Err(format!("params for `{id}` must be a JSON object")),
        };
        // Destructive operations are refused mid-script (no confirmation channel
        // inside Code Mode) UNLESS RUSTARR_ALLOW_DESTRUCTIVE is set — the global
        // trusted-test-stack override that the contract harness uses to drive
        // deletes against shart.
        let destructive_ok = crate::config::destructive_allowed();
        // A generated operation that is a DELETE is destructive (shared detection).
        if id == "op" && !destructive_ok {
            let service_name = args.get("service").and_then(Value::as_str).unwrap_or("");
            let op_name = args.get("op").and_then(Value::as_str).unwrap_or("");
            if self.op_is_destructive_delete(service_name, op_name) {
                return Err(format!(
                    "operation `{op_name}` is a DELETE (destructive) and cannot run inside \
                     codemode (no confirmation channel); set RUSTARR_ALLOW_DESTRUCTIVE on a \
                     disposable test stack, or call it directly with confirm=true"
                ));
            }
        }
        args.insert("action".to_string(), Value::String(id.to_owned()));

        let action =
            RustarrAction::from_mcp_args(&Value::Object(args)).map_err(|e| e.to_string())?;
        if action_is_destructive(action.name()) && !destructive_ok {
            return Err(format!(
                "action `{id}` is destructive and cannot run inside codemode (no confirmation \
                 channel); set RUSTARR_ALLOW_DESTRUCTIVE on a disposable test stack, or call it \
                 directly with confirm=true"
            ));
        }
        // Box the recursive call: `codemode` reaches `execute_service_action`,
        // which can reach `codemode` again, so the future cycle must be heap-broken
        // (E0733). In practice self-invocation is already refused above.
        let value = Box::pin(execute_service_action(self, &action))
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&value).map_err(|e| format!("could not serialize `{id}` result: {e}"))
    }

    /// The snippet store root (the data dir), or an error when none is configured.
    fn snippet_store_root(&self) -> Result<PathBuf> {
        self.data_dir().map(Path::to_path_buf).ok_or_else(|| {
            anyhow::anyhow!("snippets are unavailable: no data dir is configured for this server")
        })
    }

    /// `snippet_list` — saved snippet metadata.
    pub async fn snippet_list(&self) -> Result<Value> {
        let dir = self.snippet_store_root()?;
        let snippets = codemode::store::list(&dir).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(json!({ "snippets": snippets }))
    }

    /// `snippet_save` — create or overwrite a named snippet.
    pub async fn snippet_save(
        &self,
        name: &str,
        code: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        if code.trim().is_empty() {
            anyhow::bail!("snippet_save requires a non-empty `code`");
        }
        if code.len() > CODEMODE_MAX_CODE_BYTES {
            anyhow::bail!("snippet `code` exceeds {CODEMODE_MAX_CODE_BYTES} bytes");
        }
        let dir = self.snippet_store_root()?;
        let meta = codemode::store::save(&dir, name, code, description)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(json!({ "saved": meta }))
    }

    /// `snippet_run` — load and execute a saved snippet (one level deep), binding
    /// `globalThis.input`. Reuses the shared executor with `in_snippet = true`, so
    /// the snippet cannot itself run another snippet.
    pub async fn snippet_run(&self, name: &str, input: &Value) -> Result<Value> {
        let dir = self.snippet_store_root()?;
        let source =
            codemode::store::load_source(&dir, name).map_err(|e| anyhow::anyhow!("{e}"))?;
        // Propagate (don't silently bind `null`) if the caller's input can't be
        // serialized — they should learn their input was rejected.
        let input_json = serde_json::to_string(input)
            .map_err(|e| anyhow::anyhow!("snippet input is not serializable as JSON: {e}"))?;
        // Box: snippet_run -> run_script -> dispatch -> execute_service_action can
        // reach snippet_run again, so the future cycle must be heap-broken (E0733).
        Box::pin(self.run_script(&source, Some(input_json), true)).await
    }

    /// `snippet_delete` — remove a saved snippet. Mutating but NOT destructive
    /// (operator-authored source, recoverable), so it runs immediately.
    pub async fn snippet_delete(&self, name: &str) -> Result<Value> {
        let dir = self.snippet_store_root()?;
        let existed = codemode::store::delete(&dir, name).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(json!({ "deleted": existed, "name": name }))
    }
}

/// Write one Code Mode artifact under the per-run dir, returning a receipt JSON
/// string `{path, bytes, contentType}` or an error message. Enforces the per-run
/// count cap and per-artifact byte cap, validates the path fail-closed, and never
/// escapes the run dir. Synchronous (small files; not a hot path).
fn write_codemode_artifact(
    run: Option<&(String, PathBuf)>,
    written: &mut usize,
    path: &str,
    content: &str,
    options_json: &str,
) -> Result<String, String> {
    let (_, dir) = run.ok_or_else(|| {
        "writeArtifact is unavailable: no artifacts root is configured for this server".to_string()
    })?;
    if *written >= CODEMODE_MAX_ARTIFACTS {
        return Err(format!(
            "writeArtifact limit reached ({CODEMODE_MAX_ARTIFACTS} artifacts per run)"
        ));
    }
    if content.len() > CODEMODE_MAX_ARTIFACT_BYTES {
        return Err(format!(
            "writeArtifact content is {} bytes; the per-artifact limit is {CODEMODE_MAX_ARTIFACT_BYTES}",
            content.len()
        ));
    }

    let rel = codemode::artifact::validate_artifact_path(path)?;
    let full = codemode::artifact::resolve_under_root(dir, &rel)?;
    let options: Value = serde_json::from_str(options_json).unwrap_or(Value::Null);
    let content_type = codemode::artifact::content_type_for(
        &rel,
        options.get("contentType").and_then(Value::as_str),
    );

    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("writeArtifact could not create directory: {e}"))?;
    }
    std::fs::write(&full, content.as_bytes()).map_err(|e| format!("writeArtifact failed: {e}"))?;
    *written += 1;

    Ok(json!({
        "path": rel.to_string_lossy(),
        "bytes": content.len(),
        "contentType": content_type,
    })
    .to_string())
}

#[cfg(test)]
#[path = "codemode_tests.rs"]
mod tests;
