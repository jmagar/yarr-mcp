---
title: "Web UI (removed)"
doc_type: "guide"
status: "removed"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
last_reviewed: "2026-06-15"
---

# Web UI — removed

rustarr no longer ships a web UI. The former Next.js app (`apps/web/`), the Rust
asset-embedding module (`src/web.rs`), and the static-export build recipes were
removed.

rustarr is now **MCP + CLI only**. See the Surfaces table in `README.md` and
`docs/ARCHITECTURE.md` for the current surface layout.
