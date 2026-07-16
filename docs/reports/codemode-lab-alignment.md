# Code Mode: lab vs yarr alignment review

Review of yarr's Code Mode against lab's gateway Code Mode
(`/home/jmagar/workspace/lab/crates/lab/src/dispatch/gateway/code_mode/`).
The core execution, discovery, response-budget, audit, and artifact semantics
are aligned. Remaining differences are product choices, not correctness gaps.

## Aligned (parity or near-parity)
- **One code-running tool** + JS arrow fn (`yarr`/`codemode` ↔ lab's broker).
- **search/describe discovery** — pure-JS, no host round-trip; same result shape.
- **describe surfaces TS types** on demand (yarr also resolves response types).
- **Per-callable namespacing** — lab `codemode.<upstream>.<tool>`; yarr
  `<service>.<verb>` with the service baked in (no `service` param).
- **Snippets** — `codemode.run`/`snippets`, input binding, one-level recursion guard.
- **Artifacts** — `writeArtifact(path, content, options?)` + per-run dir, identical API.
- **Destructive-delete refusal** mid-script; **timeouts** (30s) and **heap** (64 MiB) match.
- **Console capture**, tagged `{__codemode_error}` errors.
- **Response budgeting** — yarr applies a Code-Mode-aware budget before the
  transport cap, replaces oversized results with a structured truncation marker,
  and trims call/log history with explicit sentinels.
- **Per-call timing** — the call audit records `elapsed_ms`.

## Intentional differences

1. **Execution history.** Lab keeps a bounded per-scope deque; yarr returns an
   audit in each run envelope and relies on normal structured logs for durable
   operator history.
2. **Execution source promotion.** Lab can promote a successful script into a
   tool; yarr uses the explicit snippet lifecycle instead.
3. **Result shaping.** Lab applies a schema projection policy; yarr expects the
   script to project its own result before the response budget is applied.
4. **MCP-UI capture.** Lab surfaces `_meta.ui`; yarr has no MCP-UI surface.

## yarr advantages over lab
- **In-process QuickJS** (rquickjs) — no javy/wasmtime subprocess; fits the single binary.
- **Generated OpenAPI operations** exposed as first-class callables for the 6
  spec-backed services — lab's Code Mode only wraps MCP tools, not full upstream specs.

## Verification pointers

- `src/codemode/truncate.rs` owns Code-Mode-aware response budgeting.
- `src/app/codemode.rs` records per-call timing and applies the budget.
- `src/app/codemode_tests/runtime.rs` covers timing and truncation behavior.
