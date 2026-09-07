---
phase: 05-pane-grid-workspace-operations
plan: "02"
subsystem: desktop-gpui
tags: [rust, gpui, tmux-geometry, divider-drag, context-menus, rename-dialogs, window-toolbar]

# Dependency graph
requires:
  - phase: 05-pane-grid-workspace-operations
    provides: 05-01 tracer grid (pane_geometry, correlated sends, kill gates, positioned grid with headers)
provides:
  - Throttled divider-drag state machine (40ms + FLIP, incremental pane.resize, grid-level streaming)
  - Pane context menu (9-row FE-verbatim, same-window swap submenu, kill-confirm routing, no join)
  - Rename Pane / Rename Window DLG1-clone dialogs with dialog-feedback correlated sends
  - Window toolbar (5 presets + Next, verbatim layout strings, inline error line)
  - Title-bar Plus button + per-chip context menus + window kill-confirm dialog
  - pane_ops headless suite (drag throttle/FLIP/no-viewport, swap scope, layout strings)
affects: [05-VALIDATION, 06-settings, 07-parity-audit]

actuals:
  tokens: 26800
  tasks: 3
  commits: 5

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure drag decision (drag_step_throttled over elapsed-ms) with Instant-backed AppState drag state; views pass window coords, AppState picks the axis"
    - "Grid-level mouse_move/mouse_up/mouse_up_out as the pointer-capture equivalent (div listeners cannot reach Window::capture_pointer)"
    - "Generic await_command_feedback for dialog submits (close-on-success, inline-error-on-failure) alongside the existing fire-and-record submits"
    - "True PopupMenu submenus for the swap picker with a disabled-item branch when empty"

key-files:
  created:
    - desktop-gpui/crates/webtmux/src/views/pane_context_menu.rs
    - desktop-gpui/crates/webtmux/src/views/rename_pane_dialog.rs
    - desktop-gpui/crates/webtmux/src/views/rename_window_dialog.rs
    - desktop-gpui/crates/webtmux/src/views/window_toolbar.rs
    - desktop-gpui/crates/webtmux/tests/pane_ops_test.rs
  modified:
    - desktop-gpui/crates/webtmux/src/app_state.rs
    - desktop-gpui/crates/webtmux/src/pane_geometry.rs
    - desktop-gpui/crates/webtmux/src/views/mod.rs
    - desktop-gpui/crates/webtmux/src/views/pane_grid.rs
    - desktop-gpui/crates/webtmux/src/views/pane_view.rs
    - desktop-gpui/crates/webtmux/src/views/tab_strip.rs

key-decisions:
  - "Drag arm sets last_sent=now (plan throttle contract) instead of FE lastSent:0, so the opening 40ms window holds"
  - "Pane/tab menu rows and labels follow FE sources verbatim where the plan shorthand differs (Rename Pane before Zoom; Break To Window; Rename Window/Break Active Pane/Kill Window; window kill button Close)"
  - "Swap picker is a true PopupMenu submenu (plan wording) rather than the FE dialog, with a disabled Swap item when empty"
  - "Toolbar renders the active session last_error inline as the dialog-free mutations' error surface"
  - "No pane.join anywhere except a D8-gap doc comment; no new packages; Cargo.toml/Cargo.lock untouched"

patterns-established:
  - "Pattern 1: Pure AppState query helpers (swap_candidates, pane_for_rename, window_name, active_window_id) tested headlessly; thin menu/dialog shells own sockets and awaits"
  - "Pattern 2: try_send_* public sync halves + await_command_feedback serve every dialog submit; header/toolbar/menu fire paths keep the existing submit_* methods"
  - "Pattern 3: Stable menu ids derived from entity ids (pane-menu/%N, window-tab/@N, pane-divider/key, window-layout/id, window-create); no counters or indices"

requirements-completed:
  - PANE-04
  - PANE-05
  - PANE-06
  - PANE-08
  - DLG-02

coverage:
  - id: T1
    description: "Divider drags throttle at ~40ms with FLIP, zero-step drops, and never emit terminal.resize or hello"
    requirement: PANE-04
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/pane_ops_test.rs#test_drag_throttle"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/pane_ops_test.rs#test_drag_flip"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/pane_ops_test.rs#test_drag_no_viewport_send"
        status: pass
    human_judgment: false
  - id: T2
    description: "Swap picker lists same-window panes only (excluding self) and toolbar maps six buttons to verbatim layout strings"
    requirement: PANE-05
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/pane_ops_test.rs#test_swap_picker_scope"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/pane_ops_test.rs#test_layout_strings"
        status: pass
    human_judgment: false
  - id: T3
    description: "Pane menu opens with 9 FE-verbatim rows, swap submenu, and kill-confirm routing; rename pane dialog submits"
    requirement: PANE-05
    verification:
      - kind: integration
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace"
        status: pass
    human_judgment: true
    rationale: "Menu open/click and dialog typing cannot render headlessly in GPUI — compile + scope tests prove wiring, a human must click through"
  - id: T4
    description: "Toolbar presets apply correlated window.layout; title-bar Plus creates windows; chip menus rename/move/break/kill with gates"
    requirement: PANE-06
    verification:
      - kind: integration
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace"
        status: pass
    human_judgment: true
    rationale: "Button/menu click behavior needs a live window — same GPUI headless limit as T3"
  - id: T5
    description: "Fast drags past the 4px handle keep streaming, menus survive poll re-renders, dialogs type cleanly, pixel parity holds vs Electron"
    verification: []
    human_judgment: true
    rationale: "Plan backstop criterion is explicitly manual-UAT class, recorded for the Phase-7 parity audit"

duration: 60min
completed: 2026-09-07
status: complete
---

# Phase 05 Plan 02: Workspace Operations Summary

**Divider drags with 40ms throttle + FLIP over grid-level streaming, FE-verbatim pane/window menus with same-window swap picker and kill gates, DLG1-clone Rename Pane/Window dialogs with feedback awaits, preset toolbar, and title-bar Plus + chip menus — all headless-locked plus full-suite green.**

## Performance

- **Duration:** ~60 min (cold GPUI compiles dominate; incremental targets run in ~45-90 s)
- **Started:** 2026-09-07T09:35:00+07:00
- **Completed:** 2026-09-07T10:35:00+07:00
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- Implemented the D3 drag state machine: `drag_step_throttled` pure helper (`pane_geometry.rs`) plus `PaneDragState` on `AppState` (`begin/poll/move/stream/end`), divider `mouse_down` arming with axis cell size, and grid-container `mouse_move`/`mouse_up`/`mouse_up_out` streaming so fast drags past the 4px handle keep resizing with col-resize/row-resize cursors and hover affordance.
- Rendered the 9-row pane menu FE-verbatim (Split Right / Split Down / Rename Pane / sep / Zoom / Swap picker / Break To Window / sep / Kill) with stable `pane-menu/%N` ids, a true submenu for same-window swap targets (disabled Swap when empty), and kill-confirm routing through `confirm_kill_pane`. `pane.join` stays absent with a D8-gap code comment.
- Cloned DLG1 for Rename Pane (`title || current_command || ''`) and Rename Window (`w.name`) with autofocus, Enter-submit, trim + non-empty gate, and a generic `await_command_feedback` (close on success, inline error with dialog open on failure, snapshot as sole truth).
- Rendered the h-8 toolbar (5 presets + separator + Next, stable `window-layout/*` ids) sending correlated `window.layout` with `@N` targets plus an inline `last_error` line; mounted above the grid in a flex column the canvas probe excludes from pane geometry.
- Extended the title bar in place: trailing Plus (`window.create`, no args), per-chip menus (Rename Window / Move Left −1 / Move Right +1 / Break Active Pane / sep / Kill Window), window kill-confirm dialog (Close button per FE), chip click still fire-and-forget `window.select`.
- Routed header Kill through the same pane gate (confirm dialog vs direct kill) and wrapped pane roots in the menu; all sends reuse the 05-01 correlated paths with receiver-dropped drag steps.
- Full workspace suite green with zero new packages and `Cargo.lock` untouched.

## Task Commits

Each task was committed atomically following TDD and conventional commit guidelines:

1. **Task 1 (TDD RED):** `e40f22a` (test(05-02): add failing drag throttle, FLIP, and no-viewport-send contracts)
2. **Task 1 (TDD GREEN):** `7837db7` (feat(05-02): implement divider drag state machine with 40ms throttle and FLIP)
3. **Task 2 (TDD RED):** `5e3add6` (test(05-02): add failing swap-picker scope and layout string contracts)
4. **Task 2 (TDD GREEN):** `0070d74` (feat(05-02): implement pane menu, rename dialog, and window toolbar presets)
5. **Task 3 (Window ops):** `8b723e4` (feat(05-02): extend title-bar tabs with Plus, chip menus, and window rename dialog)

## Files Created/Modified

- `desktop-gpui/crates/webtmux/src/pane_geometry.rs` - Added `DRAG_THROTTLE_MS` + pure `drag_step_throttled` (step==0 drop, 40ms gate without advancing, negative FLIP)
- `desktop-gpui/crates/webtmux/src/app_state.rs` - `PaneDragState` + begin/poll/move/stream/end; `SwapCandidate` + scope/label helpers; rename lookups; `active_window_id`; `try_send_*` + `await_command_feedback`; four dialog form slots
- `desktop-gpui/crates/webtmux/src/views/pane_grid.rs` - Divider handles with ids/cursors/mouse-down, grid-level move/up/up-out streaming, toolbar-above-grid flex column with probe measuring the inner container
- `desktop-gpui/crates/webtmux/src/views/pane_view.rs` - Stable `pane-menu/%N` root id, menu wrap, header Kill gate, window-passing action buttons
- `desktop-gpui/crates/webtmux/src/views/pane_context_menu.rs` - 9-row FE-verbatim menu + swap submenu + `request_kill_pane` + `KillPaneForm` dialog
- `desktop-gpui/crates/webtmux/src/views/rename_pane_dialog.rs` - DLG1-clone Rename Pane dialog with feedback submit
- `desktop-gpui/crates/webtmux/src/views/rename_window_dialog.rs` - DLG1-clone Rename Window dialog with feedback submit
- `desktop-gpui/crates/webtmux/src/views/window_toolbar.rs` - Preset constants + h-8 bar + inline error line
- `desktop-gpui/crates/webtmux/src/views/tab_strip.rs` - Plus button, chip menu wrap, `KillWindowForm` dialog + submits
- `desktop-gpui/crates/webtmux/src/views/mod.rs` - Registered four new modules
- `desktop-gpui/crates/webtmux/tests/pane_ops_test.rs` - Five headless tests (drag ×3, swap scope, layout strings)

## Decisions Made

- Drag arm records `last_sent = now` (plan throttle contract) rather than FE `lastSent: 0`, so the opening 40ms window holds and the first inside-window move sends nothing without advancing `last_cells`.
- Menu rows/labels follow the FE sources verbatim where the plan shorthand differs (Rename Pane sits before Zoom per `PaneContextMenu.tsx`; `Break To Window`; `Rename Window` / `Break Active Pane` / `Kill Window` per `WindowContextMenu.tsx`; window kill button `Close` per `WindowTabs.tsx`) — the plan's own "do not reorder, rename, or improve" rule points at FE as the authority.
- Swap picker is a true `PopupMenu` submenu (the plan's literal wording) instead of the FE dialog; empty candidates render a disabled Swap item.
- Grid-level move/up listeners are the pointer-capture equivalent: `Window::capture_pointer` needs a `HitboxId` no div listener receives, while the full-workspace container hitbox keeps moves firing anywhere inside the window (research A2 answered + noted).
- One generic `await_command_feedback` serves all four dialog submits instead of per-dialog await clones; fire paths keep the existing `submit_*` methods (session `last_error` convention).
- Toolbar shows the active session `last_error` inline so dialog-free preset applies still surface `command.error`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected the swap-picker label expectation in the new test**
- **Found during:** Task 2 (GREEN verification)
- **Issue:** Test asserted `%0`'s picker label is `"editor"` (title), but the FE-verbatim picker order is `currentCommand || title || currentPath`, so `"nvim"` is correct.
- **Fix:** Fixed the test expectation and comment; implementation already matched FE.
- **Files modified:** `desktop-gpui/crates/webtmux/tests/pane_ops_test.rs`
- **Verification:** `cargo test -p webtmux --test pane_ops_test` — 5 passed.
- **Committed in:** `0070d74`

**2. [Rule 3 - Blocker] Moved SwapCandidate to module scope**
- **Found during:** Task 2 (GREEN compile)
- **Issue:** Struct defined inside `impl AppState` — Rust forbids item definitions in impl blocks.
- **Fix:** Moved `SwapCandidate` to module level next to `WindowTab`/`KillRoute`.
- **Files modified:** `desktop-gpui/crates/webtmux/src/app_state.rs`
- **Verification:** Full `pane_ops_test` target compiles warning-free.
- **Committed in:** `0070d74`

**3. [Rule 3 - Blocker] Removed the unused await session param, prefixed an unused receiver**
- **Found during:** Task 2 (GREEN compile)
- **Issue:** `await_command_feedback` took a `session` it never consumed (callers keep their own clone); kill closure's `this` unused — both break the warning-clean bar.
- **Fix:** Dropped the param (two call sites updated), renamed to `_this`.
- **Files modified:** `desktop-gpui/crates/webtmux/src/app_state.rs`, `desktop-gpui/crates/webtmux/src/views/rename_pane_dialog.rs`, `desktop-gpui/crates/webtmux/src/views/pane_context_menu.rs`, `desktop-gpui/crates/webtmux/src/views/pane_view.rs`
- **Verification:** `cargo build -p webtmux --tests` warning-free.
- **Committed in:** `0070d74`

**4. [Rule 3 - Blocker] Deferred window dialog form slots to Task 3**
- **Found during:** Task 2 (GREEN scoping)
- **Issue:** `app_state.rs` fields referencing `rename_window_dialog`/`tab_strip::KillWindowForm` would force Task-3 files into the Task-2 commit for it to compile.
- **Fix:** Task 2 ships pane slots only; Task 3 adds the window slots with its dialog and strip changes.
- **Files modified:** `desktop-gpui/crates/webtmux/src/app_state.rs`
- **Verification:** Task-2 commit builds and tests green standalone.
- **Committed in:** `0070d74` (deferral), `8b723e4` (addition)

**5. [Rule 3 - Blocker] Mounted the toolbar in pane_grid.rs though unlisted**
- **Found during:** Task 2 (GREEN)
- **Issue:** Task-2 file list names no host for "mount the toolbar above the grid" (`session_states.rs` owns the host, also unlisted).
- **Fix:** `render_pane_grid` returns toolbar-above-grid column; probe measures the inner container so toolbar px never enter geometry.
- **Files modified:** `desktop-gpui/crates/webtmux/src/views/pane_grid.rs`
- **Verification:** Workspace suite green; geometry math untouched (inner size converges like before).
- **Committed in:** `0070d74`

**6. [Rule 3 - Blocker] Scoped cargo invocations per target at JOBS=2**
- **Found during:** Task 1 verification
- **Issue:** Same 05-01 environment limits: full-workspace builds risk paging-file exhaustion above JOBS=2; cargo 1.96 takes one test-name filter per invocation.
- **Fix:** Verified per target (`--test pane_ops_test`, `--tests` builds) under `CARGO_TARGET_DIR=C:\cargo-target\web-tmux` at `CARGO_BUILD_JOBS=2`, then the full `--workspace` gate the same way.
- **Files modified:** None (verification procedure only)
- **Verification:** Every target plus the workspace gate, all green.

---

**Total deviations:** 6 auto-fixed (1 bug, 5 blockers)
**Impact on plan:** None on scope or behavior. Fixes were required for compilation against Rust/GPUI APIs, warning-clean builds, commit atomicity, and the Windows build environment; menu-order calls follow the plan's own FE-verbatim rule.

## Issues Encountered

- `Instant + Duration` arithmetic and `saturating_duration_since` cover the poll clock headlessly — no mock clock needed since `begin_pane_drag_at`/`poll_pane_drag` take explicit `Instant`s.
- `Context<AppState>` derefs to `App`, so dialog open functions keep their `&mut App` signatures at every call site (listeners, menus, subscriptions) with zero adaptation code.
- The Go `service_integration_test.go` LSP diagnostic (`svc.CapturePane` arity) is pre-existing backend drift, untouched and out of scope.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 5 is now complete on this wave: drags, pane menus, presets, window-tab ops, and both rename dialogs all ride correlated sends with gates; remaining Phase-5 surface (if any) is Phase-6 settings toggles for `confirm_kill_pane/window` and the Phase-7 parity audit (backstop UAT: drag feel, fast-drag capture, menu poll-tick survival, dialog typing, pixel parity vs Electron — see coverage T5).
- Known limits for the audit: drags stall if the pointer leaves the OS window mid-drag (no capture outside the window); toolbar/preset buttons carry stable ids + hover affordance but no hover tooltips (05-01 D10/A3 finding stands).

---
*Phase: 05-pane-grid-workspace-operations*
*Completed: 2026-09-07*

## Self-Check: PASSED
- All 11 created/modified files exist on disk.
- All 5 task commits verified in git log (`e40f22a`, `7837db7`, `5e3add6`, `0070d74`, `8b723e4`).
- Full workspace suite green with zero failures across every target; lib + tests warning-free.
- No stub patterns in new code (only legit InputState `"New name"` placeholders); no `pane.join` surface (one D8-gap doc comment); drag path builds only `pane.resize`; zero new packages with `Cargo.toml`/`Cargo.lock` untouched.
