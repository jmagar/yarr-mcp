//! Code Mode orchestration (business layer).
//!
//! Bridges the synchronous JS engine ([`crate::codemode`]) to rustarr's async
//! action dispatch. The engine runs on a blocking thread; each `callTool` becomes
//! a [`ToolRequest`] sent over a channel to the async loop here, which dispatches
//! it through the shared [`execute_service_action`] path and sends the result
//! back. Destructive actions are refused (no confirmation channel mid-script), so
//! Code Mode can read and perform non-destructive writes but never deletes.

use std::time::Instant;

use anyhow::Result;
use serde_json::{Map, Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::actions::{RustarrAction, action_is_destructive, execute_service_action};
use crate::app::RustarrService;
use crate::codemode::{
    self, CODEMODE_MAX_CODE_BYTES, CODEMODE_MEMORY_LIMIT, CODEMODE_STACK_LIMIT, CODEMODE_TIMEOUT,
    EngineLimits, EngineOutcome,
};

/// One `callTool` round-trip: the action id + JSON params, plus a one-shot channel
/// the async loop replies on (a JSON result string or an error message).
struct ToolRequest {
    id: String,
    params_json: String,
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
        if code.trim().is_empty() {
            anyhow::bail!("codemode requires a non-empty `code` string");
        }
        if code.len() > CODEMODE_MAX_CODE_BYTES {
            anyhow::bail!(
                "codemode `code` is {} bytes; the limit is {CODEMODE_MAX_CODE_BYTES}",
                code.len()
            );
        }

        let preamble = codemode::build_preamble(&self.configured_service_names());
        let code = code.to_owned();
        let (req_tx, mut req_rx) = mpsc::channel::<ToolRequest>(8);
        let limits = EngineLimits {
            memory_bytes: CODEMODE_MEMORY_LIMIT,
            stack_bytes: CODEMODE_STACK_LIMIT,
            deadline: Instant::now() + CODEMODE_TIMEOUT,
        };

        // The engine runs on a blocking thread; `on_call` blocks it on a channel
        // round-trip to the async loop below (never the reverse, so no deadlock).
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
            codemode::run(&code, &preamble, &limits, on_call)
        });

        // Drive tool calls until the engine finishes (it drops the sender → `None`).
        let mut calls: Vec<Value> = Vec::new();
        while let Some(req) = req_rx.recv().await {
            let outcome = self.codemode_dispatch(&req.id, &req.params_json).await;
            calls.push(json!({
                "action": req.id,
                "ok": outcome.is_ok(),
                "error": outcome.as_ref().err().cloned(),
            }));
            // Receiver gone (script aborted/timed out) → ignore; the engine result
            // carries the real outcome.
            let _ = req.reply.send(outcome);
        }

        let outcome: EngineOutcome = handle
            .await
            .map_err(|e| anyhow::anyhow!("codemode task panicked: {e}"))?
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(json!({
            "result": outcome.result,
            "calls": calls,
            "logs": outcome.logs,
        }))
    }

    /// Dispatch a single in-sandbox `callTool(id, params)` to the shared action
    /// path. Returns the result as a JSON string (the engine bridge speaks JSON
    /// strings) or an error message. Destructive actions are refused.
    async fn codemode_dispatch(&self, id: &str, params_json: &str) -> Result<String, String> {
        if id == "codemode" {
            return Err("codemode cannot invoke codemode".to_string());
        }
        let params: Value = serde_json::from_str(params_json)
            .map_err(|e| format!("invalid params for `{id}`: {e}"))?;
        let mut args: Map<String, Value> = match params {
            Value::Object(map) => map,
            _ => return Err(format!("params for `{id}` must be a JSON object")),
        };
        args.insert("action".to_string(), Value::String(id.to_owned()));

        let action =
            RustarrAction::from_mcp_args(&Value::Object(args)).map_err(|e| e.to_string())?;
        if action_is_destructive(action.name()) {
            return Err(format!(
                "action `{id}` is destructive and cannot run inside codemode (no confirmation \
                 channel); call it directly with confirm=true"
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
}

#[cfg(test)]
#[path = "codemode_tests.rs"]
mod tests;
