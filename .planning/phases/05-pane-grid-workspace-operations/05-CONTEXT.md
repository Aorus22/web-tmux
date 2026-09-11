# Phase 5: Pane Grid & Workspace Operations - Context

**Gathered:** 2026-09-07
**Status:** Ready for planning
**Mode:** Auto-accepted autonomous (user sleeping — D1–D10 from 05-RESEARCH.md locked as decided; flagged for post-hoc audit per the Phase-4 precedent)

<domain>
## Phase Boundary

Replace the Phase-4 stacked terminal placeholder with a faithful tmux-geometry
workspace inside the Phase-3 shell: a pure `pane_geometry` module (verbatim
port of `fe/src/lib/geometry.ts` — `pixelRect` + `closePaneGaps` + divider
adjacency + drag-step math), absolutely positioned panes inside the workspace
div, divider drags sending throttled incremental `pane.resize` steps, zoom as
a server toggle, six layout presets from a window toolbar, full pane/window
context menus, pane + window rename dialogs cloning the Phase-3 DLG1 pattern,
and the title-bar Plus button + chip context menus Phase 3 deliberately left
out. Every mutation rides the existing correlated
`SessionWsHandle::send_command` path — no new crates, no backend changes, no
protocol fork. Delivers PANE-01..PANE-08, DLG-02. Out: theme system +
Settings page + palette + font/icons beyond 9 new lucide constants (Phase 6),
reconnect banners/toasts (Phase 7 — drag/mutation errors surface inline like
Phase 3), backend protocol changes including `pane.join` (logged gap, not
forked), session shortcuts (EXTRA-02, v2), tab persistence (EXTRA-01, v2).

</domain>

<decisions>
## Implementation Decisions

### Geometry port + absolute layout (D1, D2 carry — LOCKED)
- **D1 — Verbatim geometry port into a pure module (LOCKED):** new
  `pane_geometry` module in the `webtmux` crate ports `pixelRect`,
  `closePaneGaps`, `pxToCells`, `resizeDragStep` (and divider adjacency) as
  pure functions over `i64` cell coordinates — snapshot `usize` cells convert
  at the module boundary (`p.left - 1` underflows `usize` in debug). No GPUI
  types inside; headless `cargo test` coverage ships in the same wave.
- **D2 — Absolute positioning, not flex tiling (LOCKED):** panes render as
  absolutely positioned children of the workspace div at their `pixelRect`
  rects (FE `PaneWorkspace.tsx:353-360` parity). Zoomed window renders only
  the zoomed pane full-size with dividers hidden. Zoom button hidden for
  single-pane windows (`canZoom = otherPanes.len() > 0`).

### Divider drags (D3 carry — LOCKED)
- **D3 — Divider drag state machine with 40ms throttle + FLIP (LOCKED):**
  `mouse_down` records `(pane_id, direction, start_px, last_cells=0,
  last_sent)`; `mouse_move` reuses the proven `terminal_view.rs:345-379`
  pattern, computes the incremental step (`px_to_cells(pos - start, cell) -
  last_cells`), drops `step==0`, drops sends inside the 40ms window WITHOUT
  advancing `last_cells`; negative steps flip direction with positive amount
  (`FLIP = {L:R, R:L, U:D, D:U}`). Steps ride correlated `pane.resize` with
  the receiver dropped (no per-step await/task spam); truth follows via
  snapshot deltas. Drag code NEVER sends `terminal.resize` (Phase-4
  layout-key timers own viewport resync) and NEVER sends cumulative
  displacement (tmux applies each resize on top of current size — cumulative
  sends overshoot and stick at limits).

### Zoom + sends (D4, D5 carry — LOCKED)
- **D4 — Zoom is a server toggle; no client zoom state (LOCKED):** header
  button, header double-click, and menu Zoom all send `pane.zoom`; render
  filters `panes.find(zoomed) → [zoomed]`. No client-side zoom boolean —
  external tmux clients invalidate any local copy instantly.
- **D5 — Correlated sends for everything except select + drag steps
  (LOCKED):** all pane/window mutations use `SessionWsHandle::send_command`
  + 10s correlated await with inline error (the Phase-3
  `submit_rename`/`await_rename_result` shape); `pane.select`/
  `window.select` and drag steps are fire-and-forget correlated sends
  (receiver dropped — pending entry cleans itself on reply). Never send
  `hello` (it is a resize, not a handshake). Never optimistic-flip UI —
  server snapshot is the sole truth.

### Dialogs + menus + toolbar + headers (D6, D7 carry — LOCKED)
- **D6 — Rename dialogs clone DLG1 exactly (LOCKED):** one form entity per
  dialog on `AppState`, `has_active_dialog` double-open guard, `InputState`
  prefill + autofocus, Enter submits, `is_submitting` double-submit guard,
  reset-on-dismiss, inline error line. Prefill: pane → `title ||
  current_command`; window → `name`. Gate is trim + non-empty only (no
  session-name validation — tmux titles accept anything). Delivers DLG-02
  for pane + window (session done in Phase 3).
- **D7 — Menus/toolbar/headers copy FE item lists verbatim (LOCKED):** pane
  menu (9 rows incl. separators), window tab menu (6 rows), toolbar (5
  presets + Next), header (path + id + TUI switch + 4 actions). Kill entries
  honor `confirm_kill_pane`/`confirm_kill_window` via new AppState gates
  mirroring `kill_requires_confirm` (struct flags already `default_true` —
  only gates + dialogs are missing). TUI switch reads/writes the existing
  `AppState::tui_scroll` map (default ON); no persistence in Phase 5
  (Phase 6 owns it). Split mapping verbatim: Split right → `"horizontal"`
  (tmux `-h`), Split down → `"vertical"` (tmux `-v`). Window targets ride
  the `paneId` field with the `@N` id (no `windowId` field exists
  server-side). All six layout strings pass through verbatim incl.
  `next-layout`.

### Scope + coverage + icons (D8, D9, D10 carry — LOCKED)
- **D8 — "Join" descoped; window ops extend `tab_strip.rs` (LOCKED):**
  PANE-05's "join" has no backend WS route (`pane.join` absent from
  `protocol.go`, `handler.go`, `websocket.ts`, all of `fe/src` — zero grep
  hits) and no Electron surface. It is logged as a backend gap, not
  invented — no raw tmux, no new message type. PANE-08 adds the Plus button
  (`window.create`, no args) and chip context menus to the existing
  title-bar tabs in place.
- **D9 — Coverage lands with the geometry wave (LOCKED):** new headless
  tests for the geometry module (pixelRect scale, gap absorption, drag-step
  incrementality, FLIP), the STATE.md-flagged missing `window_state.rs`
  clamp-guard test (800..3840 / 500..2160 clamps, -10000 sentinel,
  maximized preservation), WS envelope tests for the 15 pane/window
  commands, pane/window kill-gate tests, rename prefill/gate helper tests.
- **D10 — Nine lucide icons appended to `icons.rs` (LOCKED):**
  SplitSquareHorizontal, SplitSquareVertical, Maximize2, Columns2, Rows2,
  SquareSplitHorizontal, SquareSplitVertical, LayoutGrid, ArrowRightLeft
  (stroke-width 1.5 to match). Tooltips via gpui-component tooltip if
  present in 0.6.0, else hover-reveal label — executor verifies against the
  vendored source.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets (from Phases 1–4)
- `SessionWsHandle::send_command` (correlated send + forget-timeout drop) +
  `MSG_PANE_*`/`MSG_WINDOW_*` consts + `WsIncoming` fields (`pane_id`,
  `direction`, `amount: Option<i32>`, `title`, `other_pane_id`, `layout`,
  `name`) in `crates/backend-client/src/ws.rs` — every Phase-5 mutation
  rides this; no transport work.
- `AppState::apply_event` triple guard + forward pump (all sessions ingest
  regardless of visibility) + `send_window_select` fire-and-forget shape +
  `submit_rename`/`await_rename_result` correlated shape (`app_state.rs`).
- DLG1 dialog clone source: `views/rename_session_dialog.rs:41-126` (open
  guard + prefill + Enter via `InputEvent::PressEnter`) + `views/
  session_context_menu.rs:33-66` (stable-id `context_menu` + destructive
  red Kill) — D6/D7 copy these.
- `TerminalView` dumb views over store-owned `Arc` terminals + mouse
  down/move/up drag-tracking precedent (`terminal_view.rs:305-379`) — D3
  copies the drag pattern; Phase 5 re-hosts views in geometry, never
  touches emulation.
- `AppState::tui_scroll` map + `unwrap_or(true)` default-ON, pane title
  store (D8 titles), `pending_viewport` + layout-key 150/325ms timers,
  `confirm_kill_pane/window` struct flags (`default_true`), `views/
  tab_strip.rs` title-bar tabs (Phase-3 D8 carve-out: no Plus/menu — Phase 5
  owns window ops).
- Parity anchors (all Read-verified in research): `fe/src/lib/geometry.ts`
  (formulas), `PaneWorkspace.tsx` (absolute layout + zoom filter + timers),
  `PaneResizeHandle.tsx` (throttle + FLIP), `PaneHeader.tsx` (header
  contents + split mapping), `PaneContextMenu.tsx` (9-row menu + prefill),
  `PaneView.tsx` (active border + canZoom), `WindowToolbar.tsx` +
  `LayoutSelector.tsx` (six presets), `WindowTabs.tsx` +
  `WindowContextMenu.tsx` (Plus + tab menu + move ±1), `websocket.ts`
  (correlated `runCommand` + 10s timeout).

### Established Patterns
- Generation/epoch guard on async commits → drag steps + mutation awaits
  layer on the same guard; snapshot stays the sole truth.
- Pure `AppState` methods (no GPUI Context) stay headless-testable →
  geometry, drag-step, prefill/gate, kill-gate logic is pure; only paint
  needs a window.
- In-process mock servers in tests (`TcpListener` + `accept_async`) →
  extend `ws_test.rs` mock with pane/window command frames (no new crates).
- Cargo 1.96 accepts a SINGLE test-name filter per invocation (03-01/03-02
  deviation) — plan verify commands run one filter per command.
- Stable element ids as formatted `String`s (`pane-menu/%N`,
  `window-tab/@N`); `.id()` before scroll containers; formatted ids are
  poll-tick stable (no counters/indices).
- Zero new packages — all imports resolve to milestone-pinned workspace
  (`gpui-pre =0.3.3`, `gpui-component =0.6.0`, `alacritty_terminal
  =0.25.1`); `Cargo.lock` committed per Phase 1.

### Integration Points
- `pane.split{%N, horizontal|vertical}` → `split-window -t %N -h|-v`;
  `pane.resize{%N, L|R|U|D, amount}` → `resize-pane -t <t> -<DIR> <amt>`;
  `pane.zoom{%N}` → `resize-pane -Z -t` toggle; `pane.kill/rename/break/
  swap{%N, title?, other_pane_id?}`; `window.select/create/rename/kill/
  layout/move/break-active{@N via paneId, layout?, name?}` — all typed
  `WsIncoming`, server rejects unknown types structurally.
- `fe/src/lib/geometry.ts:23-146` (`CELL_W = 8`, `CELL_H = 18`) is the
  geometry authority; backend protocol unchanged.
</code_context>

<specifics>
## Specific Ideas

- Swap picker lists same-window panes only (`windowId == activeWindowId`
  filter on the flat `snapshot.panes` list); disabled when no other panes
  exist in the window.
- Kill-confirm toggles have no Settings UI until Phase 6 — Phase 5 reads
  the flags (correct `true` defaults) without any toggle UI, same as Phase
  3 did for the session flag.
- TUI-scroll switch is the in-memory map only (FE localStorage parity
  arrives with Phase 6 SET-02).
- User is away: decisions auto-accepted; context kept minimal (research +
  planner files fill detail).
</specifics>

<deferred>
## Specific Ideas / Deferred Ideas

- PANE-05 "join" — descoped per D8 (no WS route, no FE surface); logged as
  a backend gap for post-hoc user audit, not implemented.
- Theme system, terminal prefs persistence, Settings page, command
  palette, font/icon embedding beyond the 9 Phase-5 icons — Phase 6.
- Reconnect banner/toast UX — Phase 7 (STATE-03); Phase 5 surfaces command
  errors inline like Phase 3.
- Session keyboard shortcuts — EXTRA-02, v2.
- Tab persistence on restart — EXTRA-01, v2.
</deferred>
