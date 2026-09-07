---
phase: 05-pane-grid-workspace-operations
plan: "01"
subsystem: desktop-gpui
tags: [rust, gpui, tmux-geometry, workspace, panes, websocket, correlated-sends]

# Dependency graph
requires:
  - phase: 04-terminal-engine-live-rendering
    provides: Arc-shared store-owned Terminals with dumb TerminalViews, layout-key 150/325ms resync, TUI map, title store
provides:
  - Pure headless-tested pane_geometry module (pixel_rect, close_pane_gaps, px_to_cells, resize_drag_step, FLIP, divider_layout over i64)
  - Correlated pane/window mutation sends with 10s await plus fire-and-forget selects and drag steps on owning sockets
  - Positioned pane grid with FE-parity headers re-hosting Phase-4 TerminalViews plus split/zoom wiring
  - Wave-0 coverage (geometry, window_state clamps, WS envelopes, kill gates, rename prefill)
affects: [05-02-PLAN]

actuals:
  tokens: 19900
  tasks: 3
  commits: 6

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Verbatim geometry.ts port as pure i64 module with usize-to-i64 conversion at the snapshot boundary"
    - "Correlated pane/window submits cloned from the Phase-3 submit_rename shape with owning-socket resolve and 10s forget-timeout"
    - "Grid measures itself with a full-size canvas probe into AppState::workspace_size (800x600 before first measure)"
    - "Pure envelope constructors plus per-flag kill gates kept headless-testable"

key-files:
  created:
    - desktop-gpui/crates/webtmux/src/pane_geometry.rs
    - desktop-gpui/crates/webtmux/src/views/pane_grid.rs
    - desktop-gpui/crates/webtmux/src/views/pane_view.rs
    - desktop-gpui/crates/webtmux/tests/pane_geometry_test.rs
    - desktop-gpui/crates/webtmux/tests/window_state_test.rs
    - desktop-gpui/crates/backend-client/tests/ws_pane_window_test.rs
  modified:
    - desktop-gpui/crates/webtmux/src/lib.rs
    - desktop-gpui/crates/webtmux/src/app_state.rs
    - desktop-gpui/crates/webtmux/src/icons.rs
    - desktop-gpui/crates/webtmux/src/views/mod.rs
    - desktop-gpui/crates/webtmux/src/views/session_states.rs
    - desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs

key-decisions:
  - "Port geometry.ts verbatim over i64 with cell_pane boundary conversion so p.left - 1 never underflows usize in debug"
  - "Split right encodes horizontal and Split down encodes vertical, locking the PANE-02 naming trap headlessly"
  - "Zoom stays a server toggle with zero client zoom state; the grid filters snapshot panes and hides dividers plus the zoom button for single-pane windows"
  - "Raw-div header actions carry stable ids plus hover affordance after verifying no tooltip API exists for raw divs in gpui-pre 0.3.3"

patterns-established:
  - "Pattern 1: Pure AppState envelope constructors (build_pane_*/build_window_*) tested headlessly, thin send/submit shells own the socket and the 10s await"
  - "Pattern 2: Grid render splits into a mutable ensure phase (prune, pane_entry, TerminalView retention, capture) and a shared render phase (positioned panes plus divider bars)"
  - "Pattern 3: Canvas probe converges workspace size in one extra frame by notifying only on change"

requirements-completed:
  - PANE-01
  - PANE-02
  - PANE-03
  - PANE-07

coverage:
  - id: T1
    description: "pane_geometry scale, gap absorption, divider adjacency, drag steps, and FLIP match geometry.ts"
    requirement: PANE-01
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/pane_geometry_test.rs#test_pixel_rect_scale"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/pane_geometry_test.rs#test_close_pane_gaps"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/pane_geometry_test.rs#test_divider_adjacency"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/pane_geometry_test.rs#test_drag_step_incremental"
        status: pass
    human_judgment: false
  - id: T2
    description: "window_state clamp gap from STATE.md closed via pure restore paths"
    requirement: PANE-01
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/window_state_test.rs#test_window_state_clamps"
        status: pass
    human_judgment: false
  - id: T3
    description: "All 15 pane/window WS envelopes locked with @N window targets and six verbatim layout strings, and no pane.join exists"
    requirement: PANE-02
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/backend-client/tests/ws_pane_window_test.rs#test_ws_pane_window_envelopes"
        status: pass
    human_judgment: false
  - id: T4
    description: "Pane/window kill gates read only their own flag, split directions map right-to-horizontal, and rename prefill plus gate match FE sources"
    requirement: PANE-02
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs#test_pane_kill_gate"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs#test_window_kill_gate"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs#test_split_direction_map"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs#test_rename_prefill"
        status: pass
    human_judgment: false
  - id: T5
    description: "Positioned grid renders live terminals with FE-parity headers while split right/down plus zoom round-trip through correlated sends"
    requirement: PANE-03
    verification:
      - kind: integration
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace"
        status: pass
      - kind: other
        ref: "cargo build --manifest-path desktop-gpui/Cargo.toml --package webtmux"
        status: pass
    human_judgment: false

duration: 150min
completed: 2026-09-07
status: complete
---

# Phase 05 Plan 01: Pane Grid Tracer Summary

**Pure `pane_geometry` module with headless contracts, correlated pane/window mutation sends with per-flag kill gates, and the positioned workspace grid with FE-parity headers re-hosting Phase-4 TerminalViews plus split/zoom round-trip.**

## Performance

- **Duration:** ~150 min (cold GPUI builds dominate; incremental test targets run in ~15-90 s)
- **Started:** 2026-09-07T01:50:36Z
- **Completed:** 2026-09-07
- **Tasks:** 3
- **Files modified:** 12

## Accomplishments

- Implemented `pane_geometry.rs` as a verbatim `geometry.ts` port over `i64` cells (`pixel_rect`, `close_pane_gaps`, `px_to_cells`, `resize_drag_step`, `flip_direction`, `divider_layout`, `CELL_W = 8`, `CELL_H = 18`) with zero GPUI imports and the `ww <= 0 || wh <= 0` zero-rect guard.
- Closed the STATE.md `window_state` clamp gap with `window_state_test.rs` (800..3840, 500..2160, -10000 sentinel to default origin, zero/missing to defaults, maximized preservation).
- Locked all 15 pane/window WS envelopes with `ws_pane_window_test.rs` (split direction strings, `paneId`-carried `@N` targets, six verbatim layouts including `next-layout`, move offsets, and an explicit no-`pane.join` assertion per D8).
- Added every Task-2 `AppState` send: correlated `submit_pane_split/zoom/kill/break/swap/rename` plus `submit_window_layout/rename/kill/move/break-active/create` with 10s await and inline `last_error`, alongside fire-and-forget `send_pane_select` and drag-step `submit_pane_resize` (receiver dropped, `amount == 0` dropped, never `hello`, never `terminal.resize`).
- Added per-flag `kill_requires_confirm_pane/window` gates, the `pane_split_direction` pitfall lock, pure rename prefill plus trim/non-empty gate, `set_tui_scroll`, and the 9 D10 lucide icons at `stroke-width="1.5"`.
- Rendered the tracer grid: active-window filter, zoomed-pane full-size render with dividers hidden, `close_pane_gaps` then `pixel_rect` per pane, absolutely positioned panes, Task-1-tested divider bars (visual only, drag arrives in 05-02), canvas probe into `workspace_size` (800x600 before first measure), and h-7 headers (path, mono id, TUI switch default ON, Split right/down, Zoom hidden single-pane, Kill, double-click zoom, active border, inactive-click select) re-hosting Phase-4 `TerminalView`s with store ownership untouched.
- Full workspace suite green with zero new packages and `Cargo.lock` untouched.

## Task Commits

Each task was committed atomically following TDD and conventional commit guidelines:

1. **Task 1 (TDD RED):** `9a4c54c` (test(05-01): add failing headless contracts for geometry, window clamps, WS envelopes)
2. **Task 1 (TDD GREEN):** `c712df7` (feat(05-01): implement pure pane_geometry module with headless contracts)
3. **Task 2 (TDD RED):** `cf6cf23` (test(05-01): add failing pane/window gates, split map, rename prefill)
4. **Task 2 (TDD GREEN):** `3240942` (feat(05-01): add correlated pane/window sends plus kill gates and icons)
5. **Task 3 (Tracer Slice):** `ca0da69` (feat(05-01): render positioned pane grid with headers re-hosting TerminalViews)

## Files Created/Modified

- `desktop-gpui/crates/webtmux/src/pane_geometry.rs` - Pure verbatim geometry port over `i64` with unit tests inline
- `desktop-gpui/crates/webtmux/src/views/pane_grid.rs` - Workspace grid (zoom filter, positioned panes, divider bars, canvas probe)
- `desktop-gpui/crates/webtmux/src/views/pane_view.rs` - Positioned pane with h-7 header plus re-hosted `TerminalView`
- `desktop-gpui/crates/webtmux/tests/pane_geometry_test.rs` - Headless geometry contracts (scale, gaps, dividers, drag steps, FLIP)
- `desktop-gpui/crates/webtmux/tests/window_state_test.rs` - STATE.md clamp-guard coverage via pure `restore`
- `desktop-gpui/crates/backend-client/tests/ws_pane_window_test.rs` - Envelope JSON shapes for all 15 commands plus no-join lock
- `desktop-gpui/crates/webtmux/src/lib.rs` - Registered `pane_geometry` module
- `desktop-gpui/crates/webtmux/src/app_state.rs` - Pane/window sends, gates, prefill helpers, TUI setter, `workspace_size`
- `desktop-gpui/crates/webtmux/src/icons.rs` - Nine Phase-5 lucide constants
- `desktop-gpui/crates/webtmux/src/views/mod.rs` - Registered `pane_grid` and `pane_view`
- `desktop-gpui/crates/webtmux/src/views/session_states.rs` - Workspace body now renders the geometry grid
- `desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs` - Extended with pane/window gates, split map, rename prefill

## Decisions Made

- Used `i64` internals with a `cell_pane` boundary converter so `p.left - 1` never underflows `usize` in debug builds.
- Resolved the acceptance ambiguity toward research D5: split/zoom/kill and all window mutations await correlated replies while selects and drag steps stay fire-and-forget.
- Measured the grid container with a full-size canvas probe because GPUI has no ResizeObserver; 800x600 covers the first frame before measurement converges.
- Kept the Phase-4 layout-key resync untouched since `TerminalView::render` already observes and schedules it with first-mount skip.
- Left the header Kill as a tracer direct submit because confirm dialogs belong to the 05-02 menu wave that owns kill-confirm routing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] Added the Fullscreen arm to window_state matches**
- **Found during:** Task 1 (GREEN verification)
- **Issue:** `gpui-pre =0.3.3` defines a third `WindowBounds::Fullscreen` variant, so exhaustive matches in the new test failed to compile.
- **Fix:** Matched `Windowed | Maximized | Fullscreen` for size and origin extraction; `restore` never returns Fullscreen, so behavior is unchanged.
- **Files modified:** `desktop-gpui/crates/webtmux/tests/window_state_test.rs`
- **Verification:** `cargo test --manifest-path desktop-gpui/Cargo.toml -p webtmux --test window_state_test` passes.
- **Committed in:** `c712df7`

**2. [Rule 1 - Bug] Fixed unused-variable warnings blocking a warning-clean build**
- **Found during:** Task 1 and Task 3 verification
- **Issue:** Two destructured geometry bindings and one header color were unused.
- **Fix:** Underscore-prefixed the dead bindings and removed the dead color; `header_action_button` carries its own hover literal.
- **Files modified:** `desktop-gpui/crates/webtmux/src/pane_geometry.rs`, `desktop-gpui/crates/webtmux/src/views/pane_view.rs`
- **Verification:** `cargo build --manifest-path desktop-gpui/Cargo.toml -p webtmux` finishes warning-free for the touched modules.
- **Committed in:** `c712df7`, `ca0da69`

**3. [Rule 3 - Blocker] Refactored the header button closure into a free function**
- **Found during:** Task 3 (first `cargo test -p webtmux --lib`)
- **Issue:** A closure capturing `cx` for the four header buttons held an immutable borrow across later mutable `cx` uses (E0502).
- **Fix:** Replaced the closure with `header_action_button`, a free function taking `cx` per call so every invocation reborrows independently.
- **Files modified:** `desktop-gpui/crates/webtmux/src/views/pane_view.rs`
- **Verification:** Full workspace suite green.
- **Committed in:** `ca0da69`

**4. [Rule 3 - Blocker] Scoped cargo invocations per target with reduced parallelism**
- **Found during:** Task 1 verification
- **Issue:** `cargo test -p webtmux pane_geometry` builds every target including the GPUI binary, which exhausted the paging file at `CARGO_BUILD_JOBS=4` (mmap failure on `libgpui`); cargo 1.96 also accepts a single test-name filter per invocation.
- **Fix:** Verified with `--lib` plus one `--test` target per command at `CARGO_BUILD_JOBS=2` under `CARGO_TARGET_DIR=C:\cargo-target\web-tmux`, then ran the final full-workspace gate the same way.
- **Files modified:** None (verification procedure only)
- **Verification:** Every target below plus the workspace gate, all with zero failures.
- **Committed in:** N/A

---

**Total deviations:** 4 auto-fixed (1 bug, 3 blockers)
**Impact on plan:** None. Fixes were required for compilation against the pinned GPUI bindings and for the Windows build environment; no behavior or scope changed.

## Issues Encountered

- Hover tooltips could not ride raw GPUI divs: `gpui-pre =0.3.3` exposes no tooltip API on plain elements and `gpui-component =0.6.0` keeps its tooltip applicator crate-internal (verified in the vendored `src/tooltip.rs` plus the absent `group_hover` in `gpui-pre`). Header actions therefore expose stable ids (`pane-split-right/%N`, `pane-split-down/%N`, `pane-zoom/%N`, `pane-kill/%N`, `pane-tui/%N`) with hover affordance; true hover tooltips ride the 05-02 menu wave and Phase-7 parity audit. This exercises the plan's own D10/A3 fallback branch.
- Pixel parity versus Electron and drag feel under fast drags stay manual-only (GPUI is not headless-renderable, Ph2/Ph4 precedent) and are deferred to the Phase-7 parity audit with the backstop criterion recorded in the plan must-haves.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The 05-02 wave builds directly on this tracer: divider drags consume `resize_drag_step` plus `submit_pane_resize`, menus consume the `submit_pane_*`/`submit_window_*` sends plus kill gates, and rename dialogs consume `pane_rename_prefill`/`window_rename_prefill` plus `rename_name_allowed`.
- Known tracer boundaries for 05-02: divider bars have no mouse handlers yet, header Kill has no confirm dialog yet, and header/toolbar hover tooltips are still pending.

---
*Phase: 05-pane-grid-workspace-operations*
*Completed: 2026-09-07*

## Self-Check: PASSED
- All 12 created/modified files exist on disk.
- All 5 task commits verified in git log.
- Full workspace suite green with zero failures across every target.
- No stub patterns in new view/geometry code; no `pane.join`; no `terminal.resize` or `hello` from pane layout code; `pane_geometry.rs` holds no GPUI imports.
