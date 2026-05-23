---
title: "Web UI"
doc_type: "guide"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "docs/PATTERNS.md"
last_reviewed: "2026-05-22"
---

# Web UI

The optional web UI lives under `apps/web/` and is built as a static Next.js export embedded into the Rust binary at compile time using `include_dir!`. No separate file-serving process.

## Build flow

```
apps/web/           ← Next.js app source
  next.config.ts    ← output: "export" (static HTML/CSS/JS)
  out/              ← compiled static output (gitignored, built in CI)

src/web.rs          ← Rust: embeds out/ into binary with include_dir!
```

## Commands

```bash
just build-web       # build apps/web/out
just web-watch       # rebuild on changes
just build-full      # build web then release binary (CI)
pnpm -C apps/web check
pnpm -C apps/web typecheck
pnpm -C apps/web test
pnpm -C apps/web build
```

## Embedding in Rust

```rust
use include_dir::{Dir, include_dir};

// Compiled at build time — zero runtime file I/O
static WEB_ASSETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/apps/web/out");

pub fn web_assets_available() -> bool {
    WEB_ASSETS.get_file("index.html").is_some()
}

pub async fn serve_web_assets(request: Request<Body>) -> Response {
    let path = normalize_asset_path(request.uri().path());

    // Try exact path, then with .html, then route index.html.
    let candidates = asset_candidates(path);

    for candidate in candidates {
        if let Some(file) = WEB_ASSETS.get_file(candidate.as_str()) {
            let content_type = guess_mime(candidate.as_str());
            let cache_control = cache_control_for(candidate.as_str());
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, content_type),
                 (header::CACHE_CONTROL, cache_control)],
                file.contents().to_vec(),
            ).into_response();
        }
    }

    // 404 → SPA fallback (client-side routing handles the rest)
    // ...
}
```

## Build orchestration

There is no `build.rs` web build step. Cargo embeds whatever is present in
`apps/web/out/` at compile time.

Local builds:

```bash
just build-web   # scripts/build-web.sh: frozen pnpm install if needed, then pnpm build
just build-full  # build web assets, then cargo build --release
```

Docker builds use the `web` stage in `config/Dockerfile`, run
`pnpm install --frozen-lockfile`, build the static export, and copy `apps/web/out`
into the Rust builder stage before compiling the binary.

`just web-watch` performs one initial `scripts/build-web.sh` run, then rebuilds
the static export when files under `apps/web/` change.

## Feature gate

The web feature is optional:

```toml
# Cargo.toml
[features]
default = ["web"]
web = ["dep:include_dir"]

[dependencies]
include_dir = { version = "0.7", optional = true }
```

## Runtime configuration

`apps/web/lib/template.ts` defines the service display name, endpoints, and optional API base URL. `NEXT_PUBLIC_RUSTARR_API_BASE_URL` should be empty by default so the UI uses same-origin API calls when served by the Rust binary.

Use `apps/web/.env.rustarr` for local web development overrides only.

## Static export configuration

```typescript
// apps/web/next.config.ts
const config = {
  output: "export",
  trailingSlash: true,
  images: { unoptimized: true },
  basePath: "",
};
```

## API surfaces

The UI calls:
- `/health`
- `/status`
- `/v1/rustarr`
- `/mcp` for MCP clients rather than browser UI calls

## Aurora design system

The web UI uses the Aurora design system — shadcn-compatible components for operator-grade AI products.

Registry: `https://aurora.tootie.tv` / GitHub: `https://github.com/jmagar/aurora-design-system`

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "rsc": true,
  "tsx": true,
  "tailwind": {
    "css": "app/globals.css",
    "baseColor": "neutral",
    "cssVariables": true
  },
  "registries": {
    "@aurora": "https://aurora.tootie.tv/r/{name}.json"
  }
}
```

Install Aurora:

```bash
cd apps/web
pnpm dlx shadcn@latest add https://aurora.tootie.tv/r/aurora-tokens.json
```

## Static export

`apps/web/out/.gitkeep` is tracked so Docker COPY paths exist, but generated files under `apps/web/out/*` are ignored. Build assets locally before embedding them in release builds.

See `docs/PATTERNS.md` §A3, §A4, §A5 for embedding, Aurora, and the web feature gate patterns.
