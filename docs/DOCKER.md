---
title: "Docker"
doc_type: "guide"
status: "active"
owner: "yarr"
audience: ["operators", "contributors", "agents"]
scope: "project"
source_of_truth: false
last_reviewed: "2026-07-16"
---

# Docker

The production image is built from `config/Dockerfile`; local and production
Compose contracts are separate.

## Local build

```bash
docker network inspect mcp >/dev/null 2>&1 || docker network create mcp
cp .env.example .env
docker compose config --quiet
docker compose up --build -d --wait yarr-mcp
docker compose logs -f yarr-mcp
```

`docker-compose.yml` builds `yarr:dev` from the checkout. It overrides the
production image reference and is not a production publication path.

## Production image contract

```bash
export YARR_MCP_IMAGE='ghcr.io/jmagar/yarr@sha256:<64-hex-digest>'
docker compose -f docker-compose.prod.yml config --quiet
docker compose -f docker-compose.prod.yml pull yarr-mcp
docker compose -f docker-compose.prod.yml up -d --wait yarr-mcp
```

Production requirements:

- `YARR_MCP_IMAGE` is mandatory and should be an immutable manifest digest.
- `.env` is mandatory and must define a valid service inventory/credentials.
- The `mcp` external network must already exist.
- `/ready`, not `/health`, gates container readiness.
- The data bind mount is `$HOME/.yarr:/data`.
- The root entrypoint adjusts data permissions, then uses `gosu` to exec the
  binary as UID/GID 1000. Compose does not bypass that startup sequence.
- The root filesystem is read-only, capabilities are dropped, and `/tmp` is a
  tmpfs.
- Dockerfile frontend/base manifests and installed Debian package versions are
  pinned; refresh them deliberately and keep `hadolint config/Dockerfile` green.

The Dockerfile command is `serve mcp`; both `serve` and `serve mcp` select the
HTTP mode. The image does not validate every upstream during startup. Use
`yarr doctor --json` for upstream diagnostics and `/ready` for cheap local
configuration readiness.

## Publication safety

The Docker Publish workflow builds a quarantine tag tied to the source SHA,
scans its immutable digest with Trivy, and promotes `main`/`latest` or semver
tags only after a scan with no HIGH/CRITICAL finding succeeds, including
findings without an upstream fix. A failed scan may leave a candidate tag but
cannot move production tags.

Deploy from the promoted digest shown by the workflow or registry, not from a
mutable tag:

```bash
docker buildx imagetools inspect ghcr.io/jmagar/yarr:1.2.3
# Copy the sha256 manifest digest into YARR_MCP_IMAGE.
```

## Probes

```bash
curl --fail http://127.0.0.1:40070/health
curl --fail http://127.0.0.1:40070/ready
curl --fail http://127.0.0.1:40070/status
curl --fail http://127.0.0.1:40070/metrics
```

`/ready` returns 503 when no services are configured. It does not call
upstreams; run `docker compose ... run --rm --no-deps yarr-mcp doctor --json`
before promotion when upstream reachability is required.

## Rollback

Record the exact current image before replacing it:

```bash
docker inspect --format '{{.Config.Image}}' yarr-mcp | tee .yarr-previous-image
```

Rollback recreates the service from that recorded digest:

```bash
export YARR_MCP_IMAGE="$(cat .yarr-previous-image)"
docker compose -f docker-compose.prod.yml up -d --wait --force-recreate yarr-mcp
```

See `docs/runbooks/deployment-rollback.md` for the full failure checklist.
