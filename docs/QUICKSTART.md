# yarr Quickstart

## 1. Configure one service

```bash
cp .env.example .env
export YARR_MCP_HOST=127.0.0.1
export YARR_MCP_PORT=40070
export YARR_MCP_NO_AUTH=true
export YARR_SERVICES=radarr
export YARR_RADARR_URL=http://127.0.0.1:7878
export YARR_RADARR_API_KEY=change-me
```

## 2. Try the CLI

```bash
npx -y yarr-mcp help
npx -y yarr-mcp radarr status
npx -y yarr-mcp radarr get --path /api/v3/system/status
```

For a permanent command on `PATH`:

```bash
npm i -g yarr-mcp
yarr --version
```

For local development from source, replace `npx -y yarr-mcp` with `cargo run --`.

## 3. Start HTTP MCP

```bash
yarr serve
```

Call the tool:

```bash
curl -s http://127.0.0.1:40070/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"yarr","arguments":{"code":"async () => radarr.service_status()"}}}'
```

## 4. Run stdio MCP

```bash
npx -y yarr-mcp mcp
```

Use stdio mode for local child-process MCP clients. It bypasses HTTP auth because the OS process boundary is the trust boundary.

## 5. Verify

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
```
