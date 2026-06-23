# Code Mode: lab vs rustarr alignment review

Review of rustarr's Code Mode against lab's gateway Code Mode
(`/home/jmagar/workspace/lab/crates/lab/src/dispatch/gateway/code_mode/`).
**Overall alignment ≈ 75%** on core semantics; the gaps are operational, not
architectural.

## Aligned (parity or near-parity)
- **One code-running tool** + JS arrow fn (`yarr`/`codemode` ↔ lab's broker).
- **search/describe discovery** — pure-JS, no host round-trip; same result shape.
- **describe surfaces TS types** on demand (rustarr also resolves response types).
- **Per-callable namespacing** — lab `codemode.<upstream>.<tool>`; rustarr
  `<service>.<verb>` with the service baked in (no `service` param).
- **Snippets** — `codemode.run`/`snippets`, input binding, one-level recursion guard.
- **Artifacts** — `writeArtifact(path, content, options?)` + per-run dir, identical API.
- **Destructive-delete refusal** mid-script; **timeouts** (30s) and **heap** (64 MiB) match.
- **Console capture**, tagged `{__codemode_error}` errors.

## Gaps (ranked)
1. **Response truncation (HIGH).** Lab caps result by bytes+tokens and returns
   `{truncated, preview, original_size, next_action}`; trims logs oldest-first.
   rustarr's Code Mode envelope has no budget of its own — though the MCP `yarr`
   tool result IS capped at the transport layer by `src/token_limit.rs`
   (`MAX_RESPONSE_BYTES`, applied in `tool_result_from_json`). Closing this fully
   means a Code-Mode-level budget + log trimming in `src/app/codemode.rs`.
2. **Execution history / audit trail (MED).** Lab keeps a bounded per-scope deque;
   rustarr is ephemeral per run.
3. **Execution source store / promotion (MED).** Lab can promote a successful
   script into a tool; rustarr has explicit snippets instead (different workflow).
4. **Result shaping / projection (MED).** Lab applies a schema projection policy
   before truncation; rustarr returns raw results (scripts can shape locally).
5. **Per-call `elapsed_ms` (LOW).** Lab records timing per call; rustarr records
   `{action, ok, error, delivered}` (rustarr adds `delivered`, lab adds timing).
6. **mcp-ui widget capture (LOW).** Lab surfaces `_meta.ui`; rustarr has no MCP-UI.

## rustarr advantages over lab
- **In-process QuickJS** (rquickjs) — no javy/wasmtime subprocess; fits the single binary.
- **Generated OpenAPI operations** exposed as first-class callables for the 6
  spec-backed services — lab's Code Mode only wraps MCP tools, not full upstream specs.

## Punch-list to close the high-value gaps
- Add a Code-Mode response budget + log trimming in `src/app/codemode.rs::run_script`
  (port lab's `truncate.rs` strategy), on top of the existing transport-layer cap.
- (Optional) per-call `elapsed_ms`; bounded execution history.
