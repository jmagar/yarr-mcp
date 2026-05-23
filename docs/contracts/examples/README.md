# Scaffold intent rustarrs

Checked-in rustarr payloads for `docs/contracts/scaffold-intent.schema.json`.

| File | Use case |
|---|---|
| `scaffold-intent-upstream-client.json` | Thin upstream API wrapper. Required surfaces are MCP + CLI. |
| `scaffold-intent-application-platform.json` | Application/platform server. Required surfaces are API + CLI + MCP + Web. |

Validate rustarrs with:

```bash
just scaffold-contract-check
```

These rustarrs are intent payloads only. They are not permission to mutate files; the `scaffold-project` skill must turn them into an approval-first plan.
