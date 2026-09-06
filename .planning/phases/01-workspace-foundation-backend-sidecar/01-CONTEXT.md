# Phase 1: Workspace Foundation & Backend Sidecar - Context

**Gathered:** 2026-09-06
**Status:** Ready for planning
**Mode:** Auto-accepted (smart discuss — user authorized unattended execution; grey areas resolved from the proven web-term reference; CTRL-03: this documents decisions a live discuss would have ratified)

<domain>
## Phase Boundary

A buildable `desktop-gpui/` Rust workspace that spawns the Go backend sidecar on a dynamic
port and renders the Electron startup path (Starting → Ready / Failed with reason + Retry/Quit),
plus custom window controls, window-geometry persistence and settings persistence (.bak recovery).
In: workspace skeleton (5 crates, exact web-term pins, committed Cargo.lock), supervisor spawn
pipeline, status UI, window controls, settings/window-state persistence, Windows (incl. release
fxc) + Linux build readability. Out: REST/WS clients (Phases 2–3), terminal engine (Phase 4),
pane grid (Phase 5), theme/settings content + palette (Phase 6), package bundles (Phase 7).

</domain>

<decisions>
## Implementation Decisions

### Workspace layout & pins
- `desktop-gpui/` at repo root, 5-crate workspace mirroring web-term (`supervisor`, `settings`, `backend-client`, `terminal`, app crate named `webtmux`)
- All five crates created now; `backend-client`/`terminal` as compilable stubs filled by later phases
- Workspace Cargo.toml copies web-term pins verbatim: gpui-pre =0.3.3, gpui-component =0.6.0, alacritty_terminal =0.25.1, tokio =1.53.1, reqwest 0.12.28 (rustls), tokio-tungstenite 0.26.2, flume, parking_lot, dirs; app crate adds `gpui-platform = { package = "gpui-pre-platform", version = "=0.3.3" }`; Cargo.lock committed
- fxc HLSL tool ported from web-term `tools/fxc` (Windows release builds only; set via GPUI_FXC_PATH)

### Sidecar spawn & port discovery
- Locate `tmux-gui-server(.exe)` adjacent to the running executable; repo-root fallback for dev runs; env override for tests
- Spawn with `--port 0`; parse `BACKEND_PORT:<n>` from stdout (Electron desktop/main.js handshake parity); probe `/api/health` for readiness
- Strip `TMUX`/`TMUX_PANE` from the child environment; drop web-term-only spawn args (encryption key, db path)
- BackendStatus::{Starting, Ready, Failed{reason, stderr_tail}}; 10s port timeout → Failed (Electron parity)

### Startup UI shell
- Starting page mirrors Electron: centered spinner + "Starting backend…" muted text; Failed page shows reason + redacted stderr tail + working Retry (re-spawned supervisor) and Quit
- Phase 1 title bar is minimal: drag region + custom min/max/restore/close; full tab strip/gear arrives Phases 3/6 (staged to keep phase boundaries clean)
- Port web-term `window_state.rs` verbatim (restore at boot, observe + save on move/resize/maximize)
- Port the Windows DWM dark-frame block from web-term `main.rs` verbatim now (LOW cost, avoids later dark-frame bugs)

### Settings persistence
- Port webterm-settings crate pattern: JSON settings in the platform config dir via `dirs`, corrupt-file `.bak` recovery, defaults on first run
- Phase 1 fields: window geometry + theme placeholder; tmux/terminal preference fields arrive with the Phase 6 data model

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- E:\Coding Stuff\web-term\desktop-gpui: supervisor crate (spawn + port handshake + health wait), settings crate (load/save/.bak), webterm `window_state.rs` (restore/observe), `main.rs` (font embedding, window options, DWM block, TOKIO_RT guard), `status.rs` (Starting/Failed pages), `theme.rs::apply_theme`
- web-tmux backend: `be/cmd/server/main.go` prints `BACKEND_PORT:<n>` under `--port 0`; `/api/health` exists; `make be` builds repo-root `tmux-gui-server(.exe)`
- STACK research: zero extra runtime crates; release builds need the fxc drop-in; Linux needs fontconfig/xkbcommon/X11/Wayland dev libs + Vulkan loader

### Established Patterns
- Exact `=` dep pins + committed Cargo.lock (gpui is pre-1.0 with breaking churn)
- Tokio entered once in `main()` (`TOKIO_RT.enter()`) before GPUI `Application::run`
- Free-function views reading theme helpers from the AppState entity

### Integration Points
- Sidecar exe name/path resolution (bundle.rs pattern) is the seam between supervisor and the existing Go build
- `BACKEND_PORT` stdout line ↔ Electron parity — do not invent a new handshake

</code_context>

<specifics>
## Specific Ideas

- User directive: 1:1 port of the Electron UI — no added functionality, Windows + Linux
- Accepted deviation on record: sidebar collapse is a binary snap (no CSS-like slide)
- Decisions here were auto-accepted from the web-term reference per the user's "don't ask anything" instruction — flagged for post-hoc audit

</specifics>

<deferred>
## Deferred Ideas

- Session-tab persistence across restarts (Electron restores nothing) — EXTRA-01, v2
- Session-level keyboard shortcuts — EXTRA-02, v2
</deferred>
