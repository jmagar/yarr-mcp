# apps/web — Claude Code instructions

## What this app is

A Next.js 16 frontend (static export) that serves the rmcp-template demo UI. It connects to the `example` MCP server's REST API and provides interactive tooling, an API explorer, and a dashboard.

Use `apps/web` only for application/platform servers that intentionally expose API + CLI + MCP + Web (for example `axon`, `lab`, and `syslog`). Upstream-client MCP servers such as `unrust`, `rustifi`, `rustify`, `rustscale`, and `apprise` should keep MCP + CLI parity and omit REST/Web unless they own additional workflows, state, dashboards, or non-MCP consumers.

The UI is built entirely on the **Aurora design system** — a shadcn-compatible component registry at `https://aurora.tootie.tv`. Aurora components are dark-first, operator-grade, and designed for AI/agent UIs.

---

## Aurora registry — how to add components

`components.json` is already configured with the Aurora registry namespace:

```json
{
  "registries": {
    "@aurora": "https://aurora.tootie.tv/r/{name}.json"
  }
}
```

**Install tokens first (one time only — already done):**
```bash
pnpm dlx shadcn@latest add https://aurora.tootie.tv/r/aurora-tokens.json
```

**Install any Aurora component:**
```bash
pnpm dlx shadcn@latest add @aurora/aurora-button
pnpm dlx shadcn@latest add @aurora/aurora-dialog
pnpm dlx shadcn@latest add @aurora/aurora-data-table
# blocks use the same pattern
pnpm dlx shadcn@latest add @aurora/aurora-prompt-input
pnpm dlx shadcn@latest add @aurora/aurora-artifact
```

Components land in `components/ui/` and are immediately usable via their named exports.

---

## Token system

All styling uses Aurora CSS custom properties defined in `components/aurora.css` (imported in `app/globals.css`).

**Never hardcode hex values.** Always reference tokens:

```tsx
// ✓ correct — token reference
style={{ color: "var(--aurora-text-primary)" }}
className="text-[var(--aurora-text-muted)] bg-[var(--aurora-panel-medium)]"

// ✗ wrong — hardcoded
style={{ color: "#e6f4fb" }}
```

**Key token groups:**

| Group | Example vars |
|-------|-------------|
| Surfaces | `--aurora-page-bg`, `--aurora-nav-bg`, `--aurora-panel-medium`, `--aurora-panel-strong`, `--aurora-control-surface` |
| Borders | `--aurora-border-default`, `--aurora-border-strong` |
| Text | `--aurora-text-primary`, `--aurora-text-muted` |
| Accent cyan | `--aurora-accent-primary` (#29b6f6), `--aurora-accent-strong`, `--aurora-accent-deep` |
| Accent rose | `--aurora-accent-pink`, `--aurora-accent-pink-strong` |
| Accent violet | `--aurora-accent-violet` (AI/automation identity) |
| Status | `--aurora-success`, `--aurora-warn`, `--aurora-error`, `--aurora-info` |
| Radii | `--aurora-radius-1` (14px), `--aurora-radius-2` (18px), `--aurora-radius-3` (22px) |
| Typography | `--aurora-font-display` (Manrope), `--aurora-font-sans` (Inter), `--aurora-font-mono` (JetBrains Mono) |

Tokens are also exported to Tailwind via `@theme inline` in `globals.css`, so `bg-[var(--aurora-panel-medium)]` and similar utilities work as expected.

---

## Component conventions

### Class merging

Always use `cn()` from `@/lib/utils` — never concatenate className strings manually:

```tsx
import { cn } from "@/lib/utils"

<div className={cn("base-class", isActive && "active-class", className)} />
```

### Variants with CVA

Aurora components use `class-variance-authority` for variants. When building a new component, follow the same pattern:

```tsx
import { cva, type VariantProps } from "class-variance-authority"

const myVariants = cva("base classes", {
  variants: {
    variant: { aurora: "...", neutral: "...", ghost: "..." },
    size: { sm: "...", default: "...", lg: "..." },
  },
  defaultVariants: { variant: "aurora", size: "default" },
})
```

### Radix UI primitives

Radix packages are installed (`@radix-ui/react-dialog`, `@radix-ui/react-select`, `@radix-ui/react-tabs`, etc.). The Aurora registry components already wrap these — prefer the Aurora registry component over building a new Radix wrapper from scratch.

---

## Aurora component catalog

### UI primitives (`aurora-{name}`)

These are foundational building blocks. All depend on `aurora-tokens`.

**Controls:** `aurora-button`, `aurora-button-group`, `aurora-badge`, `aurora-switch`, `aurora-toggle`, `aurora-toggle-group`, `aurora-avatar`, `aurora-progress`, `aurora-spinner`, `aurora-separator`, `aurora-toolbar`, `aurora-kbd`

**Form:** `aurora-input`, `aurora-field`, `aurora-label`, `aurora-input-group`, `aurora-input-otp`, `aurora-native-select`, `aurora-select`, `aurora-combobox`, `aurora-checkbox`, `aurora-radio-group`, `aurora-slider`, `aurora-number-input`, `aurora-textarea`, `aurora-calendar`, `aurora-date-picker`, `aurora-tabs`

**Data display:** `aurora-card`, `aurora-stat-card`, `aurora-table`, `aurora-data-table`, `aurora-filter-bar`, `aurora-timeline`, `aurora-description-list`, `aurora-search-results`, `aurora-listbox`, `aurora-item`

**Feedback:** `aurora-callout`, `aurora-banner`, `aurora-toast`, `aurora-tooltip`, `aurora-empty-state`, `aurora-skeleton`, `aurora-status-indicator`, `aurora-alert-dialog`, `aurora-hover-card`

**Navigation:** `aurora-breadcrumb`, `aurora-pagination`, `aurora-navigation-menu`, `aurora-menubar`

**Overlays:** `aurora-dialog`, `aurora-accordion`, `aurora-collapsible`, `aurora-dropdown-menu`, `aurora-context-menu`, `aurora-popover`, `aurora-sheet`, `aurora-scroll-area`, `aurora-aspect-ratio`, `aurora-carousel`, `aurora-chart`

### Blocks (domain-organized)

Complex multi-component blocks. Install via same `pnpm dlx shadcn@latest add @aurora/aurora-{name}` pattern.

**AI (48 blocks):** `aurora-prompt-input`, `aurora-message`, `aurora-conversation`, `aurora-artifact`, `aurora-thinking`, `aurora-tool-calls`, `aurora-reasoning`, `aurora-agent`, `aurora-checkpoint`, `aurora-commit`, `aurora-context`, `aurora-panel`, `aurora-plan`, `aurora-task`, `aurora-queue`, `aurora-stack-trace`, `aurora-test-results`, `aurora-model-selector`, `aurora-persona`, `aurora-suggestion`, `aurora-sandbox`, `aurora-audio-player`, `aurora-speech-input`, `aurora-transcription`, `aurora-voice-selector`, `aurora-inline-citation`, `aurora-sources`, `aurora-code-block` (AI variant)

**Workspace:** `aurora-sidebar`, `aurora-command-palette`, `aurora-marketplace`, `aurora-share-dialog`, `aurora-web-preview`, `aurora-code-block`

**Files:** `aurora-attachment`, `aurora-file-picker`, `aurora-file-tree`, `aurora-code-editor`

**Auth:** `aurora-login`, `aurora-oauth`, `aurora-permission-prompt`, `aurora-permissions-dropdown`

**Feedback:** `aurora-error-page`

**Navigation:** `aurora-terminal`

---

## Current app structure

```
apps/web/
├── app/
│   ├── layout.tsx          # Root layout — Aurora nav, dark mode forced
│   ├── page.tsx            # Dashboard
│   ├── tools/page.tsx      # Interactive tool runner
│   └── api/page.tsx        # API explorer
├── components/
│   ├── aurora.css          # Aurora token definitions (auto-imported)
│   └── ui/                 # Aurora components land here when installed
│       ├── button.tsx      # Already has Aurora variants
│       └── ...             # Others installed via pnpm dlx shadcn add
├── lib/
│   ├── utils.ts            # cn() + devWarn()
│   ├── template.ts         # Template knobs: branding, endpoints, action metadata
│   └── api.ts              # Typed REST client for example server
├── components.json         # shadcn config — @aurora registry wired in
├── next.config.ts          # Static export (output: "export")
└── globals.css             # @import tailwindcss + aurora.css + @theme
```

---

## Build commands

```bash
pnpm dev             # Dev server (http://localhost:3001 or next available)
pnpm build           # Static export to out/
pnpm lint            # Biome lint
pnpm typecheck       # TypeScript type check
pnpm validate        # Biome check + typecheck + static build

# Install Aurora components
pnpm dlx shadcn@latest add @aurora/aurora-dialog
pnpm dlx shadcn@latest add @aurora/aurora-data-table
```

---

## Important constraints

- **Static export only** — `next.config.ts` sets `output: "export"`. No server actions, no API routes, no `useSearchParams` without a Suspense boundary.
- **Dark mode is forced** — `app/layout.tsx` sets `<html className="dark">`. Do not add a theme toggle unless Aurora tokens are present for both modes (they are — light mode vars exist in `aurora.css`).
- **No raw HTML elements for UI** — use Aurora components. Never write a raw `<button>`, `<input>`, `<select>`, or `<textarea>` outside of an Aurora component file. Use `aurora-button`, `aurora-input`, etc.
- **No hardcoded colors** — all colors via `var(--aurora-*)` tokens.
- **cn() for all className construction** — never string concatenation.
- **API calls go through `lib/api.ts`** — do not fetch the example server directly from page components; add methods to `api.ts` and call those.

---

## Common patterns

### Replacing inline-styled UI with Aurora components

When you see inline `style={{ color: "var(--aurora-...)" }}` on a div that is functioning as a button, card, or input, replace it with the Aurora component:

```tsx
// Before (inline styled div acting as a button)
<div style={{ cursor: "pointer", color: "var(--aurora-accent-primary)", ... }}>
  Click me
</div>

// After — install aurora-button first
import { Button } from "@/components/ui/button"
<Button variant="aurora">Click me</Button>
```

### Adding a new page

1. Create `app/{route}/page.tsx`
2. Use Aurora layout primitives: `aurora-card` for content sections, `aurora-breadcrumb` for navigation, `aurora-stat-card` for metrics
3. Keep service/action metadata in `lib/template.ts`
4. Keep data fetching in `lib/api.ts`, not in the component

### Status/health display

Use `aurora-status-indicator` for live/dead service status and `aurora-stat-card` for numeric metrics. Both accept Aurora token colors for their state variants.
