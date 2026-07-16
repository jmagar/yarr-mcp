# Quickstart

Yarr provides a service-grouped CLI and MCP access to 11 supported media-stack
service kinds.

```bash
export YARR_SERVICES=sonarr
export YARR_SONARR_URL=http://127.0.0.1:8989
export YARR_SONARR_API_KEY=replace-me

npx -y yarr-mcp sonarr status
npx -y yarr-mcp mcp
```

For a permanent install:

```bash
npm install --global yarr-mcp
yarr doctor --json
```

The default MCP mode advertises one `yarr` tool whose `code` argument is an
async JavaScript arrow function. Discover the current callable table instead of
guessing names:

```javascript
async () => codemode.search("sonarr system status")
```

Six services have generated OpenAPI metadata tables. The executor preserves
their declared parameter serialization, request-media, and successful-response
transport contract. Operations that cannot be represented losslessly are not
published; read [Domain concepts](domain.md) and the generated capability matrix
in `docs/TOOLS_ACTIONS_ENDPOINTS.md` for exact counts and omission reasons.

For HTTP MCP, run `yarr serve`, configure OAuth or a read-only static bearer
token, and connect at `/mcp`. `/health`, `/ready`, `/status`, and `/metrics` are
public probe routes.
