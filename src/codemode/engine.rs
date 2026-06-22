//! The rquickjs execution harness.
//!
//! Pure with respect to rustarr's domain: it takes the user code, a JS preamble,
//! resource limits, and an opaque [`ToolCaller`] callback. The engine knows
//! nothing about actions, services, or tokio — the caller wires `on_call` to the
//! async dispatcher (typically via a blocking channel round-trip).
//!
//! Execution model: the preamble's `__rustarrRun(entry)` kicks off
//! `Promise.resolve().then(() => entry())`, stores the JSON-stringified result on
//! `globalThis.__rustarrResult`, and sets `globalThis.__rustarrDone`. Because the
//! only async operation (`callTool`) is a *synchronous* blocking native call,
//! draining the microtask queue settles the whole chain — no async JS runtime is
//! required.

use std::time::Instant;

use rquickjs::{CatchResultExt, Context, Function, Runtime};

/// Synchronous bridge from JS `callTool` to the host. Given `(action, params_json)`
/// it returns either a result JSON string (`Ok`) or an error message (`Err`, thrown
/// into JS as an `Error`). The engine treats it as opaque; the real implementation
/// blocks on a channel round-trip to the async dispatcher.
///
/// Owned and `'static` because rquickjs stores the native bridge in the JS context
/// (it cannot borrow the caller's stack); the real caller boxes a closure that
/// captures a channel sender (which is `Send`).
pub type ToolCaller = Box<dyn Fn(&str, &str) -> Result<String, String> + Send>;

/// Synchronous bridge for `writeArtifact(path, content, options_json)`: returns a
/// receipt JSON string (`Ok`) or an error message (`Err`, thrown into JS). Same
/// ownership rationale as [`ToolCaller`]; the real impl blocks on a channel
/// round-trip to the async writer.
pub type ArtifactWriter = Box<dyn Fn(&str, &str, &str) -> Result<String, String> + Send>;

/// Resource limits for one execution.
pub struct EngineLimits {
    pub memory_bytes: usize,
    pub stack_bytes: usize,
    /// Absolute wall-clock instant past which execution is aborted.
    pub deadline: Instant,
}

/// The successful outcome of an execution.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineOutcome {
    /// The script's return value (already JSON-decoded from its stringified form).
    pub result: serde_json::Value,
    /// Captured `console.*` lines, in emission order.
    pub logs: Vec<String>,
}

/// Run `user_code` (an async-arrow-function expression, or any expression that
/// evaluates to a function or value) after evaluating `preamble`. Returns the
/// decoded result + captured logs, or an error string (timeout, JS exception, or
/// a thrown tool error).
pub fn run(
    user_code: &str,
    preamble: &str,
    limits: &EngineLimits,
    on_call: ToolCaller,
    on_write: ArtifactWriter,
) -> Result<EngineOutcome, String> {
    let rt = Runtime::new().map_err(|e| format!("codemode: runtime init failed: {e}"))?;
    rt.set_memory_limit(limits.memory_bytes);
    rt.set_max_stack_size(limits.stack_bytes);
    let deadline = limits.deadline;
    rt.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));

    let ctx = Context::full(&rt).map_err(|e| format!("codemode: context init failed: {e}"))?;

    // Phase 1 (inside ctx.with): register the native bridge, eval the preamble,
    // and kick off the user code. No microtask runs yet — `__rustarrRun` schedules
    // the work and returns immediately.
    ctx.with(|ctx| {
        let emit = Function::new(
            ctx.clone(),
            move |cx: rquickjs::Ctx<'_>,
                  id: String,
                  params_json: String|
                  -> rquickjs::Result<String> {
                match on_call(&id, &params_json) {
                    Ok(result_json) => Ok(result_json),
                    // Throw a proper `Error` object (not a bare string) so user
                    // `catch (e)` blocks see `e.message` and `e instanceof Error`.
                    Err(message) => Err(rquickjs::Exception::throw_message(&cx, &message)),
                }
            },
        )
        .map_err(|e| format!("codemode: failed to register bridge: {e}"))?;
        ctx.globals()
            .set("__rustarrEmitToolCall", emit)
            .map_err(|e| format!("codemode: failed to install bridge: {e}"))?;

        let write = Function::new(
            ctx.clone(),
            move |cx: rquickjs::Ctx<'_>,
                  path: String,
                  content: String,
                  options_json: String|
                  -> rquickjs::Result<String> {
                match on_write(&path, &content, &options_json) {
                    Ok(receipt_json) => Ok(receipt_json),
                    Err(message) => Err(rquickjs::Exception::throw_message(&cx, &message)),
                }
            },
        )
        .map_err(|e| format!("codemode: failed to register artifact bridge: {e}"))?;
        ctx.globals()
            .set("__rustarrEmitWriteArtifact", write)
            .map_err(|e| format!("codemode: failed to install artifact bridge: {e}"))?;

        ctx.eval::<(), _>(preamble)
            .catch(&ctx)
            .map_err(|e| format!("codemode: preamble error: {e}"))?;

        // `user_code` is wrapped, not concatenated into a statement position, so a
        // bare arrow-function expression parses (and a leading newline guards
        // against a `//` line-comment swallowing the closing paren).
        let invoke = format!("__rustarrRun(\n{user_code}\n)");
        ctx.eval::<(), _>(invoke.as_bytes())
            .catch(&ctx)
            .map_err(|e| format!("codemode: script error: {e}"))?;
        Ok::<(), String>(())
    })?;

    // Phase 2 (OUTSIDE ctx.with, else the runtime double-borrows): drain microtasks
    // until the chain settles. Each job may run the synchronous `callTool` bridge,
    // which blocks this thread on the dispatcher round-trip.
    drain_jobs(&rt, deadline)?;

    // Phase 3: read back the result + logs.
    ctx.with(|ctx| {
        let done: bool = ctx
            .globals()
            .get("__rustarrDone")
            .map_err(|e| format!("codemode: missing completion flag: {e}"))?;
        if !done {
            return Err(
                "codemode: script did not settle (did it return a never-resolving promise?)"
                    .to_string(),
            );
        }
        let result_json: String = ctx
            .globals()
            .get("__rustarrResult")
            .map_err(|e| format!("codemode: missing result: {e}"))?;
        let logs: Vec<String> = ctx.globals().get("__rustarrLogs").unwrap_or_default();

        let value: serde_json::Value = serde_json::from_str(&result_json)
            .map_err(|e| format!("codemode: result was not valid JSON: {e}"))?;
        // A thrown/ rejected script surfaces as a tagged object so we can promote
        // it to a host error rather than returning it as a normal value.
        if let Some(message) = value
            .get("__codemode_error")
            .and_then(serde_json::Value::as_str)
        {
            return Err(format!("codemode: script error: {message}"));
        }
        Ok(EngineOutcome {
            result: value,
            logs,
        })
    })
}

/// Drain the QuickJS job queue until empty, enforcing the deadline. Returns a
/// timeout error if the deadline passes mid-drain.
fn drain_jobs(rt: &Runtime, deadline: Instant) -> Result<(), String> {
    while rt.is_job_pending() {
        if Instant::now() >= deadline {
            return Err("codemode: timed out".to_string());
        }
        rt.execute_pending_job()
            .map_err(|e| format!("codemode: job error: {e:?}"))?;
    }
    if Instant::now() >= deadline {
        return Err("codemode: timed out".to_string());
    }
    Ok(())
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
