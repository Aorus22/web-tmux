# Phase 2: REST Client, Sidebar & Session Management - Context

**Gathered:** 2026-09-06
**Status:** Ready for planning
**Mode:** Auto-accepted (smart discuss — unattended authorization; CTRL-03: decisions below will be ratified de-facto by the reference implementation; flagged for post-hoc audit)

<domain>
## Phase Boundary

Live tmux state over REST inside the Phase-1 shell: a `webtmux-backend-client` crate (REST base,
`/api/health` + `/api/tmux/info` + `/api/sessions` GET/POST + optional per-session snapshot),
the 3-level collapsible sidebar tree (session→window→pane) with ~1.5s polling + manual refresh,
the Create Session dialog (real text input, native dir picker, optional initial command), and
the four zero/error pages (Empty, Error-tmux-missing, SelectSession, starting—already built in P1).
Delivers SESS-01, SESS-03, SESS-06, SHELL-03, STATE-02. Out: WebSocket protocol + always-connected
tabs (Phase 3), any terminal rendering beyond what Phase 1 shipped.

</domain>

<decisions>
## Implementation Decisions

### REST client crate
- Lift the Phase-1 `backend-client` stub to a full crate mirroring web-term's `backend-client` (rest.rs pattern): reqwest (rustls) with a `RestClient { base_url: String, http }`; typed structs `TmuxSession {name, windows, attached, created_at, width, height}`, `TmuxTree { SessionNode { session, windows: Vec<WindowNode { window, panes }> } }` matching the Go JSON (be/ handlers) with `#[serde(default)]` and null→empty normalization (api.ts parity: sessions ?? [])
- Endpoints exactly as fe/src/lib/api.ts: GET /api/health, GET /api/tmux/info, POST /api/tmux/binary (Phase 6 uses it — NOT this phase), GET /api/sessions → TmuxTree, POST /api/sessions {name, cwd?, initialCommand?} → {name}; NO snapshot fetch this phase (Phase 4 owns capture replay; tree only)

### Sidebar tree + polling
- Hand-rolled sidebar (web-term nav.rs pattern; gpui-component TitleBar/Tree is out for 1:1): `views/sidebar.rs` — w-60 (240px) column, border-r, bg #1e1e1e, right of nothing-left (title bar persists)
- Header row h-9: "Sessions" label (12px, #808080) + two 3px-icon ghost buttons (RefreshCw lucide + Plus) — sha specifics per fe AppSidebar.tsx
- Rows: session row (ChevronRight 3.5px pivot + name 14px/500 + windows-count badge muted 10px), expand → window rows (index: name, 12px), expand → pane rows (id, current command); expanded state: HashSet<String> per node id; active-session row bg #2d2d2d (FE bg-accent-equivalent token) — clicking a row = SelectSession (workspace body gets a placeholder "Select a session" page — STATE-02)
- Poll: tokio interval 1500ms refetch tree from REST (TanStack parity) + Refresh button triggers same path immediately; new (CLI-created) sessions appear on the next poll — SESS-06
- Sidebar toggle button lands in the Phase-1 title bar bar (SHELL-03): binary snap w-60 ↔ w-0, no slide animation (documented deviation A-accepted)

### Create Session dialog
- gpui-component Dialog (open_dialog) + Input/InputState (first real text entry — proves the gpui-component 0.6 Input path): fields Name (required, validated via tmux session-name rules client-side too), Directory (Input + "Browse…" button → gpui-pre App::prompt_for_paths(PathPromptOptions{directories:true}) built-in picker — stack research verified; no rfd), Initial command (optional Input)
- Submit → POST /api/sessions → on success close dialog + poll-refresh + open the new session as the active workspace; on error render backend error message (e.g. duplicate session) inline under the form (fe parity)
- Dialog visual tokens from ui-themes default-dark (card #1e1e1e, border #3c3c3c, radius 8px, primary #d4d4d4 fill button) — 1:1 with fe CreateSessionDialog.tsx

### Zero/error states
- EmptyState (no sessions): exact fe copy + layout (div.p-8 flex column centered, muted copy) — STATE-02
- ErrorState (tmux missing — tree GET fails with backend-alive): fe ErrorState.tsx structure: heading + explanation + Retry (primary) wired to the poll path
- SelectSessionView: muted centered copy over the bare body when a session tab is open but no session selected (parity fe SelectSessionView.tsx)
- The Phase-1 Starting/Failed pages remain untouched; ErrorState wiring = tree fetch mapped to *error branch*

</code_context placeholder — see code_context below...]

<code_context>
## Existing Code Insights

### Reusable Assets (from Phase 1)
- `webtmux-backend-client` stub crate (lift to full): reqwest/parking_lot already in workspace pins
- AppState entity + free-function views + theme helpers (Phase 1) — sidebar/status render trees collocate in222-theme applicator
- start_supervisor async pump; `BackendStatus` watch channel drives state pages already
- gpui-component 0.6: Dialog/Input/ContextMenu known-good from the inventory; icons.rs lucide constants (add: Plus, RefreshCw, ChevronRight, FolderOpen, Terminal/TerminalSquare)

### Established Patterns
- env-only sidecar spawn (fixed in Phase 1); READY watch → ascendants state
- EXACT-pinned deps + committed Cargo.lock; atomic commits per task

### Integration Points
- RestClient consumes ONLY `/api/*` REST shapes; backend unchanged
- fe/src/lib/api.ts is the REST contract reference (paths + payload normalization)
- fe/src/features/sessions + components/layout/AppSidebar.tsx are the parity anchors
</code_context>

<specifics>
## Specific Ideas

- "SAMA PERSIS" — sidebar anatomy, spacing, tokens, and interaction mirror fe AppSidebar.tsx exactly
- Polling cadence 1.5s matches TanStack refetchInterval; manual Refresh triggers instant refetch and also restarts the interval (fe behavior)
- User is away: decisions auto-accepted; context kept minimal (research + planner files fill detail)
</specifics>

<deferred>
## Specific Ideas / Deferred Ideas

- Session keyboard shortcuts (ctrl-tab/alt-1…) — EXTRA-02, v2
- Rename/kill session flows + context menus — Phase 3 (mutating commands ride the WebSocket there)
- tab persistence on restart — EXTRA-01 v2
</deferred>
