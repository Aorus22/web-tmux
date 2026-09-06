# Requirements: Tmux GUI (web-tmux)

**Defined:** 2026-09-06
**Core Value:** Full visual control over tmux while tmux remains the single source of truth

## v1 Requirements

Requirements for milestone v1.0 (Desktop GPUI). Each maps to roadmap phases.

### Shell (window chrome)

- [ ] **SHELL-01**: User sees a frameless title bar with drag region, app identity, tmux-window tabs of the active session, and a Settings gear — visually identical to the Electron title bar
- [ ] **SHELL-02**: User can minimize/maximize/restore/close via the custom title-bar window controls
- [ ] **SHELL-03**: User can toggle the sidebar collapsed/expanded (binary snap; no slide animation — accepted deviation, documented)
- [ ] **SHELL-04**: Window geometry (position/size) and theme preference persist across restarts
- [ ] **SHELL-05**: App renders with the embedded monospace font (JetBrains Mono) and embedded lucide SVG icons — no system font or icon dependency

### Sessions (sidebar & lifecycle)

- [ ] **SESS-01**: User sees a collapsible session→window→pane tree in the sidebar, refreshed by 1.5s polling plus a manual refresh button
- [ ] **SESS-02**: User can open a session as a workspace; multiple sessions stay connected simultaneously and switching never tears down a connection
- [ ] **SESS-03**: User can create a session (name required, optional cwd + initial command) via the Create Session dialog with a native directory picker
- [ ] **SESS-04**: User can rename a session via context menu (dialog with real text input)
- [ ] **SESS-05**: User can kill a session via context menu with confirmation (respecting the kill-confirm setting)
- [ ] **SESS-06**: Sessions created from the tmux CLI appear in the sidebar without an app restart

### Panes (grid & window ops)

- [ ] **PANE-01**: Panes render at positions/sizes derived from tmux cell geometry (pixelRect + closePaneGaps), matching the Electron layout exactly
- [ ] **PANE-02**: User can split a pane right/down via pane header buttons or context menu, targeted by stable pane ID (`%N`)
- [ ] **PANE-03**: User can zoom a pane to fill the workspace (button or double-click header); the next action restores the layout
- [ ] **PANE-04**: User can drag pane dividers to resize; drags send incremental cell steps, throttled (~40ms), with negative-direction flip — matching Electron semantics
- [ ] **PANE-05**: User can kill, rename, swap, break, and join panes via the pane context menu (confirmations where Electron confirms)
- [ ] **PANE-06**: User can apply layout presets (even-h, even-v, main-h, main-v, tiled, next) from the window toolbar
- [ ] **PANE-07**: User sees pane headers with path, pane ID, TUI-scroll switch, and quick actions with tooltips
- [ ] **PANE-08**: User can rename/move left/right/break/kill windows via the title-bar tab context menu

### Terminal (rendering & I/O)

- [ ] **TERM-01**: Terminal output renders in an alacritty_terminal-backed grid per pane (same engine as web-term)
- [ ] **TERM-02**: Opening a pane replays history: capture blob ingests the scrollback delta then rebuilds the positioned screen (applyCapture semantics with ingestedHistory counter)
- [ ] **TERM-03**: Keyboard input reaches the pane byte-safe through the WS `terminal.input` path
- [ ] **TERM-04**: User can scroll scrollback with the wheel; with the TUI-scroll switch on, the wheel sends PageUp/PageDown to TUI panes
- [ ] **TERM-05**: User can select text and copy/paste via the clipboard
- [ ] **TERM-06**: Terminals report cols/rows on container resize with the Electron debounce dance (~100ms resize, 150/325ms capture resync, layout-key invalidation) — no resize storms
- [ ] **TERM-07**: Inactive (hidden) session workspaces keep ingesting terminal output while not rendered

### Dialogs & palette

- [ ] **DLG-01**: User can open the command palette with Ctrl+Shift+P, filter as they type, and execute actions including the Open Session group
- [ ] **DLG-02**: All rename flows (session/window/pane) open a dialog with a real text input (gpui-component Input)
- [ ] **DLG-03**: Kill confirmations honor the per-surface settings toggles exactly like the Electron helpers

### Settings

- [ ] **SET-01**: User can pick a UI theme from a 3-column card grid with a theme-mode filter (dark/light)
- [ ] **SET-02**: User can configure terminal prefs: font size, line height, scrollback lines, per-pane TUI-scroll default
- [ ] **SET-03**: User can set the tmux binary path (Windows), validated against `/api/tmux/binary` with status/error surfaced
- [ ] **SET-04**: User can toggle the three kill-confirmation switches
- [ ] **SET-05**: Settings persist across restarts (webterm settings-crate pattern)

### Theming

- [ ] **THEME-01**: All 102 UI theme presets and 78 terminal presets from the Electron FE are available (script-generated port, generator-compatible files)
- [ ] **THEME-02**: Selecting a UI theme applies live across all chrome; the terminal follows the matched terminal theme unless overridden
- [ ] **THEME-03**: Dark/light counterparts behave like Electron (dark semantics, colorScheme)

### States & resilience

- [ ] **STATE-01**: User sees "Starting backend…" while the sidecar warms up, and a failed-backend page with reason + Retry/Quit after the 10s port timeout
- [ ] **STATE-02**: User sees Empty (no sessions), Error (tmux missing), and SelectSession states matching Electron
- [ ] **STATE-03**: User sees a reconnect banner and toasts on `tmux.disconnected`/`tmux.reconnecting` and command errors
- [ ] **STATE-04**: A WS generation guard ensures stale session events never render into the wrong tab

### Build & packaging

- [ ] **PKG-01**: `cargo build` works on Windows (debug without fxc; release with the ported fxc tool) and Linux (dev-package prerequisites documented)
- [ ] **PKG-02**: Package scripts produce Windows + Linux dist bundles with the Go backend sidecar adjacent to the app executable
- [ ] **PKG-03**: Makefile targets integrate desktop-gpui build/dev into the repo workflow

## v2 Requirements

Deferred to future milestones. Tracked but not in current roadmap.

### Desktop-native extras

- **EXTRA-01**: Session-tab persistence across app restarts (Electron restores nothing; web-term persists — deferred to keep 1:1 parity)
- **EXTRA-02**: Session-level keyboard shortcuts (ctrl-tab / alt-1..9 session switching)
- **EXTRA-03**: Linux window-control polish beyond the shared window_state/DWM path

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Backend protocol changes | Frontend-only milestone; the existing REST/WS surface already serves three frontends |
| macOS support | No macOS machine for testing; web-term's proven pattern covers Windows + Linux |
| shadcn components ported 1:1 as GPUI widgets | Anti-feature: widget default styling diverges from the 1:1 goal; hand-rolled divs + gpui-component behaviors only |
| CSS-like animations (sidebar slide, hover transitions) | GPUI has no transition equivalent; static snap accepted (documented deviation) |
| Mobile frontend changes | Separate surface, untouched this milestone |
| Replacing tmux / app-owned terminal backend | PRD principle: tmux owns all state |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| (populated by roadmap) | | |

**Coverage:**
- v1 requirements: 44 total
- Mapped to phases: 0
- Unmapped: 44 ⚠️

---
*Requirements defined: 2026-09-06*
*Last updated: 2026-09-06 after initial definition*
