# Scaffold intent examples

Checked-in example payloads for `docs/contracts/scaffold-intent.schema.json`.

| File | Use case |
|---|---|
| `scaffold-intent-upstream-client.json` | Thin upstream API wrapper. Required surfaces are MCP + CLI. |
| `scaffold-intent-application-platform.json` | Application/platform server. Required surfaces are API + CLI + MCP + Web. |

Validate examples with:

```bash
just scaffold-contract-check
```

These examples are intent payloads only. They are not permission to mutate files; the `scaffold-project` skill must turn them into an approval-first plan.
