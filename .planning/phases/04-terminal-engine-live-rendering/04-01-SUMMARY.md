---
phase: 04-terminal-engine-live-rendering
plan: "01"
subsystem: desktop-gpui
tags: [rust, gpui, alacritty-terminal, terminal, capture-replay, websocket, tracer]

# Dependency graph
requires:
  - phase: 03-websocket-multi-session-tabs
    provides: per-session WS tabs, triple generation guard, forward pump for all sessions
provides:
  - Tested webtmux-terminal engine crate (Term + Processor, keystroke table, capture replay)
  - Pane-id-keyed TerminalStore on AppState with guarded terminal commit arms
  - TerminalView rendering one live terminal per pane of the active window
affects: [04-02-PLAN, 04-03-PLAN, 05-pane-grid-workspace-operations]

actuals:
  tokens: 33164
  tasks: 3
  commits: 5

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Byte-oriented apply_capture port (history-delta + clear/home/positioned screen, legacy branch, shrank-history reset, CRLF normalization)"
    - "Pane-id-keyed TerminalStore with snapshotWritten exactly-once gate and ingestedHistory delta counter"
    - "Owning-socket fire-and-forget terminal.input/capture sends (receiver dropped, uncorrelated)"
    - "Shared Arc<Mutex<Terminal>> store ownership with dumb views holding clones for paint/input"
    - "D7 pane-session attribution with reset-on-reaffiliation for tmux %N reuse"

key-files:
  created:
    - desktop-gpui/crates/terminal/src/terminal.rs
    - desktop-gpui/crates/terminal/src/input.rs
    - desktop-gpui/crates/terminal/src/mouse.rs
    - desktop-gpui/crates/terminal/src/render.rs
    - desktop-gpui/crates/terminal/src/event.rs
    - desktop-gpui/crates/terminal/src/colors.rs
    - desktop-gpui/crates/terminal/src/capture.rs
    - desktop-gpui/crates/terminal/tests/capture_test.rs
    - desktop-gpui/crates/terminal/tests/input_test.rs
    - desktop-gpui/crates/terminal/tests/fixtures/terminal_capture_vim.json
    - desktop-gpui/crates/terminal/tests/fixtures/terminal_capture_unicode.json
    - desktop-gpui/crates/terminal/tests/fixtures/terminal_output_frames.json
    - desktop-gpui/crates/webtmux/src/views/terminal_view.rs
    - desktop-gpui/crates/webtmux/tests/terminal_test.rs
  modified:
    - desktop-gpui/crates/terminal/src/lib.rs
    - desktop-gpui/crates/webtmux/src/app_state.rs
    - desktop-gpui/crates/webtmux/src/views/mod.rs
    - desktop-gpui/crates/webtmux/src/views/session_states.rs

key-decisions:
  - "TerminalConfig scrollback_limit 2000 (D1 FE default, not the reference 10_000)"
  - "Renderer defaults 14px / 1.35 line-height / fixed dark palette (D2, Phase 6 wires prefs)"
  - "ALT_SCREEN arrow-key fallback left uncalled with D5 comment; scroll_report returns None without mouse mode"
  - "No pendingScreen frame queue: synchronous Processor commits on the GPUI thread give frame atomicity"
  - "Pane-session attribution map scopes D7 retirement per session and resets entries on cross-session %N reuse"
  - "No per-view terminal-event subscription: the flume queue is mpsc and the commit path is its single consumer (D8 titles reliable)"
  - "TerminalView holds a shared Arc clone for paint/input; the store stays the owner (Pitfall 6 intent)"
  - "Re-export alacritty geometry/mode types through webtmux-terminal so webtmux needs no direct dep"

patterns-established:
  - "Pattern 1: apply_capture(data, screen_rows, ingested_history) pure byte function returning the atomic feed Vec<u8>"
  - "Pattern 2: Triple-guard-first terminal arms, then snapshotWritten gate (snapshot) or gateless replace path (output replace=true) or raw feed (replace=false)"
  - "Pattern 3: build_terminal_input/capture pure constructors returning (owning_session, envelope); send_* fire-and-forget with typed miss on unknown pane"
  - "Pattern 4: Tracer view retention map in AppState (create-once per pane, initial capture on creation, prune on retirement)"

requirements-completed:
  - TERM-01
  - TERM-02
  - TERM-03
  - TERM-07

coverage:
  - id: D1
    description: "webtmux-terminal ports all six reference modules with D1/D2/D5 deltas plus byte-oriented apply_capture"
    requirement: TERM-01
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/terminal/tests/capture_test.rs#test_live_output_parity"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/terminal/tests/input_test.rs#test_keystroke_table"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/terminal/tests/input_test.rs#test_input_byte_roundtrip"
        status: pass
    human_judgment: false
  - id: D2
    description: "First terminal.snapshot per pane replays history exactly once; repeats drop; live output continues (incl. reconnect re-capture)"
    requirement: TERM-02
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/terminal_test.rs#test_snapshot_exactly_once"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/terminal_test.rs#test_stale_pane_retirement"
        status: pass
    human_judgment: false
  - id: D3
    description: "Keyboard input reaches the owning pane byte-safe through terminal.input with from_utf8 contract"
    requirement: TERM-03
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/terminal_test.rs#test_terminal_input_ownership"
        status: pass
    human_judgment: false
  - id: D4
    description: "Hidden session tabs ingest terminal output without rendering; guard layers still drop stale/mismatched frames"
    requirement: TERM-07
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/terminal_test.rs#test_hidden_session_ingest"
        status: pass
    human_judgment: false
  - id: D5
    description: "Full tracer slice renders end to end: capture replay, live output, byte-safe input over real session sockets"
    requirement: TERM-01
    verification:
      - kind: other
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace (25 suites ok, 75 tests green)"
        status: pass
    human_judgment: false

duration: 70min
completed: 2026-09-06
status: complete
---

# Phase 04 Plan 01: Terminal Engine Tracer Slice Summary

**Tested `webtmux-terminal` crate (six ported engine modules + `apply_capture`), pane-id-keyed `TerminalStore` with guarded commit arms and owning-socket sends in `AppState`, and `TerminalView` rendering one live terminal per pane of the active window — full workspace suite green (75/75).**

## Performance

- **Duration:** 70 min (dominated by a cold-dependency rebuild after the E: drive filled; see deviations)
- **Started:** 2026-09-06T17:04:14Z
- **Completed:** 2026-09-06T18:14:25Z
- **Tasks:** 3
- **Files modified:** 18

## Accomplishments

- Ported the six reference engine modules into `webtmux-terminal` with only the RESEARCH-delta table applied (D1 scrollback 2000, D2 dark-default palette, D5 no arrow fallback, no frame queue) plus `grid_text`/`drain_events` headless helpers on `Terminal`.
- Implemented `capture.rs::apply_capture` as a pure byte-oriented function (normalize CRLF first, char-boundary split, explicit per-row origin, delta join with `\r\n`) backed by three JSON fixtures.
- Added `TerminalStore` types and `AppState` fields (`terminals`, `pane_session`, `tui_scroll`, `pane_titles`, `pending_viewport`) with pure headless-testable commit helpers following the 03-01 pure-apply pattern.
- Replaced the parse-and-ignore terminal arm with guarded commit arms (triple guard first, exactly-once gate for snapshot, gateless replace path, raw feed otherwise), D7 per-session retirement, D6 reconnect re-capture on transport→Connected and `connection.ready`, and D8 title commits with bell no-op.
- Added owning-socket `terminal.input`/`terminal.capture` fire-and-forget sends with typed misses for unknown panes; never the active-session proxy; no `hello` anywhere.
- Created `views/terminal_view.rs` as a dumb view over shared store handles (reference key/mouse/scroll/canvas shape, 14px/1.35 renderer, copy→paste→interrupt key order, SGR passthrough, resize arms `pending_viewport` only) and replaced the Phase-3 snapshot placeholder with per-pane `TerminalView`s for the active window, firing initial `terminal.capture` per pane on creation.
- Proved the tracer end to end with the single-invocation full workspace battery green.

## Task Commits

Each task was committed atomically following TDD and conventional commit guidelines:

1. **Task 1 (TDD RED):** `643eff1` (test(04-01): add failing contract tests for terminal engine and capture replay)
2. **Task 1 (TDD GREEN):** `e3826f6` (feat(04-01): port webtmux-terminal engine modules plus apply_capture)
3. **Task 2 (TDD RED):** `933bcc0` (test(04-01): add failing tests for TerminalStore commit arms and sends)
4. **Task 2 (TDD GREEN):** `3fd6b23` (feat(04-01): add AppState TerminalStore with guarded commit arms and sends)
5. **Task 3 (Tracer Slice):** `b66d3cd` (feat(04-01): render live TerminalViews for active window panes over store)

## Files Created/Modified

- `desktop-gpui/crates/terminal/src/terminal.rs` - Term + Processor engine (D1 2000 scrollback), grid_text/drain_events helpers
- `desktop-gpui/crates/terminal/src/input.rs` - Verbatim keystroke_to_bytes table with ported tests
- `desktop-gpui/crates/terminal/src/mouse.rs` - Geometry/SGR helpers verbatim; ALT_SCREEN arrow fallback uncalled (D5)
- `desktop-gpui/crates/terminal/src/render.rs` - Measure/layout/paint port (cell batching, selection, cursor)
- `desktop-gpui/crates/terminal/src/event.rs` - GpuiEventProxy + TerminalEvent verbatim
- `desktop-gpui/crates/terminal/src/colors.rs` - Fixed dark default palette (D2)
- `desktop-gpui/crates/terminal/src/capture.rs` - Byte-oriented apply_capture port with legacy/shrank branches
- `desktop-gpui/crates/terminal/src/lib.rs` - Module re-exports plus apply_capture and alacritty type facade
- `desktop-gpui/crates/terminal/tests/capture_test.rs` - Split/legacy/shrank/unicode/parity contracts
- `desktop-gpui/crates/terminal/tests/input_test.rs` - Full keystroke table + from_utf8 round-trip contracts
- `desktop-gpui/crates/terminal/tests/fixtures/*.json` - vim capture, unicode capture, output frames
- `desktop-gpui/crates/webtmux/src/app_state.rs` - TerminalStore, guarded arms, sends, recapture, retirement, view retention
- `desktop-gpui/crates/webtmux/src/views/terminal_view.rs` - Live per-pane view over shared store handles
- `desktop-gpui/crates/webtmux/src/views/session_states.rs` - Active-window per-pane rendering (placeholder replaced)
- `desktop-gpui/crates/webtmux/src/views/mod.rs` - Registered terminal_view module
- `desktop-gpui/crates/webtmux/tests/terminal_test.rs` - Exactly-once, hidden ingest, retirement, input ownership

## Decisions Made

- Used `Arc<Mutex<Terminal>>` shared handles for store ownership so views borrow for paint/input while hidden sessions keep ingesting.
- Scoped D7 retirement per session via an explicit pane→session attribution map (multi-tab correctness), with fresh-reset on cross-session `%N` reuse.
- Made the `AppState` commit path the single consumer of the mpsc terminal-event queue (no per-view drain) so D8 title commits are reliable.
- Kept `scroll_to_arrow_keys` as explicitly dead-ported code with a D5 comment rather than deleting it, so the omission stays reviewable.
- Retained per-pane view entities in `AppState` (created once, pruned on retirement) to avoid per-frame entity churn and remount capture storms.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] E: drive full (254 KB free) — redirected Cargo target dir to C:**
- **Found during:** Task 1 RED verification (`cargo test` failed with "No space left on device")
- **Issue:** `desktop-gpui/target` (16 GB) plus the web-term workspace target (23 GB) filled the E: drive; no room to compile or link.
- **Fix:** Set `CARGO_TARGET_DIR=C:\cargo-target\web-tmux` (25+ GB free) for every cargo invocation; full cold rebuild of pinned deps. The in-repo `target/` dir was left untouched. Follow-up builds must reuse the same `CARGO_TARGET_DIR` or they rebuild from scratch.
- **Files modified:** None (env-only; no repo files touched)
- **Verification:** Full workspace suite green under the C: target dir
- **Committed in:** N/A (no file change)

**2. [Rule 3 - Blocker] Parallel rustc target-dir metadata corruption — serialized test-target builds**
- **Found during:** Task 2 verification (`cargo test -p webtmux` failed with E0786 invalid metadata / missing rlib)
- **Issue:** Building all webtmux test targets in one parallel invocation intermittently corrupted target metadata on this machine (unrelated to code changes; single-target builds always passed).
- **Fix:** `cargo clean` on the C: target dir, then one cargo invocation per test target plus `CARGO_BUILD_JOBS=4`; the authoritative single-invocation full-workspace battery was still run once at the end and passed.
- **Files modified:** None
- **Verification:** 25/25 suites ok, 75/75 tests green in the final single-invocation run
- **Committed in:** N/A (no file change)

**3. [Rule 1 - Bug] RED test moved `feed` before reuse**
- **Found during:** Task 1 GREEN compile (`capture_test.rs:107` borrow-after-move)
- **Issue:** `String::from_utf8(feed)` moved the Vec before `process_bytes(&feed)`.
- **Fix:** Clone for the UTF-8 assertion.
- **Files modified:** `desktop-gpui/crates/terminal/tests/capture_test.rs`
- **Verification:** `cargo test -p webtmux-terminal` green
- **Committed in:** `e3826f6`

**4. [Rule 1 - Bug] RED test assumed a store entry pre-exists for fresh panes**
- **Found during:** Task 2 GREEN (`test_snapshot_exactly_once` unwrap on None)
- **Issue:** Test asserted an empty grid for a fresh pane, but entries are created lazily on the first terminal frame (by design).
- **Fix:** Assert `pane_grid_text` is `None` pre-snapshot (honest lazy-creation contract).
- **Files modified:** `desktop-gpui/crates/webtmux/tests/terminal_test.rs`
- **Verification:** `cargo test -p webtmux --test terminal_test` 4/4 green
- **Committed in:** `3fd6b23`

---

**Total deviations:** 4 auto-fixed (2 blockers, 2 bugs)
**Impact on plan:** None. No architectural changes, no new crates, no protocol changes. Scope stayed exactly on plan files plus this summary.

## Issues Encountered

None beyond the deviations above. The pre-existing Go backend LSP error (`be/test/tmux/service_integration_test.go:147` CapturePane arity) was observed but is out of scope — backend protocol is unchanged by this plan.

## User Setup Required

None - no external service configuration required. Note for follow-up builds: reuse `CARGO_TARGET_DIR=C:\cargo-target\web-tmux` (E: drive is full).

## Next Phase Readiness

- The tracer slice (capture-replay → live-output → byte-safe input) is proven headless with contracts; plan 04-02 can build wheel policy, selection/clipboard depth, and the resize dance on top of the store and view written here.
- `pending_viewport` is armed by views but never sent — 04-02 owns the 100ms debounce send plus the 150/325ms layout-key pair.
- Tracer layout stacks panes full-width; Phase 5 replaces it with tmux-geometry rendering using the same store entries.

---
*Phase: 04-terminal-engine-live-rendering*
*Completed: 2026-09-06*

## Self-Check: PASSED
- All 14 created files exist on disk.
- All 5 task commits verified in git log.
- Full workspace suite green: 25/25 suites, 75/75 tests (single-invocation gate).
