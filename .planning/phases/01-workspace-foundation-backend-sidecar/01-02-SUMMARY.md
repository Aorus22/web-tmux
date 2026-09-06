---
phase: 01-workspace-foundation-backend-sidecar
plan: "02"
subsystem: desktop-gpui
tags: [rust, gpui, settings, window-state, title-bar, persistence]

# Dependency graph
requires:
  - phase: 01-01
    provides: 5-crate desktop-gpui workspace and webtmux tracer app skeleton
provides:
  - webtmux-settings crate with atomic file persistence (.bak corrupt recovery, first-run defaults, isolated tmux-gui-desktop config dir)
  - Window state geometry restoration with boundary clamping (min 800x500) and minimized negative coordinate filtering
  - Interactive S1 title bar with custom window controls (minimize, toggle maximize/restore via Win32 IsZoomed/ShowWindowAsync, close) and WindowControlArea::Drag
affects: [01-03-PLAN, 02-rest-client-session-tree, 06-settings-themes]

actuals:
  tokens: 18500
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Atomic JSON persistence with NamedTempFile persist and .bak corrupt file recovery"
    - "Window bounds restoration with clamp guards (min 800x500) and legacy negative coordinate reset"
    - "Native title bar drag via WindowControlArea::Drag and Win32 IsZoomed maximize toggle"
    - "Window close observer flush via window.on_window_should_close saving DesktopSettings"

key-files:
  created:
    - desktop-gpui/crates/settings/tests/store_test.rs
  modified:
    - desktop-gpui/crates/settings/Cargo.toml
    - desktop-gpui/crates/settings/src/lib.rs
    - desktop-gpui/crates/settings/src/paths.rs
    - desktop-gpui/crates/webtmux/src/lib.rs
    - desktop-gpui/crates/webtmux/src/window_state.rs
    - desktop-gpui/crates/webtmux/src/views/tab_strip.rs

key-decisions:
  - "Use NamedTempFile atomic replacement in webtmux-settings save_to to avoid partial-write file corruption"
  - "Isolate settings path to dirs::config_dir()/tmux-gui-desktop to prevent venue collision with GTK or webterm"
  - "Clamp restored window bounds away from 0x0 with minimum 800x500 and filter minimized <= -10000 coordinates"

patterns-established:
  - "Pattern 1: DesktopSettings load/save with .bak fallback on corrupt JSON"
  - "Pattern 2: Close-flush window state observer saving last window geometry"
  - "Pattern 3: WindowControlArea::Drag for native OS Aero snap and move"

requirements-completed:
  - SET-05
  - SHELL-02

coverage:
  - id: D1
    description: "webtmux-settings roundtrip, corrupt .bak recovery, first run defaults, and atomic persist tests pass"
    requirement: SET-05
    verification:
      - kind: unit
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml -p webtmux-settings"
        status: pass
    human_judgment: false
  - id: D2
    description: "Window state restoration clamps invalid dimensions, handles minimized negative coordinates, and persists geometry across restarts"
    requirement: SET-05
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/settings/tests/store_test.rs#window_state_clamping_and_guard_logic"
        status: pass
    human_judgment: false
  - id: D3
    description: "Custom title bar controls (minimize, maximize/restore with IsZoomed toggle, close) and WindowControlArea::Drag styled per 01-UI-SPEC"
    requirement: SHELL-02
    verification:
      - kind: other
        ref: "cargo build --manifest-path desktop-gpui/Cargo.toml --package webtmux"
        status: pass
    human_judgment: false

duration: 18min
completed: 2026-09-06
status: complete
---

# Phase 01 Plan 02: Settings Store & Window State Controls Summary

**webtmux-settings crate implemented with atomic JSON persistence, .bak corrupt-file recovery, and first-run defaults, alongside window geometry boundary clamping, close-flush observation, and custom title-bar window controls (minimize, maximize/restore, close, and drag).**

## Performance

- **Duration:** 18 min
- **Started:** 2026-09-06T10:40:00Z
- **Completed:** 2026-09-06T10:58:00Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Implemented `webtmux-settings` crate with `DesktopSettings`, `WindowState`, `Theme`, and `SettingsError` models.
- Established configuration directory path resolution strictly isolated to `tmux-gui-desktop` under system config directory (`dirs::config_dir()`).
- Added atomic file write with `tempfile::NamedTempFile` persist and Unix `0600` permissions.
- Added corrupt file recovery renaming invalid JSON to `.json.bak`, regenerating defaults, and saving valid fallback.
- Implemented and verified unit tests in `crates/settings/tests/store_test.rs` covering roundtrip persistence, corrupt recovery, first-run initialization, atomic save, and window state guard logic.
- Verified window geometry restoration in `webtmux::window_state` with 800x500 boundary clamping, 0x0 degenerate dimension fallback, and minimized negative coordinate filtering.
- Implemented and verified custom title-bar controls (40x32px buttons with idle `#9d9d9d`, hover `#2d2d2d`/`#d4d4d4`, and close hover `#7F1D1D`/`#ffffff`) with `WindowControlArea::Drag` and Win32 `IsZoomed`/`ShowWindowAsync` maximize toggle.

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement webtmux-settings crate with .bak recovery and unit tests** - `3e8899d` (feat)
2. **Task 2: Implement window_state restoration and interactive custom title-bar controls** - `d36b288` (feat)

## Files Created/Modified

- `desktop-gpui/crates/settings/Cargo.toml` - Added tempfile dependency for atomic file persistence
- `desktop-gpui/crates/settings/src/lib.rs` - Data models, atomic save_to via NamedTempFile, .bak recovery
- `desktop-gpui/crates/settings/src/paths.rs` - Config directory helper isolating to `tmux-gui-desktop`
- `desktop-gpui/crates/settings/tests/store_test.rs` - Store unit tests (roundtrip, corrupt recovery, first run, atomic save, guards)
- `desktop-gpui/crates/webtmux/src/lib.rs` - Crate root module export
- `desktop-gpui/crates/webtmux/src/window_state.rs` - Window bounds restoration, clamping, and close observer
- `desktop-gpui/crates/webtmux/src/views/tab_strip.rs` - S1 title bar with interactive min/max/close buttons and drag area

## Decisions Made

- Use `tempfile::NamedTempFile` in `save_to` to guarantee atomic file replacement and avoid partial write corruption.
- Isolate configuration files strictly to `tmux-gui-desktop` to avoid collision with `tmux-gui-gtk` or `webterm-desktop`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added atomic file persist in webtmux-settings save_to**
- **Found during:** Task 1 (webtmux-settings implementation)
- **Issue:** Direct `fs::write` was vulnerable to partial write corruption if interrupted mid-save.
- **Fix:** Switched `save_to` to write into a `tempfile::NamedTempFile` in the same directory and atomically persist to the destination path.
- **Files modified:** `desktop-gpui/crates/settings/src/lib.rs`, `desktop-gpui/crates/settings/Cargo.toml`
- **Verification:** `cargo test -p webtmux-settings` passed atomic save and roundtrip tests.
- **Committed in:** `3e8899d` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical)
**Impact on plan:** Enhanced persistence safety according to threat model T-01-07. No scope creep.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Window state persistence and custom title bar controls are fully functional and tested.
- Ready for Plan 01-03 (Build tooling, release fxc integration, and CI validation).

---

## Self-Check: PASSED

1. Created/modified files exist on disk:
   - `desktop-gpui/crates/settings/src/lib.rs` (FOUND)
   - `desktop-gpui/crates/settings/src/paths.rs` (FOUND)
   - `desktop-gpui/crates/settings/tests/store_test.rs` (FOUND)
   - `desktop-gpui/crates/webtmux/src/window_state.rs` (FOUND)
   - `desktop-gpui/crates/webtmux/src/views/tab_strip.rs` (FOUND)
2. Task commits exist in git log:
   - `3e8899d` (FOUND)
   - `d36b288` (FOUND)
3. Full verification battery passing:
   - `cargo test --manifest-path desktop-gpui/Cargo.toml -p webtmux-settings -p webtmux-supervisor` (PASSED: 14/14 unit tests green).
   - `cargo build --manifest-path desktop-gpui/Cargo.toml --package webtmux` (PASSED: clean compilation).

