# apps/web

Operator dashboard and interactive tool runner for the MCP server. Built with Next.js 16 (static export), React 19, Tailwind CSS 4, Biome, and the Aurora design system.

## What it is

A static web UI served by the Rust binary alongside the MCP API. This app is for application/platform servers that intentionally expose API + CLI + MCP + Web. Upstream-client MCP servers should keep MCP + CLI parity and omit REST/Web unless they own additional workflows, state, dashboards, or non-MCP consumers.

Three pages:

- **Dashboard** (`/`) — Server health (10s polling), status cards, quick action buttons, activity feed
- **Tool Runner** (`/tools/`) — Select an action, fill in parameters, see the request preview and live JSON response
- **API Explorer** (`/api/`) — Endpoint reference, surface parity table (MCP / REST / CLI), cURL rustarrs for REST-capable actions, and notes for MCP-only actions

## Stack

| Layer | Choice |
|---|---|
| Framework | Next.js 16 (App Router, static export) |
| Runtime | React 19 |
| Language | TypeScript 6 (strict) |
| Styles | Tailwind CSS 4 + Aurora design tokens |
| Components | shadcn/ui scaffolding over Radix UI primitives |
| Icons | shadcn configured for lucide; add lucide-react when introducing icons |
| Fonts | Manrope (display), Inter (sans), JetBrains Mono (mono) |

## Dev commands

```bash
pnpm dev        # dev server at http://localhost:3000
pnpm build      # static export -> out/
pnpm start      # serve the built out/ directory
pnpm lint       # Biome lint
pnpm check      # Biome lint + format check
pnpm typecheck  # TypeScript type check
pnpm test       # Vitest contract tests
pnpm validate   # Biome check + typecheck + tests + static build
```

## How it connects to the backend

All API calls go through `lib/api.ts`. Template-facing service names, endpoints, and action metadata live in `lib/template.ts` so a scaffolded project has one obvious place to update the web UI.

By default, the base URL is empty (relative) — the Rust server serves both the static files and the API from the same origin, so no CORS configuration is needed. For local `pnpm dev` against a separately running backend, copy `.env.rustarr` to `.env.local` and set `NEXT_PUBLIC_RUSTARR_API_BASE_URL` (for rustarr, `http://localhost:3100`).

Every action is dispatched as:

```
POST /v1/rustarr
{ "action": "<action>", "params": { ... } }
```

Helper functions (`integrations`, `help`, `callAction`) wrap the fetch call with typed `ApiResponse<T>` returns. Health and status use `GET /health` and `GET /status`.

## Design system (Aurora)

Dark mode is forced (`<html className="dark">`). All colors are CSS custom properties — never hardcoded hex values.

**Token layers** (defined in `components/aurora.css`):

| Category | Rustarrs |
|---|---|
| Surfaces | `--aurora-page-bg`, `--aurora-panel-medium`, `--aurora-control-surface` |
| Borders | `--aurora-border-default`, `--aurora-border-strong` |
| Text | `--aurora-text-primary`, `--aurora-text-muted` |
| Accents | `--aurora-accent-*` (cyan), `--aurora-accent-pink*` (rose) |
| Status | `--aurora-success`, `--aurora-warn`, `--aurora-error`, `--aurora-info` |
| Radii | `--aurora-radius-1` (14px), `--aurora-radius-2` (18px), `--aurora-radius-3` (22px) |

Aurora tokens are bridged to shadcn's `--primary`, `--card`, `--destructive` aliases in `globals.css`.

**Adding a component:**

```bash
pnpm dlx shadcn@latest add @aurora/aurora-dialog
pnpm dlx shadcn@latest add @aurora/aurora-data-table
```

Components land in `components/ui/`. Use CVA (`class-variance-authority`) for variants and `cn()` from `lib/utils.ts` for className construction.

## File structure

```
apps/web/
├── app/
│   ├── layout.tsx        # Root layout — nav, forced dark mode, font variables
│   ├── page.tsx          # Dashboard (client component, polling)
│   ├── tools/page.tsx    # Tool Runner
│   ├── api/page.tsx      # API Explorer (static)
│   └── globals.css       # Tailwind import + Aurora token bridge + @theme
├── components/
│   ├── aurora.css        # Aurora token definitions (dark + light)
│   └── ui/               # Aurora/shadcn components
├── lib/
│   ├── api.ts            # Typed REST client
│   ├── template.ts       # Template knobs: branding, endpoints, action metadata
│   └── utils.ts          # cn() helper
├── components.json       # shadcn config — @aurora registry
├── next.config.ts        # Static export, output: "export"
└── tsconfig.json         # Path aliases (@/* → ./*), strict mode
```

## Constraints

- **Static export only** — no server actions, API routes, or streaming. `output: "export"` in `next.config.ts`.
- **Client components only** for interactive pages — use `"use client"` and React hooks.
- **All colors via CSS custom properties** — never write a raw hex value in a component.
- **All API calls through `lib/api.ts`** — don't fetch directly in components.
- **`cn()` for classNames** — never string concatenation.

## TEMPLATE

When adapting for a real service:

0. First decide whether the service should keep only MCP + CLI. If it is an upstream-client server like `unrust`, `rustifi`, `rustify`, `rustscale`, or `apprise`, remove/ignore `apps/web` unless there is a specific product need for API/Web.
1. For application/platform servers, update `WEB_APP_CONFIG` in `lib/template.ts` with your service name, display name, endpoints, and env var prefix.
2. Update `ACTIONS` in `lib/template.ts` to match your service's actions, parameters, scopes, and rustarrs.
3. Update `lib/api.ts` helper functions and response interfaces to match your service's action result shapes.
4. Replace `NEXT_PUBLIC_RUSTARR_API_BASE_URL` in `.env.rustarr` and docs with your service-specific public env var name.
5. Run `pnpm validate` before committing the scaffolded web app.
