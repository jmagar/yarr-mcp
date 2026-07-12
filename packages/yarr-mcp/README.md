# yarr-mcp

Node launcher for the `yarr` MCP and CLI binary.

`yarr-mcp` is the npm package name because the shorter `yarr` package name is
already occupied. The installed command is still `yarr`, and the GitHub
repository remains `jmagar/yarr`.

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
The npm package version and the yarr release tag are expected to match.
Release automation publishes this package from the repository `v*` tag workflow;
the GitHub repository must have an `NPM_TOKEN` secret with publish access.

The package name is `yarr-mcp` because the shorter `yarr` npm name is already
occupied by an unrelated package. The installed command is still `yarr`.

## 30-second path

```bash
export YARR_SERVICES=sonarr
export YARR_SONARR_URL=http://127.0.0.1:8989
export YARR_SONARR_API_KEY=...
npx -y yarr-mcp sonarr status
npx -y yarr-mcp mcp
```

## Overrides

```bash
YARR_BINARY_VERSION=v1.1.0 npm i -g yarr-mcp
YARR_RELEASE_BASE_URL=https://github.com/jmagar/yarr/releases/download npm i -g yarr-mcp
YARR_SKIP_DOWNLOAD=1 npm i -g yarr-mcp
```

Supported binary targets are Linux x64 and Windows x64, matching the current
GitHub Release assets.
