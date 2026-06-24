# rustarr Quickstart

## 1. Configure one service

```bash
cp .env.example .env
export RUSTARR_MCP_HOST=127.0.0.1
export RUSTARR_MCP_PORT=40070
export RUSTARR_MCP_NO_AUTH=true
export RUSTARR_SERVICES=radarr
export RUSTARR_RADARR_URL=http://127.0.0.1:7878
export RUSTARR_RADARR_API_KEY=change-me
```

## 2. Try the CLI

```bash
cargo run -- help
cargo run -- radarr status
cargo run -- radarr get --path /api/v3/system/status
```

## 3. Start HTTP MCP

```bash
cargo run -- serve
```

Call the tool:

```bash
curl -s http://127.0.0.1:40070/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"yarr","arguments":{"code":"async () => radarr.service_status()"}}}'
```

## 4. Run stdio MCP

```bash
cargo run -- mcp
```

Use stdio mode for local child-process MCP clients. It bypasses HTTP auth because the OS process boundary is the trust boundary.

## 5. Verify

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
```
