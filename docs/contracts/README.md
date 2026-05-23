# JSON contracts

Machine-readable contracts for template handoff payloads and generated artifacts.

## Scaffold intent

- Schema: `scaffold-intent.schema.json`
- Rustarrs:
  - `rustarrs/scaffold-intent-upstream-client.json`
  - `rustarrs/scaffold-intent-application-platform.json`
- Spec: `../specs/scaffold-intent-handoff.md`

`rustarr_scaffold_intent` is returned by the MCP-only `scaffold_intent` elicitation action and consumed by the `scaffold-project` skill. The payload is intent only; it is not permission to mutate files.

Validate with:

```bash
just scaffold-contract-check
```
