---
phase: 01-workspace-foundation-backend-sidecar
plan: "01"
subsystem: desktop-gpui
tags: [rust, gpui, supervisor, sidecar, tokio, desktop]

# Dependency graph
requires: []
provides:
  - 5-crate desktop-gpui workspace with exact pinned dependencies and committed Cargo.lock
  - Headless webtmux-supervisor crate with env-only spawn, BACKEND_PORT stdout handshake, and health probe
  - Runnable webtmux tracer app booting to Starting (S2), Ready (S4), or Failed (S3) status surfaces
affects: [01-02-PLAN, 01-03-PLAN, 02-rest-client-session-tree, 03-websocket-terminal-io]

actuals:
  tokens: 28500
  tasks: 3
  commits: 4

# Tech tracking
tech-stack:
  added:
    - "gpui-pre =0.3.3"
    - "gpui-pre-platform =0.3.3"
    - "gpui-component =0.6.0"
    - "tokio =1.53.1"
    - "reqwest =0.12.28 (rustls)"
    - "dirs =6.0.0"
    - "parking_lot =0.12.5"
    - "thiserror =2.0.20"
    - "raw-window-handle =0.6.2"
  patterns:
    - "TOKIO_RT LazyLock runtime entered at bootstrap before Application::run"
    - "Watch-to-mpsc-to-GPUI async status pump with weak.upgrade() listener lifecycle"
    - "Env-only child process spawn stripping TMUX/TMUX_PANE with empty argv"
    - "Newest-wins 2000-character stderr ring buffer with secret redaction"
    - "Frameless window with WindowControlArea::Drag and Win32 DWM dark frame"

key-files:
  created:
    - desktop-gpui/Cargo.toml
    - desktop-gpui/Cargo.lock
    - desktop-gpui/assets/fonts/JetBrainsMono-Regular.ttf
    - desktop-gpui/crates/supervisor/src/lib.rs
    - desktop-gpui/crates/supervisor/tests/integration.rs
    - desktop-gpui/crates/settings/src/lib.rs
    - desktop-gpui/crates/settings/src/paths.rs
    - desktop-gpui/crates/backend-client/src/lib.rs
    - desktop-gpui/crates/terminal/src/lib.rs
    - desktop-gpui/crates/webtmux/src/main.rs
    - desktop-gpui/crates/webtmux/src/app_state.rs
    - desktop-gpui/crates/webtmux/src/bundle.rs
    - desktop-gpui/crates/webtmux/src/icons.rs
    - desktop-gpui/crates/webtmux/src/theme.rs
    - desktop-gpui/crates/webtmux/src/views/mod.rs
    - desktop-gpui/crates/webtmux/src/views/status.rs
    - desktop-gpui/crates/webtmux/src/views/tab_strip.rs
    - desktop-gpui/crates/webtmux/src/window_state.rs
    - scripts/build-test-backend.cmd
    - scripts/build-test-backend.sh
  modified: []

key-decisions:
  - "Spawn Go backend with empty argv and TMUXGUI_PORT=0 env variable (matching Go config.go os.Getenv-only contract)"
  - "Use 3-variant BackendStatus enum (Starting, Ready, Failed) folding early-exits into Failed reason without Crashed variant"
  - "Embed JetBrainsMono-Regular.ttf byte-identically and register with GPUI text system prior to initial element shaping"
  - "Support Win32 DWM immersive dark mode frame attributes (20 and 19) at window open callback"

patterns-established:
  - "Pattern 1: Sidecar child spawn via tokio::process::Command with kill_on_drop(true), null stdin, and piped stdout/stderr"
  - "Pattern 2: Handshake parser reading BACKEND_PORT:<port> from stdout and launching background stdout drain"
  - "Pattern 3: Polling /api/health probe with 250ms cadence and 10s timeout before transitioning to Ready"

requirements-completed:
  - STATE-01
  - PKG-01

coverage:
  - id: D1
    description: "5-crate desktop-gpui workspace compiles under cargo check and cargo build on Windows in debug mode"
    requirement: PKG-01
    verification:
      - kind: other
        ref: "cargo build --manifest-path desktop-gpui/Cargo.toml --package webtmux"
        status: pass
    human_judgment: false
  - id: D2
    description: "webtmux-supervisor spawns Go backend with empty argv and TMUXGUI_PORT=0, parses BACKEND_PORT, probes /api/health to reach Ready, and supports adopt_or_clear"
    requirement: STATE-01
    verification:
      - kind: integration
        ref: "desktop-gpui/crates/supervisor/tests/integration.rs#reaches_ready"
        status: pass
      - kind: integration
        ref: "desktop-gpui/crates/supervisor/tests/integration.rs#adopt_or_clear"
        status: pass
      - kind: integration
        ref: "desktop-gpui/crates/supervisor/tests/integration.rs#invalid_path"
        status: pass
      - kind: integration
        ref: "desktop-gpui/crates/supervisor/tests/integration.rs#handshake_timeout"
        status: pass
      - kind: integration
        ref: "desktop-gpui/crates/supervisor/tests/integration.rs#early_exit"
        status: pass
    human_judgment: false
  - id: D3
    description: "webtmux tracer app boots window with S1 title bar, S2 Starting page, S3 Failed page with Quit/Retry, and S4 Ready shell"
    requirement: STATE-01
    verification:
      - kind: other
        ref: "cargo build --manifest-path desktop-gpui/Cargo.toml --package webtmux"
        status: pass
    human_judgment: false

duration: 22min
completed: 2026-09-06
status: complete
---

# Phase 01 Plan 01: Workspace Foundation & Backend Sidecar Tracer Slice Summary

**5-crate desktop-gpui Rust workspace initialized with exact pinned dependencies, headless webtmux-supervisor sidecar lifecycle management, and a runnable webtmux tracer app booting through Starting, Ready, and Failed UI states.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-09-06T10:12:00Z
- **Completed:** 2026-09-06T10:34:00Z
- **Tasks:** 3
- **Files modified:** 20

## Accomplishments

- Established root `desktop-gpui/Cargo.toml` with `resolver = "2"`, exact dependency pins (`gpui-pre =0.3.3`, `gpui-component =0.6.0`, `tokio =1.53.1`, `reqwest =0.12.28`, `dirs =6.0.0`), and committed lockfile `desktop-gpui/Cargo.lock`.
- Implemented `webtmux-supervisor` with env-only spawn contract (`TMUXGUI_PORT=0`, `TMUXGUI_HOST=127.0.0.1`, `TMUXGUI_TMUX_BIN`, `TMUX`/`TMUX_PANE` stripped), `BACKEND_PORT:<n>` stdout handshake parser, background stdout drain, 250ms `/api/health` polling loop, 2000-char stderr ring buffer, child exit monitor, and `adopt_or_clear` recovery.
- Implemented `webtmux` application tracer slice with `TOKIO_RT` runtime, `AppState` watch-to-mpsc pump, `JetBrainsMono-Regular.ttf` embedded font, DWM immersive dark mode frame attrs, S1 minimal title bar (44px, drag area, custom window controls), S2 Starting page (rotating Loader2 + "Starting backend…"), S3 Failed page (700px card, border `#7F1D1D`, redacted stderr tail, Quit/Retry), and bare `#1e1e1e` S4 Ready body.

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold 5-crate desktop-gpui workspace and commit Cargo.lock** - `2a85045` (feat)
2. **Task 2 (TDD RED): Add failing test for webtmux-supervisor** - `54da97d` (test)
3. **Task 2 (TDD GREEN): Implement webtmux-supervisor crate with env-only spawn contract** - `3962809` (feat)
4. **Task 3: Implement webtmux tracer app booting to Starting, Ready, and Failed UI states** - `1c21912` (feat)

## Files Created/Modified

- `desktop-gpui/Cargo.toml` - 5-crate workspace manifest with exact pinned dependencies
- `desktop-gpui/Cargo.lock` - Committed lockfile for reproducible GPUI dependency graph
- `desktop-gpui/assets/fonts/JetBrainsMono-Regular.ttf` - Embedded JetBrains Mono font asset
- `desktop-gpui/crates/supervisor/Cargo.toml` - Supervisor crate manifest
- `desktop-gpui/crates/supervisor/src/lib.rs` - Supervisor lifecycle, spawn, handshake, health probe, and status enum
- `desktop-gpui/crates/supervisor/tests/integration.rs` - Unit and integration tests for supervisor
- `desktop-gpui/crates/settings/Cargo.toml` - Settings store crate manifest
- `desktop-gpui/crates/settings/src/lib.rs` - Settings data models, JSON serialization, and .bak recovery
- `desktop-gpui/crates/settings/src/paths.rs` - Config paths helper resolving to `tmux-gui-desktop`
- `desktop-gpui/crates/backend-client/Cargo.toml` - Backend client stub manifest
- `desktop-gpui/crates/backend-client/src/lib.rs` - Backend client stub entry point
- `desktop-gpui/crates/terminal/Cargo.toml` - Terminal engine stub manifest
- `desktop-gpui/crates/terminal/src/lib.rs` - Terminal engine stub entry point
- `desktop-gpui/crates/webtmux/Cargo.toml` - webtmux application crate manifest
- `desktop-gpui/crates/webtmux/src/lib.rs` - webtmux library root exposing modules
- `desktop-gpui/crates/webtmux/src/main.rs` - App entry point with runtime guard and window bootstrap
- `desktop-gpui/crates/webtmux/src/app_state.rs` - AppState entity and supervisor event pump
- `desktop-gpui/crates/webtmux/src/bundle.rs` - Backend binary path resolution
- `desktop-gpui/crates/webtmux/src/icons.rs` - Embedded lucide SVG constants
- `desktop-gpui/crates/webtmux/src/theme.rs` - Theme management bridge and color helpers
- `desktop-gpui/crates/webtmux/src/views/mod.rs` - Views module definition
- `desktop-gpui/crates/webtmux/src/views/status.rs` - S2 Starting and S3 Failed status page rendering
- `desktop-gpui/crates/webtmux/src/views/tab_strip.rs` - S1 title bar with custom window controls
- `desktop-gpui/crates/webtmux/src/window_state.rs` - Window geometry restoration and toggle maximize/restore
- `scripts/build-test-backend.cmd` - Windows batch script to build Go backend fixture
- `scripts/build-test-backend.sh` - Shell script to build Go backend fixture

## Decisions Made

- Spawn Go backend with empty argv and `TMUXGUI_PORT=0` env variable to respect the Go backend's `os.Getenv`-only configuration contract.
- Use a 3-variant `BackendStatus` enum (`Starting`, `Ready`, `Failed`) where early child process exits fold cleanly into `BackendStatus::Failed { reason, stderr_tail }`.
- Embed `JetBrainsMono-Regular.ttf` directly into the binary and register it with GPUI before any text shaping occurs.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - build, TDD cycle, and workspace integration tests succeeded cleanly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The 5-crate workspace and core supervisor lifecycle are verified and ready for Plan 01-02 (Settings & Window Controls) and Plan 01-03 (Release Tooling & CI Validation).

---

## Self-Check: PASSED

1. Created files exist on disk:
   - `desktop-gpui/Cargo.toml` (FOUND)
   - `desktop-gpui/Cargo.lock` (FOUND)
   - `desktop-gpui/crates/supervisor/src/lib.rs` (FOUND)
   - `desktop-gpui/crates/webtmux/src/main.rs` (FOUND)
   - `desktop-gpui/crates/webtmux/src/app_state.rs` (FOUND)
   - `desktop-gpui/crates/webtmux/src/views/status.rs` (FOUND)
2. Task commits exist in git log:
   - `2a85045` (FOUND)
   - `54da97d` (FOUND)
   - `3962809` (FOUND)
   - `1c21912` (FOUND)
3. Full verification battery passing: `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` (PASSED: 9 unit/integration tests green).
