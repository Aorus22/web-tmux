# Roadmap: Tmux GUI (web-tmux) — Milestone v1.0 Desktop GPUI

## Overview

Milestone v1.0 adds a native GPUI (Rust) desktop frontend that replicates the Electron UI
exactly (1:1) on Windows + Linux, spawning the existing Go backend (`tmux-gui-server`) as a
sidecar and consuming the unchanged REST/WS protocol. The build order mirrors the proven
`web-term/desktop-gpui` reference: foundation crates first (exact dependency pins, supervisor,
settings), then the protocol client chain (REST → WebSocket → session-tab lifecycle), then the
two genuinely novel engineering areas (terminal engine merge, pane grid geometry), then the
generated-data polish cluster (themes, settings page, palette, icons), and finally resilience,
packaging, and the parity audit that closes the "looks like Electron" claim.

Seven phases follow the research's dependency chain (this is the fine end of standard
granularity, justified by 44 requirements and two HIGH-risk novel areas). Every v1.0
requirement maps to exactly one phase. Backend protocol is untouched: any protocol gap found
must be logged as a backend issue, not forked in the GPUI crate.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Workspace Foundation & Backend Sidecar** - desktop-gpui workspace (exact pins + lockfile), supervisor spawn with startup/failed status, settings persistence, custom window controls
- [ ] **Phase 2: REST Client, Sidebar & Session Management** - polling session tree, Create Session dialog, empty/error/select states
- [ ] **Phase 3: WebSocket Client & Multi-Session Tabs** - per-session WS lifecycle with generation guard, session tabs + title bar, correlated mutations
- [ ] **Phase 4: Terminal Engine & Live Pane Rendering** - alacritty-backed live terminals with capture replay, byte-safe input, scrollback, selection
- [ ] **Phase 5: Pane Grid & Workspace Operations** - tmux-geometry pane layout, split/zoom/resize drags, layout presets, pane/window context menus
- [ ] **Phase 6: Theme System, Settings & Chrome Parity** - generated 102+78 theme tables, Settings page, command palette, embedded font/icons
- [ ] **Phase 7: Resilience, Packaging & Parity Audit** - reconnect surfaces, Windows + Linux dist bundles, Makefile integration, 1:1 acceptance checklist

## Phase Details

### Phase 1: Workspace Foundation & Backend Sidecar
**Goal**: The GPUI desktop app exists as a buildable Rust workspace (web-term pin set, committed Cargo.lock), launches its own Go backend sidecar on a dynamic port, and shows the startup path (Starting → Ready / Failed with reason)
**Depends on**: Nothing (first phase)
**Requirements**: SHELL-02, SET-05, STATE-01, PKG-01
**Success Criteria** (what must be TRUE):
  1. Launching the app opens a window that renders "Starting backend…" while the sidecar warms up, then a ready main window; if the backend fails (10s port timeout or early exit), a failed-backend page shows the reason with working Retry and Quit
  2. User can minimize, maximize/restore, and close the app via the custom title-bar window controls
  3. Window position/size persist across an app restart, and a corrupted settings file recovers via .bak instead of breaking launch
  4. `cargo build` succeeds on Windows (release with the ported fxc shader tool) and Linux dev-package prerequisites are documented
**Plans**: TBD
**UI hint**: yes

### Phase 2: REST Client, Sidebar & Session Management
**Goal**: The app shows live tmux state from REST — a polling session→window→pane tree, session creation through a real dialog, and the correct page for every zero/error state
**Depends on**: Phase 1
**Requirements**: SESS-01, SESS-03, SESS-06, SHELL-03, STATE-02
**Success Criteria** (what must be TRUE):
  1. Sidebar shows the collapsible session→window→pane tree (toggle snaps collapsed/expanded) refreshed by ~1.5s polling plus a manual refresh button
  2. A session created from the tmux CLI appears in the sidebar without restarting the app
  3. The Create Session dialog requires a name, offers cwd via the native directory picker plus an optional initial command, and submitting creates the session visible in the tree
  4. Empty (no sessions), Error (tmux missing), and SelectSession states each render matching the Electron equivalents
**Plans**: TBD
**UI hint**: yes

### Phase 3: WebSocket Client & Multi-Session Tabs
**Goal**: The user can hold several tmux sessions open as always-connected workspace tabs with race-free, correlated mutations
**Depends on**: Phase 2
**Requirements**: SHELL-01, SESS-02, SESS-04, SESS-05, STATE-04
**Success Criteria** (what must be TRUE):
  1. The user opens multiple sessions as tabs; all stay connected simultaneously and switching between them never disconnects or re-handshakes a live session
  2. The title bar renders drag region, app identity, the active session's tmux-window tabs, and the Settings gear — visually identical to the Electron title bar
  3. Renaming a session via its context menu opens a dialog with a real text input and the new name propagates to sidebar and tabs — including mid-session socket re-resolution
  4. Killing a session via context menu confirms (when the stored kill-confirm setting says so) and closes its tab cleanly
  5. Rapid tab switching while a session churns never applies another session's snapshot events to the open tab (WS generation guard contract)
**Plans**: TBD
**UI hint**: yes

### Phase 4: Terminal Engine & Live Pane Rendering
**Goal**: Every pane renders a live alacritty_terminal-backed terminal through the full capture-replay → live-output pipeline, with byte-safe input, scrollback, selection, and storm-free resize
**Depends on**: Phase 3
**Requirements**: TERM-01, TERM-02, TERM-03, TERM-04, TERM-05, TERM-06, TERM-07
**Success Criteria** (what must be TRUE):
  1. A pane's live output renders in the terminal grid and matches the other frontends on the same workload (vim/htop look identical)
  2. Opening a pane with history replays captured scrollback exactly once and continues live seamlessly — including reconnect (never doubled history or a blank screen)
  3. Typed input including special keys reaches the pane byte-safe through `terminal.input`; wheel scrolls scrollback, and with TUI-scroll on the wheel sends PageUp/PageDown to TUI panes
  4. User can select text and copy it to the clipboard, and paste clipboard content into the terminal
  5. Window resizes report cols/rows with the Electron debounce dance (no resize storms); inactive hidden session tabs keep ingesting output and catch up when reshown
**Plans**: TBD

### Phase 5: Pane Grid & Workspace Operations
**Goal**: The workspace faithfully displays and manipulates real tmux pane geometry — layout, dividers, zoom, presets, and the full pane/window context menus
**Depends on**: Phase 3 (snapshots) and Phase 4 (rendered terminals)
**Requirements**: PANE-01, PANE-02, PANE-03, PANE-04, PANE-05, PANE-06, PANE-07, PANE-08, DLG-02
**Success Criteria** (what must be TRUE):
  1. Panes render at positions/sizes derived from tmux cell geometry (pixelRect + closePaneGaps) matching Electron's rendering of the same layout
  2. User can split a pane right/down via header buttons or context menu (targeted by stable pane ID) and zoom a pane to fill the workspace with the next action restoring the layout
  3. Dragging pane dividers resizes via throttled incremental cell steps (~40ms) including negative-direction flips, without resize storms
  4. Pane context menu kills/renames/swaps/breaks/joins panes and the title-bar tab context menu renames/moves/breaks/kills windows — with every rename flow (session/window/pane) opening a dialog with a real text input
  5. Layout presets (even-h, even-v, main-h, main-v, tiled, next) apply from the window toolbar; pane headers show path, pane ID, TUI-scroll switch, and quick actions with tooltips
**Plans**: TBD
**UI hint**: yes

### Phase 6: Theme System, Settings & Chrome Parity
**Goal**: Appearance and preferences reach Electron parity — all generated themes live-applied, a full Settings page, the command palette, and embedded font/icons
**Depends on**: Phases 2–4 (sidebar for chrome re-theming, dialogs validated, terminals for live preview)
**Requirements**: SHELL-04, SHELL-05, SET-01, SET-02, SET-03, SET-04, THEME-01, THEME-02, THEME-03, DLG-01, DLG-03
**Success Criteria** (what must be TRUE):
  1. The Settings page shows a 3-column UI-theme card grid with a dark/light filter; picking a theme restyles all chrome live, terminals follow the matched theme unless overridden, and the selection plus window geometry persist across restarts
  2. Terminal preferences (font size, line height, scrollback lines, per-pane TUI-scroll default) take effect immediately and persist
  3. The tmux binary path setting validates against `/api/tmux/binary` with status/error surfaced in the UI
  4. Kill-confirmation dialogs honor the three per-surface settings toggles exactly like Electron, and dark/light counterpart themes behave with Electron's dark semantics (colorScheme)
  5. All 102 UI + 78 terminal presets are available as generated files; Ctrl+Shift+P opens a filterable command palette including the Open Session group; all text and icons render from the embedded JetBrains Mono font and embedded lucide SVGs (no system dependencies)
**Plans**: TBD
**UI hint**: yes

### Phase 7: Resilience, Packaging & Parity Audit
**Goal**: The app survives tmux/backend turbulence gracefully, ships as packaged bundles for Windows + Linux, and passes the 1:1 parity checklist that closes the milestone
**Depends on**: Phases 1–6
**Requirements**: STATE-03, PKG-02, PKG-03
**Success Criteria** (what must be TRUE):
  1. tmux disconnect/reconnect shows the reconnect banner and toasts with the strict taxonomy (WS-drop ≠ tmux disconnected/reconnecting), and command errors surface as toasts
  2. Package scripts produce Windows and Linux dist bundles with the Go backend sidecar adjacent to the app executable, and the packaged app launches and reaches a session on both OSes
  3. Makefile targets build/dev/package desktop-gpui as part of the repo workflow on both OSes
  4. A side-by-side parity audit against Electron over the daily-driver checklist (vim/htop, rapid session switching, resize flood, rename mid-session, theme swap mid-session) shows 1:1 behavior with only the documented deviation (sidebar binary snap)
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Workspace Foundation & Backend Sidecar | TBD | Not started | - |
| 2. REST Client, Sidebar & Session Management | TBD | Not started | - |
| 3. WebSocket Client & Multi-Session Tabs | TBD | Not started | - |
| 4. Terminal Engine & Live Pane Rendering | TBD | Not started | - |
| 5. Pane Grid & Workspace Operations | TBD | Not started | - |
| 6. Theme System, Settings & Chrome Parity | TBD | Not started | - |
| 7. Resilience, Packaging & Parity Audit | TBD | Not started | - |

---
*Roadmap created: 2026-09-06 for milestone v1.0 (Desktop GPUI) — 44/44 v1.0 requirements mapped*
