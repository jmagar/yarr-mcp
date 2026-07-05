# MCP Registry Publishing Guide

This guide explains how to publish Yarr MCP to the
[official MCP registry](https://modelcontextprotocol.io/registry/quickstart)
using the `server.json` manifest at the repo root.

---

## Prerequisites

- You own the domain used in the `name` field (e.g. `tv.tootie` in `tv.tootie/yarr-mcp`)
- Your Docker image is published to a container registry (e.g. `ghcr.io`)
- Your GitHub repo is public

---

## Step 1 — Check server.json

`server.json` in the repo root should describe the current Yarr release:

| Field | Expected value |
|---|---|
| `name` | `tv.tootie/yarr-mcp` |
| `title` | `Yarr MCP` |
| `description` | Media automation MCP server description |
| `repository.url` | `https://github.com/jmagar/yarr-mcp` |
| `packages[0].identifier` | `ghcr.io/jmagar/yarr-mcp:<version>` |
| `remotes[0].url` | `https://yarr.tootie.tv/mcp` |

---

## Step 2 — Install mcp-publisher

```bash
# Linux amd64
curl -fsSL \
  "https://github.com/modelcontextprotocol/registry/releases/latest/download/mcp-publisher_linux_amd64.tar.gz" \
  | tar xz mcp-publisher
chmod +x mcp-publisher
```

For other platforms, check the
[releases page](https://github.com/modelcontextprotocol/registry/releases).

---

## Step 3 — Authenticate

### Option A: DNS-based (domain ownership proof — preferred for named namespaces)

```bash
./mcp-publisher login dns \
  --domain tv.tootie \
  --private-key "$MCP_PRIVATE_KEY"
```

The private key must correspond to a DNS TXT record you publish at
`_mcp.tv.tootie`. See the registry docs for the exact TXT record format.

### Option B: GitHub OAuth

```bash
./mcp-publisher login github
```

This grants you the `github.com/<your-username>/` namespace automatically,
e.g. `github.com/jmagar/yarr-mcp`.

---

## Step 4 — Publish

```bash
./mcp-publisher publish
```

This reads `server.json` from the current directory and submits it to the registry.
On success, the server appears in the registry under `tv.tootie/yarr-mcp`.

---

## Step 5 — Automate via CI (recommended)

The `docker-publish.yml` workflow can publish automatically when you push a
version tag (e.g. `v1.2.3`).

The relevant workflow snippet:

```yaml
- name: Set version in server.json
  run: |
    VERSION="${GITHUB_REF_NAME#v}"
    jq --arg v "$VERSION" \
       --arg img "ghcr.io/jmagar/yarr-mcp:${VERSION}" \
       '.version = $v | .packages[0].identifier = $img | .packages[0].version = $v' \
       server.json > server.tmp && mv server.tmp server.json

- name: Publish to MCP registry
  env:
    MCP_PRIVATE_KEY: ${{ secrets.MCP_PRIVATE_KEY }}
  run: |
    ./mcp-publisher login dns --domain tv.tootie --private-key "$MCP_PRIVATE_KEY"
    ./mcp-publisher publish
```

Add `MCP_PRIVATE_KEY` as a GitHub repository secret.

---

## Version management

`server.json` always reflects the **currently released version**. Do not manually
edit the `version` or `packages[0].identifier` fields — the `release.yml` workflow
updates them automatically when you push a version tag.

To release a new version:

```bash
git tag v1.2.3
git push origin v1.2.3
```

The `release.yml` workflow builds binaries, updates `server.json`, and triggers
`docker-publish.yml` which publishes the new Docker image and re-publishes to
the MCP registry.

---

## Troubleshooting

### "Name not in your namespace"

You must authenticate for the domain or GitHub user that prefixes your server name.
If your `name` is `tv.tootie/yarr-mcp`, you must authenticate with DNS for
`tv.tootie`. If your `name` is `github.com/jmagar/yarr-mcp`, use GitHub OAuth.

### "Invalid schema"

Run the JSON through the schema validator:

```bash
npx @modelcontextprotocol/registry-validator server.json
```

### "Image not found"

The `packages[0].identifier` OCI image must be publicly pullable before you publish.
Push to GHCR first, then publish to the registry.

---

## Registry namespace summary

| Namespace format | Auth method |
|---|---|
| `github.com/<org>/<name>` | GitHub OAuth |
| `tv.tootie/<name>` | DNS TXT record proof |
