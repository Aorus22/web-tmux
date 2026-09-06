---
phase: 04-terminal-engine-live-rendering
plan: "02"
subsystem: desktop-gpui
tags: [rust, gpui, wheel, clipboard, resize, websocket, interop]
requires:
  - phase: 04-terminal-engine-live-rendering
    provides: tracer engine store and single-path view from 04-01
provides:
  - FE-parity wheel policy (SGR vs PageUp/PageDown vs scrollback) on TerminalView
  - Mouse selection plus clipboard copy/paste depth with key-routing lock
  - Debounced window-level terminal.resize with layout-key 150/325ms resync
  - Mock-server terminal interop proving capture live resize over a real socket
affects: [05-pane-grid-workspace-operations]
tech-stack:
  added: []
  patterns:
    - "Pure WheelAction decision (mode bits plus tui flag plus pixel delta) with per-pane px accumulator, burst clamp 3"
    - "Headless KeyRoute helper mirroring the GPUI on_key_down checks"
    - "Generation-tagged 100ms debounce with fire-time actualViewport scaling and max(2)/max(1) clamp"
    - "Layout-key tracker with first-mount skip and 150/325ms invalidate-and-recapture pair"
    - "Scripted mock WS terminal frames from a shared JSON fixture with no hello on the wire"
key-files:
  created:
    - desktop-gpui/crates/backend-client/tests/fixtures/terminal_ws_frames.json
  modified:
    - desktop-gpui/crates/webtmux/src/app_state.rs
    - desktop-gpui/crates/webtmux/src/views/terminal_view.rs
    - desktop-gpui/crates/webtmux/tests/terminal_test.rs
    - desktop-gpui/crates/backend-client/tests/ws_test.rs
key-decisions:
  - "GPUI positive scroll delta means wheel-up (reference view parity): positive pages up via PageUp, negative via PageDown — mirrored vs the DOM text but semantically identical"
  - "TUI accumulator keeps the FE remainder semantics (accum minus full notches times 100, clamp applies to emitted pages only)"
  - "Debounce supersede uses a dedicated resize_seq counter; PendingViewport.generation stays the session generation for guard layering"
  - "Layout-key observation lives in TerminalView render (session_states is outside the plan file scope); dedupe via last_layout_key keeps per-pane renders to one schedule per change"
  - "Interop resize send waits 500ms before disconnect so the write pump flushes the queued frame"
requirements-completed:
  - TERM-04
  - TERM-05
  - TERM-06
coverage:
  - id: E1
    description: "Wheel pages TUI panes with PageUp/PageDown by default, scrolls scrollback otherwise, reports SGR under mouse mode"
    requirement: TERM-04
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/terminal_test.rs#test_wheel_policy"
        status: pass
    human_judgment: false
  - id: E2
    description: "Mouse selection copies to the clipboard; clipboard pastes into the terminal as one terminal.input message"
    requirement: TERM-05
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/terminal_test.rs#test_selection_text"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/terminal_test.rs#test_copy_paste_keys"
        status: pass
    human_judgment: false
  - id: E3
    description: "Resizes ride one debounced window-level terminal.resize with clamped dimensions; layout changes resync via the 150/325ms pair; paint never sends; no hello"
    requirement: TERM-06
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/terminal_test.rs#test_resize_dance"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/terminal_test.rs#test_layout_key_resync"
        status: pass
    human_judgment: false
  - id: E4
    description: "Mock-server interop proves capture to live to resize over a real socket; reconnect re-captures the session scope only"
    requirement: TERM-06
    verification:
      - kind: integration
        ref: "desktop-gpui/crates/backend-client/tests/ws_test.rs#test_terminal_frames"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/terminal_test.rs#test_hidden_session_recapture"
        status: pass
      - kind: other
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace (single final invocation, all suites green)"
        status: pass
    human_judgment: false
actuals:
  tokens: 19500
  tasks: 3
  commits: 5
duration: 90min
completed: 2026-09-06
status: complete
---

# Phase 04 Plan 02: Terminal Expansion Summary

**FE-parity wheel paging with burst-clamped notch accumulation, full mouse-selection plus clipboard copy/paste routing, and the storm-free 100ms debounce plus 150/325ms layout resync — closed by mock-server capture-live-resize interop with the full workspace battery green.**

## Performance

- **Duration:** 90 min (dominated by cold-compile filtered verifies under the C: target dir)
- **Started:** 2026-09-06T18:23:07Z
- **Completed:** 2026-09-06
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- Implemented the pure `WheelAction` decision plus `AppState::apply_wheel` emit (SGR/Page bytes over `terminal.input` on the owning socket, scrollback via `scroll_display`) with a per-pane px accumulator (`WHEEL_NOTCH_PX=100`, line x16, burst clamp 3), and rewired `TerminalView::on_scroll` to it.
- Completed selection depth: drag geometry via the ported `mouse.rs` helpers was already in the tracer; this plan added the headless `pane_selection_text` lock, the `KeyRoute` helper mirroring the view checks, and the `Ctrl+C`-empty fallthrough to `\x03` test lock. Copy feeds `selection_to_string` into the system clipboard; paste sends one `terminal.input` message (server batcher owns chunking; no client chunking).
- Implemented the resize state machine: `arm_viewport_for_pane` (zero-size never arms, dedupe, bumps `resize_seq`), `take_debounced_resize` (stale seq drops, firing consumes pending and recomputes `actualViewport` at fire time with `max(2)/max(1)` clamp), `build_terminal_resize` (no hello), `compute_layout_key`/`current_layout_key`/`decide_layout_resync` (first-mount skip, identical-key no-op), `layout_resync_panes` (registered-only targets), plus the 100ms and 150/325ms timer schedulers. The paint path stays arm-only with a container-zero early return.
- Drove mock-server interop from `terminal_ws_frames.json`: `connection.ready` plus `state.snapshot` bootstrap, `terminal.capture` for the pane, `terminal.snapshot{replace:true, screenRows}` reply, `terminal.output{replace:false}` chunks plus one `replace:true` frame, then a debounced clamped `terminal.resize` — asserting capture present, resize shape, grid substrings end to end, and hello absent.
- Locked reconnect scoping: `recapture_session` re-arms plus re-captures panes of that session only; other sessions keep their exactly-once gates shut.

## Task Commits

Each task was committed atomically following TDD and conventional commit guidelines:

1. **Task 1 (TDD RED):** `0a5e279` (test(04-02): add failing wheel selection clipboard tests)
2. **Task 1 (TDD GREEN):** `2ad07c9` (feat(04-02): add FE-parity wheel policy plus selection clipboard depth)
3. **Task 2 (TDD RED):** `f6294cf` (test(04-02): add failing resize dance and layout resync tests)
4. **Task 2 (TDD GREEN):** `bae1ab7` (feat(04-02): add debounced window resize with layout-key resync)
5. **Task 3 (Interop):** `25b81ce` (feat(04-02): add mock-server terminal interop plus recapture hardening)

## Files Created/Modified

- `desktop-gpui/crates/webtmux/src/app_state.rs` - WheelAction/WheelDelta/decide helpers, KeyRoute helper, pane_selection_text, apply_wheel, resize_seq/wheel_accum/last_layout_key fields, arm/take/scale/send/schedule helpers, layout-key tracker and resync pair
- `desktop-gpui/crates/webtmux/src/views/terminal_view.rs` - on_key_down via decide_key_route (behavior preserved), on_scroll via apply_wheel, paint zero-size early return, arm-only resize path via arm plus schedule, layout-key observer hook in render
- `desktop-gpui/crates/webtmux/tests/terminal_test.rs` - test_wheel_policy, test_selection_text, test_copy_paste_keys, test_resize_dance, test_layout_key_resync, test_hidden_session_recapture
- `desktop-gpui/crates/backend-client/tests/ws_test.rs` - test_terminal_frames interop over a real socket
- `desktop-gpui/crates/backend-client/tests/fixtures/terminal_ws_frames.json` - Scripted snapshot blob plus output chunks plus replace frame plus expected grid text plus resize dims

## Decisions Made

- GPUI positive scroll delta means wheel-up per the reference view (positive drives SGR 64 and scroll-up): the TUI mapping is positive to PageUp, negative to PageDown — text-mirrored vs the DOM source but semantically identical, with the inversion documented at the decision site.
- The notch remainder subtracts full notches (not clamped pages) times 100, exactly like the FE (`wheelAccum -= notches * NOTCH` before clamping), so a 110px batch leaves 10px.
- Debounce supersede uses a dedicated `resize_seq`; the pending viewport keeps the session generation for triple-guard layering rather than overloading one counter.
- The layout-key observer hooks into `TerminalView::render` because `session_states.rs` is outside the plan file scope; the `last_layout_key` dedupe makes per-pane renders collapse to one schedule per key change.
- The interop test sleeps 500ms between the resize send and disconnect so the split-pump write task flushes the queued frame before teardown closes it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] RED wheel test assumed DOM sign for TUI paging**
- **Found during:** Task 1 GREEN verification (`test_wheel_policy` failed with PageUp vs PageDown mismatch)
- **Issue:** The RED test expected GPUI-positive to page down, contradicting its own SGR expectations (positive to SGR 64 wheel-up) and the reference view sign.
- **Fix:** Aligned the TUI expectations to positive-means-up (positive to PageUp, negative to PageDown) with an explanatory comment; implementation unchanged in spirit, test now internally consistent.
- **Files modified:** `desktop-gpui/crates/webtmux/tests/terminal_test.rs`
- **Commit:** `2ad07c9`

**2. [Rule 1 - Bug] RED accum assertion expected zero leftover on a 110px batch**
- **Found during:** Task 1 GREEN verification (`accum2.abs() < 1.0` failed)
- **Issue:** 40px plus 70px is 110px: one notch emits and 10px must remain per the FE remainder rule.
- **Fix:** Assert leftover near 10px instead of near zero.
- **Files modified:** `desktop-gpui/crates/webtmux/tests/terminal_test.rs`
- **Commit:** `2ad07c9`

**3. [Rule 1 - Bug] Interop resize never reached the mock server**
- **Found during:** Task 3 verification (`test_terminal_frames` failed with server never saw resize)
- **Issue:** `handle.disconnect()` sets closed and queues shutdown immediately after the resize send, so the write pump dropped the queued resize frame before flushing it.
- **Fix:** Sleep 500ms between the resize send and disconnect, letting the pump flush; server window already allowed 5s.
- **Files modified:** `desktop-gpui/crates/backend-client/tests/ws_test.rs`
- **Commit:** `25b81ce`

---

**Total deviations:** 3 auto-fixed (3 bugs)
**Impact on plan:** None. No architectural changes, no new crates, no protocol changes. Scope stayed exactly on plan files plus this summary.

## Issues Encountered

- The first full-workspace invocation exceeded the 120s tool timeout with no output and was retried with a 600s timeout, which completed green. No code impact; follow-up runs should keep the long timeout for the cold workspace battery.
- The pre-existing Go backend LSP error (`be/test/tmux/service_integration_test.go:147` CapturePane arity) was observed again and remains out of scope — backend protocol is unchanged by this plan.

## User Setup Required

None - no external service configuration required. Note for follow-up builds: reuse `CARGO_TARGET_DIR=C:\cargo-target\web-tmux` with `CARGO_BUILD_JOBS=4` (E: drive is full; parallel metadata corruption was avoided by one invocation per target plus a single final workspace run).

## Next Phase Readiness

- Phase 4 is now complete on top of the 04-01 tracer: wheel, selection/clipboard, debounced resize plus layout resync, and socket interop are all proven with the workspace green.
- Phase 5 replaces the stacked tracer layout with tmux-geometry rendering using the same store entries; the pending-viewport and layout-key machinery transfers unchanged.
- Timer bodies (100/150/325ms) are manual-UAT class like the 02-02/03-02 visual checks: drag the window and confirm Electron panes do not rewrap mid-drag and settle once; change layout and confirm the quick resize plus scrollback resync.

---
*Phase: 04-terminal-engine-live-rendering*
*Completed: 2026-09-06*

## Self-Check: PASSED
- All plan files plus the new fixture exist on disk.
- All 5 task commits verified in git log (0a5e279, 2ad07c9, f6294cf, bae1ab7, 25b81ce).
- Targeted suites green before the gate (wheel/selection/copy, resize/layout, recapture, terminal interop); final single-invocation full workspace battery exited 0 with the terminal engine suites green in the tail output.
