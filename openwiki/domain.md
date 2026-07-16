# Domain concepts

## Service kinds and capabilities

Yarr supports exactly 11 kinds: Sonarr, Radarr, Prowlarr, Overseerr, SABnzbd,
qBittorrent, Plex, Jellyfin, Tautulli, Bazarr, and Tracearr.

They map to `ArrManager`, `Indexer`, `Requests`, `DownloadClient`,
`MediaServer`, `Stats`, `Subtitles`, `Trace`, or `GenericOnly` capabilities.
`KindDescriptor` in `src/capability.rs` owns API prefixes, auth styles, resource
nouns, and generic path allowlists.

## Actions

Generic actions are `service_status`, `api_get`, `api_post`, `api_put`,
`api_delete`, and `help`. Infrastructure actions are `codemode`, `op`, and the
four snippet lifecycle actions. Curated action groups currently cover download
clients, stats, subtitles, and trace behavior. Run `yarr help` or inspect the
generated `docs/TOOLS_ACTIONS_ENDPOINTS.md` for exact current names.

`help` is public at the action layer, `service_status` and snippet listing are
read-scoped, and generic credentialed calls/Code Mode/writes are write-scoped.
Mounted HTTP transport authentication still applies before an action is seen.

## Code Mode

The default MCP tool accepts one async JavaScript arrow function. Configured
services appear as callable namespaces; `codemode.search()` and
`codemode.describe()` expose the live catalog, `callTool()` is the low-level
dispatcher, and `api.<service>` provides the generic passthrough.

Generated callable rows are table-driven rather than dedicated Rust functions.
The executor preserves the declared parameter serialization, request-media,
and successful-response transport contract. Operations that cannot be
represented losslessly are excluded and listed, with exact reasons, in the
runtime-derived capability matrix in `docs/TOOLS_ACTIONS_ENDPOINTS.md`.

## Destructive dispatch

There is no `confirm` argument. CLI destructive commands run immediately. MCP
direct and Code Mode-nested destructive calls require an elicitation-capable
peer and approval at dispatch. Scope and destructive checks are re-applied to
inner calls and snippet execution; unsupported or declined elicitation fails
closed.

## Persistence

The runtime data root is `YARR_HOME`, `/data` in a container, or `~/.yarr`.
Snippets are atomically stored as JSON records under `codemode/snippets/`.
Artifacts are contained under per-run paths in `codemode/artifacts/` with path
traversal rejection and size/count budgets.
