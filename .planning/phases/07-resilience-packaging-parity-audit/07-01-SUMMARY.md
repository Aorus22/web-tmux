---
phase: 07-resilience-packaging-parity-audit
plan: "01"
subsystem: desktop-gpui
tags: [rust, gpui, resilience, reconnect, toast, backoff, STATE-03]

# Dependency graph
requires:
  - phase: 06-theme-settings-chrome-parity
    provides: 116-test workspace, AppState apply_event triple-guard, TOKIO_RT ensure_session_socket, palette overlay stacking precedent
provides:
  - Pump-local transport.lost split distinct from server tmux.disconnected/reconnecting (D2)
  - Reconnect banner view with 4-way taxonomy copy plus manual Reconnect wired into render
  - Bounded toast overlay queue (cap 5, dedupe kind+session, autohide 5s/3s) with backend-verbatim error surfaces
  - Generation-guarded BACKOFF retry loop ([250,500,1000,2000,5000,10000]ms verbatim, 10s tail forever)
affects: [07-02-PLAN, 07-03-PLAN]

actuals:
  tokens: 16624
  tasks: 3
  commits: 4

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pump-boundary taxonomy split via never-on-wire local signal (transport.lost) handled only in apply_event"
    - "Origin-bit banner model: never render from bare TransportState, always (transport, origin, attempt)"
    - "Hand-rolled VecDeque toast queue (cap 5 drop-oldest, dedupe key kind+session, autohide per-kind lifetimes)"
    - "Generation-captured retry timers on TOKIO_RT/GPUI executor with drop-on-mismatch plus close+ensure redial"

key-files:
  created:
    - desktop-gpui/crates/webtmux/src/views/reconnect_banner.rs
    - desktop-gpui/crates/webtmux/src/views/toasts.rs
    - desktop-gpui/crates/webtmux/tests/reconnect_test.rs
    - desktop-gpui/crates/webtmux/tests/toast_test.rs
  modified:
    - desktop-gpui/crates/backend-client/src/ws.rs
    - desktop-gpui/crates/backend-client/src/lib.rs
    - desktop-gpui/crates/webtmux/src/app_state.rs
    - desktop-gpui/crates/webtmux/src/views/mod.rs

key-decisions:
  - "Pump close/error arms forward EV_TRANSPORT_LOST (transport.lost), never synthesize EV_TMUX_DISCONNECTED (Pitfall 1)"
  - "Transport-lost arms the SOLE client retry; server tmux.disconnected/reconnecting never retry client-side (monitor owns it)"
  - "Banner copy table fixed per D2 with tmux- prefix on server states; attempt N only on the Local arm"
  - "Toast queue funnels every push through push_toast so no caller can grow it unbounded (T-07-02)"
  - "Retry tails at 10s forever like FE BACKOFF[5] with manual Reconnect always offered (Open Q3)"

patterns-established:
  - "Pattern 1: banner_copy_for pure model keyed on (TransportState, TransportOrigin, attempt) returning Option<(title, show_button)>"
  - "Pattern 2: schedule_transport_retry capturing (session, generation, attempt) with liveness gate before manual_reconnect"
  - "Pattern 3: expire_toasts(now) headless autohide model driven with fake Instant in tests, lazily in render with wake-up notify"

requirements-completed:
  - STATE-03

coverage:
  - id: SC1-taxonomy
    description: "WS-drop shows Connection lost retrying (attempt N) with auto-retry while server tmux.disconnected shows tmux disconnected waiting for tmux with manual Reconnect only"
    requirement: STATE-03
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/reconnect_test.rs#test_transport_lost_distinct_from_tmux_disconnected"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/reconnect_test.rs#test_tmux_reconnecting_maps_transient"
        status: pass
    human_judgment: false
  - id: SC1-toast
    description: "Failing commands surface backend-verbatim error toasts; queue caps at 5 with dedupe-by-key so flaps never cover the workspace"
    requirement: STATE-03
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/toast_test.rs#test_toast_queue_cap_and_dedupe"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/toast_test.rs#test_toast_copy_keys"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/toast_test.rs#test_command_error_surfaces_toast"
        status: pass
    human_judgment: false
  - id: SC1-retry
    description: "Auto-reconnect timing matches FE BACKOFF verbatim with generation safety; ws_guard regression green"
    requirement: STATE-03
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/reconnect_test.rs#test_backoff_ladder_verbatim"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/reconnect_test.rs#test_retry_timer_bound_to_generation"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/ws_guard_test.rs#test_ws_generation_guard"
        status: pass
      - kind: integration
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace (125 passed, 0 failed)"
        status: pass
    human_judgment: false
  - id: SC1-pixels
    description: "Banner/toast pixels, kill-tmux vs kill-backend drill, and flap behavior side-by-side with Electron"
    requirement: STATE-03
    verification:
      - kind: other
        ref: "07-03 audit backstop (GPUI not headless-renderable, Ph2/4/5 precedent)"
        status: human_needed
    human_judgment: true

duration: 40min
completed: 2026-09-07
status: complete
---

# Phase 07 Plan 01: STATE-03 Resilience Tracer Summary

**Split transport-lost from tmux-health at the pump boundary, rendered the reconnect banner plus bounded toast overlay end to end, and armed FE-verbatim BACKOFF auto-reconnect with generation guards — 7 new headless contracts green, workspace 125/125.**

## Performance

- **Duration:** 40 min (build-dominated: initial 6m44s compile, then incremental)
- **Completed:** 2026-09-07
- **Tasks:** 3 (1 tracer + 1 tdd + 1 auto)
- **Files modified:** 8 (4 created, 4 modified)
- **Commits:** 4 atomic conventional commits

## Accomplishments

- Split the pump conflation: `ws.rs` close/error arms forward pump-local `EV_TRANSPORT_LOST` (`transport.lost`, a string the server can never send); `grep EV_TMUX_DISCONNECTED` in `ws.rs` returns only the const definition plus one NEVER-synthesize comment — zero syntheses in the arms (Pitfall 1 gate).
- Extended `AppState` with `TransportOrigin` (None/Local/Server) plus `reconnect_attempt`, `RECONNECT_BACKOFF_MS = [250,500,1000,2000,5000,10_000]` verbatim with `backoff_delay`/`should_arm_retry`/`should_run_retry` helpers, and the `banner_copy_for` 4-way copy table (never bare `TransportState`).
- Built `views/reconnect_banner.rs` per UI-SPEC BANNER-1 (full-width bar above workspace, amber transient without button, both disconnected states with Reconnect wired to `manual_reconnect` close+ensure) and registered it in `views/mod.rs` with `AppState::render` `.when()` stacking.
- Built the bounded toast queue in `AppState` (`VecDeque`, cap 5 drop-oldest, dedupe key kind+session newest-wins, autohide 5s errors / 3s success+info) with `push_toast`/`expire_toasts`/`visible_toasts`; `note_session_error` and `EV_SERVER_ERROR` push backend-verbatim error toasts while inline `last_error` lines stay; snapshot closing a failure episode fires the `Reconnected` success toast and clears the banner.
- Built `views/toasts.rs` per UI-SPEC TOAST-1 (bottom-right stack, kind icon prefix per toast, plain-text backend strings per T-07-01, autohide wake-up notify, dismiss button) stacked below the palette modal in z-order.
- Armed the generation-guarded retry chain: `schedule_transport_retry` captures `(session, generation, attempt)`, sleeps the BACKOFF delay off the GPUI thread, drops on generation mismatch/close/recovery, bumps attempt for the banner, then dials via `manual_reconnect`; connect-Err arm maps to Local + preserves the running attempt and arms the next step; `ensure_session_socket` preserves the episode across redials; server-sent tmux states never arm.
- Wrote `reconnect_test.rs` (4 tests) and `toast_test.rs` (3 tests) covering every plan `<behavior>` bullet; `ws_guard_test` stays green; full workspace 125 passed / 0 failed (116 baseline + 2 from 07-02 bundle + 7 new here); `git diff` over `Cargo.toml`/`Cargo.lock` empty (zero-new-package gate).

## Task Commits

Resumed from an interrupted attempt that left unstaged partials and zero 07-01 commits; good partials were reconciled against the plan, missing pieces (toast_test, generation test) finished, then committed atomically — 07-02 files (Makefile, scripts, bundle tests) and STATE/ROADMAP untouched throughout:

1. **Task 1 (tracer RED):** `efbdeb0` (test(07-01): add reconnect transport-lost vs tmux contracts — 3 tests)
2. **Task 1 (tracer GREEN):** `70283d9` (feat(07-01): split transport-lost pump signal plus banner and toast surfaces — ws.rs, lib.rs, app_state.rs, views/mod.rs, reconnect_banner.rs, toasts.rs)
3. **Task 2 (tdd):** `1b89362` (test(07-01): add toast cap/dedupe/copy contracts — toast_test.rs, 3 tests)
4. **Task 3 (auto):** `57c90a9` (feat(07-01): add generation-guarded BACKOFF retry regression — 57-line generation-drop extension to reconnect_test.rs)

Note: the shared `app_state.rs` model plus both view files ride in `70283d9` because the prior partial had already interleaved T1/T2/T3 logic in that single file; T2/T3 commits therefore add tests plus the one view-registration slice each. See Deviations.

## Files Created/Modified

- `desktop-gpui/crates/backend-client/src/ws.rs` — `EV_TRANSPORT_LOST` const; both pump arms forward it (modified)
- `desktop-gpui/crates/backend-client/src/lib.rs` — re-export `EV_TRANSPORT_LOST` (modified)
- `desktop-gpui/crates/webtmux/src/app_state.rs` — origin bit, BACKOFF, banner copy, toast queue, retry chain, render wiring (modified)
- `desktop-gpui/crates/webtmux/src/views/mod.rs` — register `reconnect_banner` + `toasts` (modified)
- `desktop-gpui/crates/webtmux/src/views/reconnect_banner.rs` — 4-way taxonomy bar + Reconnect (created)
- `desktop-gpui/crates/webtmux/src/views/toasts.rs` — bounded overlay stack (created)
- `desktop-gpui/crates/webtmux/tests/reconnect_test.rs` — 4 mapping/ladder/generation contracts (created)
- `desktop-gpui/crates/webtmux/tests/toast_test.rs` — 3 cap/dedupe/copy contracts (created)

## Decisions Made

- Reused the interrupted attempt's partials after `git status`/`git diff` reconciliation instead of rewriting: the ws.rs split, app_state model, and both views were already correct per the plan's acceptance criteria and verified green before committing.
- Kept `views/mod.rs` whole in the T1 feat commit rather than splitting its two one-line adds across T1/T2: splitting would have left intermediate commits uncompilable (app_state references both views), violating per-commit green.
- Matched FE exactly on the retry tail (infinite, 10s forever) with manual Reconnect always offered plus attempt count, per Open Q3 resolution — parity by construction.

## Deviations from Plan

### Auto-fixed Issues

None — the reconciled partials already satisfied every acceptance criterion; no bugs, blockers, or missing critical functionality were found during resumption beyond the two absent test files, which are the plan's own Task 2/3 deliverables (not deviations).

### Resumption notes (not deviations — orchestrator-directed)

**1. Prior-partial reuse with interleaved-model commit shape**
- **Found during:** Task 1 reconciliation (`git status --short` + `git diff` vs plan files)
- **Issue:** The interrupted attempt had already interleaved T1 (origin/BACKOFF/banner), T2 (toast queue), and T3 (retry chain) logic in the single `app_state.rs`, plus both views and the 3-test `reconnect_test.rs` — with zero 07-01 commits and `toast_test.rs` plus the generation-drop test still missing.
- **Fix:** Verified the partials against each task's acceptance criteria (pump grep-gate, BACKOFF verbatim, banner wiring, toast paths, retry guards), kept everything good, wrote the two missing test files, and committed in task order (test → feat → test → retry-test) with the shared model riding in the T1 feat commit so every commit stays compilable.
- **Files modified:** all 8 plan files (see above)
- **Commits:** `efbdeb0`, `70283d9`, `1b89362`, `57c90a9`
- **Impact on plan:** None — TDD gate sequence preserved per file slice (each test file committed before/with its view); the only impurity is T2/T3 model code landing in the T1 commit, documented here.

**2. T2 RED-not-isolated (shared-file constraint)**
- **Found during:** Task 2 (`toast_test.rs` creation)
- **Issue:** A strict RED (test fails on base) is unisolatable for the toast queue because its model shares `app_state.rs` with T1 — committing the test before the model would not compile, and the model was already verified green.
- **Fix:** Committed the queue model in the T1 feat, then the toast contracts as the T2 test commit; GREEN verified per task (`toast_test` 3/3, `reconnect_test` 4/4). The T1 test commit (`efbdeb0`) is a true RED against its parent (references `EV_TRANSPORT_LOST`/`TransportOrigin` absent on base).
- **Impact on plan:** Cosmetic — headless contracts still guard every behavior bullet.

## Issues Encountered

None — no auth gates, no package installs, no architectural changes. The only friction was PowerShell `Select-String` argument quoting during the stub scan (fell back to `grep`, clean).

## User Setup Required

None - no external service configuration required. Pixels (kill-tmux vs kill-backend drill, flap behavior) remain `human_needed` for the 07-03 audit backstop by design (GPUI not headless-renderable).

## Next Phase Readiness

- STATE-03 headless contracts are green and the banner/toast/retry surfaces are wired into render; 07-03 can run the eyes-on kill-tmux vs kill-backend drill plus flap test against this build.
- Packaging (07-02) is untouched and independent: its 3 commits plus the still-unstaged Makefile targets remain for the orchestrator to land separately.
- Zero lockfile drift; no new crates; protocol unchanged except the pump-boundary split per research.

---
*Phase: 07-resilience-packaging-parity-audit*
*Completed: 2026-09-07*

## Self-Check: PASSED
- All 4 created files exist on disk (reconnect_banner.rs, toasts.rs, reconnect_test.rs, toast_test.rs).
- All 4 modified files exist with 07-01 content (ws.rs, lib.rs, app_state.rs, views/mod.rs).
- All 4 task commits verified in git log (efbdeb0, 70283d9, 1b89362, 57c90a9); no 07-02 files in any of them.
- Workspace 125 passed / 0 failed; reconnect 4/4, toast 3/3, ws_guard 2/2; pump grep-gate and lockfile gate clean.
