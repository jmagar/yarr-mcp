# yarr-mcp

Node launcher for the Rustarr MCP and CLI binary.

```bash
npx -y yarr-mcp mcp
```

Install globally when you want the command on `PATH`:

```bash
npm i -g yarr-mcp
yarr --version
yarr mcp
```

The package downloads the matching GitHub Release binary during `postinstall`.
The npm package version and the Rustarr release tag are expected to match.

The package name is `yarr-mcp` because the shorter `yarr` npm name is already
occupied by an unrelated package. The installed command is still `yarr`.

## Overrides

```bash
YARR_BINARY_VERSION=v0.4.0 npm i -g yarr-mcp
YARR_RELEASE_BASE_URL=https://github.com/jmagar/rustarr-mcp/releases/download npm i -g yarr-mcp
YARR_SKIP_DOWNLOAD=1 npm i -g yarr-mcp
```

Supported binary targets are Linux x64 and Windows x64, matching the current
GitHub Release assets.
