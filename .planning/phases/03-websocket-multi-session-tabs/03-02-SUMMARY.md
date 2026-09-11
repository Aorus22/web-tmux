---
phase: 03-websocket-multi-session-tabs
plan: "02"
subsystem: desktop-gpui
tags: [rust, gpui, gpui-component, context-menu, dialog, input-state, window-tabs, kill-confirm, tmux]

# Dependency graph
requires:
  - phase: 03-websocket-multi-session-tabs (03-01)
    provides: WS transport (connect_session + correlated send + tagged read pump), per-session tab map with triple generation guard, rename re-resolution primitive, window_tabs() model
  - phase: 02-rest-client-sidebar-session-management
    provides: DLG1 dialog Entity pattern, sidebar SB1 rows, ST1 workspace pages, settings atomic-write store
provides:
  - views/tab_strip.rs — SHELL-01 title bar (divider + WindowTabs middle + Settings gear, fire-and-forget window.select)
  - views/session_states.rs — honest Phase-6 Settings placeholder page + picker open-or-activate
  - views/session_context_menu.rs — sidebar session right-click menu (Rename + separator + Kill Session destructive) + kill-confirm dialog + D6 kill transport
  - views/rename_session_dialog.rs — RenameSessionForm Entity + DLG1-token dialog + target-socket submit + re-resolution close
  - settings confirm_kill_session/pane/window flags defaulting true with legacy-JSON defaults
  - AppState rename/kill orchestration (submit_rename, execute_kill, ephemeral one-shot, pending_rename flush)
affects: [04-terminal-io, 05-window-pane-ops, 06-settings-page, 07-reconnect-ux]

actuals:
  tokens: 20000
  tasks: 3
  commits: 4

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "gpui-component ContextMenuExt::context_menu with stable per-session element ids (poll-tick safe); on_click handlers capture WeakEntity<AppState> + target and open dialogs directly (window + &mut App in hand)"
    - "Correlated WS mutation orchestration as AppState methods: sync send_command (owned oneshot Receiver) then cx.spawn await with 10s forget-timeout; dialog close routed via AnyWindowHandle::update"
    - "Rename-after-connect via pending_rename flushed/dropped inside ensure_session_socket outcome paths"
    - "D6 kill transport order encoded as KillRoute enum: victim socket, any-live-socket + explicit session, ephemeral one-shot that commits nothing"

key-files:
  created:
    - desktop-gpui/crates/webtmux/src/views/rename_session_dialog.rs
    - desktop-gpui/crates/webtmux/src/views/session_context_menu.rs
    - desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs
  modified:
    - desktop-gpui/crates/settings/src/lib.rs
    - desktop-gpui/crates/settings/tests/store_test.rs
    - desktop-gpui/crates/webtmux/src/app_state.rs
    - desktop-gpui/crates/webtmux/src/views/mod.rs
    - desktop-gpui/crates/webtmux/src/views/tab_strip.rs
    - desktop-gpui/crates/webtmux/src/views/sidebar.rs
    - desktop-gpui/crates/webtmux/src/views/session_states.rs

key-decisions:
  - "Kill Session menu label renders red-400 (#f87171) text via PopupMenuItem::element: the DLG1 destructive #7F1D1D is unreadable as text on #1e1e1e, so #7F1D1D is reserved for the Kill button fill and inline error backgrounds (FE destructive-menu intent preserved, token adapted for dark bg)"
  - "SelectSessionView picker cards open-or-activate tabs (ensure-socket when new) like sidebar rows: their own copy says 'Pick a session to open it as a tab', and bare active_session assignment leaves a socket-less dead fallback"
  - "Rename-to-same-name dismisses the dialog without socket traffic (re-resolution cannot migrate old-to-same; sending it would false-error)"
  - "Direct-kill (no dialog) failures record entry.last_error + poll + notify with the tab kept open; the inline-error truth applies to the confirm-dialog path where a dialog exists"
  - "Ephemeral kill socket uses a fresh pending map and never forwards events, so bootstrap snapshots commit nowhere (D6 phantom-tab caution); tab-close + poll still run on correlated success"
  - "finish_kill_success always removes the sessions entry (close_session alone skips unopened entries)"

patterns-established:
  - "Pattern: correlated mutation = sync send_command returning an owned Receiver, awaited in a cx.spawn task with tokio::time::timeout(10s); late replies resolve dropped receivers and are ignored (forget-timeout semantics without map surgery)"
  - "Pattern: dialog window handles travel as AnyWindowHandle params (Option for dialog-less flows) and close via handle.update(cx, close_dialog) from any AppContext"
  - "Pattern: stable context-menu ids as format!(\"session-row/{name}\") strings (gpui-pre 0.3.3 has no (&str, String) ElementId impl; formatted ids are equally poll-tick stable)"

requirements-completed:
  - SHELL-01
  - SESS-04
  - SESS-05

# Coverage metadata (#1602)
coverage:
  - id: T1
    description: "confirm_kill_session/pane/window default true for fresh settings and legacy JSON lacking the keys; explicit false survives round-trip and save/load"
    requirement: SESS-05
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/settings/tests/store_test.rs#test_kill_confirm_defaults"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/settings/tests/store_test.rs#test_kill_confirm_legacy_json"
        status: pass
    human_judgment: false
  - id: T2
    description: "Kill flow gate branches on the session flag only (confirm dialog when true, direct kill when false; pane/window flags do not steer it)"
    requirement: SESS-05
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs#test_kill_flow_gate"
        status: pass
    human_judgment: false
  - id: T3
    description: "Title bar keeps S1 shell with divider + WindowTabs flex-1 middle only when active_session is set (h-7/max-w-44 chips, {index}: {name}, active #2d2d2d/#d4d4d4, idle #808080/hover #262626), fire-and-forget window.select on chip click, Settings gear to the honest Phase-6 placeholder with tabs left open; no Plus button, no chip menu"
    requirement: SHELL-01
    verification:
      - kind: other
        ref: "cargo check --manifest-path desktop-gpui/Cargo.toml --package webtmux (clean, zero warnings)"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/tabs_test.rs#test_window_tabs_model (03-01 guard still green: no-snapshot and empty-windows render empty)"
        status: pass
    human_judgment: true
    rationale: "Chip geometry/colors/copy and gear-to-placeholder behavior are visual judgments; no headless render assertion exists. Manual UAT: launch against a backend, open two sessions, check tabs/gear (same class as the 02-02 visual checks)."
  - id: T4
    description: "Sidebar rows wrapped in ContextMenuExt with stable per-session ids; menu is Rename + separator + Kill Session (destructive) only; left-click opens-or-activates the tab (ensure-socket when new)"
    requirement: SESS-04
    verification:
      - kind: other
        ref: "cargo check --manifest-path desktop-gpui/Cargo.toml --package webtmux (clean, zero warnings)"
        status: pass
    human_judgment: true
    rationale: "Right-click menu open/dismiss, poll-tick state retention, and click routing are interactive GPUI behaviors. Manual UAT: right-click a row, keep it open across a poll tick, pick Rename / Kill Session."
  - id: T5
    description: "Rename dialog (DLG1 tokens, prefilled, autofocus, Enter-submit, Rename disabled while empty/busy, inline #7F1D1D error) validates pre-flight and sends session.rename on the target socket; success migrates old-to-new with generation+1, reconnects, polls, closes; error/timeout stays open with the tab untouched"
    requirement: SESS-04
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/tabs_test.rs#test_rename_reresolution (03-01 pure-half guard still green)"
        status: pass
      - kind: other
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace (39 passed, 0 failed)"
        status: pass
    human_judgment: true
    rationale: "Dialog typing/prefill/focus and the live correlated rename against a real backend are interactive. The pure migration half is unit-locked; end-to-end rename needs manual UAT with two open sessions."
  - id: T6
    description: "Kill honors the setting (confirm dialog with destructive Kill vs direct kill), rides victim / any-live-plus-explicit-session / ephemeral transports, closes socket + tab with neighbor activation + poll on success, renders inline destructive without closing on error"
    requirement: SESS-05
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/tabs_test.rs#test_tab_lifecycle + test_tab_close_neighbor_middle (neighbor activation guards still green)"
        status: pass
      - kind: other
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace (39 passed, 0 failed)"
        status: pass
    human_judgment: true
    rationale: "Confirm-vs-direct branching, dialog copy, and live kill against a real backend are interactive. Neighbor-activation math is unit-locked; end-to-end kill needs manual UAT."
  - id: T7
    description: "Full workspace battery green with all 03-01 guards (tab lifecycle, rename re-resolution, window-tabs model, generation guard, terminal-ignore, bootstrap/correlation interop) still passing — STATE-04 held"
    requirement: STATE-04
    verification:
      - kind: other
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace (39 passed, 0 failed: 11 webtmux + 12 backend-client + 7 settings + 9 supervisor)"
        status: pass
    human_judgment: false

# Metrics
duration: 47min
completed: 2026-09-06
status: complete
---

# Phase 03 Plan 02: Title-Bar Tabs, Session Menus, Rename & Kill Flows Summary

**Kill-confirm settings flags with legacy defaults, SHELL-01 title bar (WindowTabs + Settings gear + Phase-6 placeholder), sidebar session context menus, rename dialog with mid-session socket re-resolution, and the three-transport kill-confirm flow — full workspace battery 39/39 green, zero warnings.**

## Performance

- **Duration:** 47 min
- **Started:** 2026-09-06T15:45:00Z
- **Completed:** 2026-09-06T16:32:08Z
- **Tasks:** 3
- **Files modified:** 10

## Accomplishments

- Implemented kill-confirm settings (Task 1, TDD): `confirm_kill_session` + `confirm_kill_pane` + `confirm_kill_window` on `DesktopSettings`, all `#[serde(default = "default_true")]` so Phase-1/2 files keep confirming (FE `settingsStore.ts` parity); `AppState::kill_requires_confirm` gate reads only the session flag.
- Implemented the SHELL-01 title bar (Task 2): S1 `h(px(44.0))` shell kept; divider + `WindowTabs` flex-1 middle rendered ONLY when `active_session` is set, fed by `window_tabs()` (03-01 model); chips `h-7 max-w-44 rounded-md px-2.5 text-sm`, `{index}: {name}` truncated, active bg `#2d2d2d`/text `#d4d4d4`, idle `#808080`/hover `#262626`; chip click = fire-and-forget `send_window_select` (correlated `window.select` with `paneId`, receiver dropped, no optimistic flip); Settings gear clears `active_session` + sets `showing_settings` to the honest static placeholder ("Settings arrive in Phase 6", Back returns to workspace routing, tabs stay open); no Plus button, no chip menu (D8). Sidebar footer Settings routes to the same page.
- Implemented session context menus + rename + kill (Task 3): rows wrapped in `ContextMenuExt::context_menu` with stable `session-row/{name}` ids (poll-tick safe), menu = Rename + separator + Kill Session (destructive red text) only, left-click opens-or-activates (ensure-socket when new, switch when open).
- Implemented `RenameSessionForm` dialog (DLG1 tokens, prefilled current name, autofocus, Enter-submit, Rename disabled while empty/busy, inline `#7F1D1D` error): submit runs `validate_session_name` pre-flight then `AppState::submit_rename` — target-socket send (D5, `ensure_session_socket(target)` + `pending_rename` when the tab is not open), 10s correlated await, success = `rename_session_entry` migration (generation+1) + reconnect + poll + close, error/timeout = inline error with dialog open and tab untouched.
- Implemented the kill flow: `KillRoute::{Victim, ViaOther, Ephemeral}` (D6) in `AppState::execute_kill` — victim socket, any-live-socket + explicit `session`, or ephemeral one-shot that commits nothing; success = `finish_kill_success` (entry drop even when unopened + neighbor activation) + poll (+ dialog close when open); error = inline destructive with dialog open, tab untouched; direct-kill failures record `last_error` + poll with the tab kept.
- Verification: settings gate tests green, carried 03-01 guards green (`test_tab_lifecycle`, `test_rename_reresolution`, `test_window_tabs_model`, `test_ws_generation_guard`), full workspace suite **39 passed / 0 failed** (11 webtmux + 12 backend-client + 7 settings + 9 supervisor), `cargo check` zero warnings.

## Task Commits

Each task was committed atomically (TDD RED→GREEN for Task 1):

1. **Task 1 (TDD RED):** `1260623` — test(03-02): add failing tests for kill-confirm flags and flow gate
2. **Task 1 (TDD GREEN):** `3b4206d` — feat(03-02): add kill-confirm settings flags with legacy defaults and flow gate
3. **Task 2:** `f8b7b4a` — feat(03-02): add title-bar WindowTabs plus Settings gear and placeholder page
4. **Task 3:** `2c0fbe7` — feat(03-02): add session context menu plus rename dialog and kill-confirm flow

## Files Created/Modified

- `desktop-gpui/crates/settings/src/lib.rs` - three confirm flags with `default_true` serde defaults + Default impl
- `desktop-gpui/crates/settings/tests/store_test.rs` - `test_kill_confirm_defaults`, `test_kill_confirm_legacy_json` (+ roundtrip literal updated for the new fields)
- `desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs` - `test_kill_flow_gate` (NEW)
- `desktop-gpui/crates/webtmux/src/app_state.rs` - `showing_settings`, rename/kill form fields, `pending_rename`, `send_window_select`, `submit_rename` + await, `KillRoute` + `execute_kill` + ephemeral path, `finish_kill_success`, ensure-socket pending flush/drain, `WindowExt` import
- `desktop-gpui/crates/webtmux/src/views/tab_strip.rs` - divider + WindowTabs middle + Settings gear (SHELL-01)
- `desktop-gpui/crates/webtmux/src/views/session_states.rs` - settings placeholder page + gate in body routing + picker open-or-activate
- `desktop-gpui/crates/webtmux/src/views/sidebar.rs` - stable row ids, context-menu wrap, row open-or-activate click, footer Settings routing
- `desktop-gpui/crates/webtmux/src/views/session_context_menu.rs` - menu builder, kill-confirm dialog, kill submit routing (NEW)
- `desktop-gpui/crates/webtmux/src/views/rename_session_dialog.rs` - rename form entity, DLG1 dialog, prefill/focus/Enter-submit, rename submit (NEW)
- `desktop-gpui/crates/webtmux/src/views/mod.rs` - registered the two new modules

## Decisions Made

- **Kill-menu red is #f87171 text**, not #7F1D1D: the DLG1 destructive token is unreadable as text on `#1e1e1e`; #7F1D1D stays on the Kill button fill and inline error backgrounds. FE destructive-menu intent preserved with a dark-bg-legible token.
- **Picker cards open-or-activate** like sidebar rows (their own copy promises "open it as a tab"; bare `active_session` assignment leaves a socket-less dead fallback). Serves SESS-02 directly.
- **Rename-to-same-name dismisses** without socket traffic (migration cannot map old-to-same; sending it would false-error after a server success).
- **Direct-kill failures record `last_error` + poll** with the tab kept open — no dialog exists to render inline in; the inline-error truth covers the confirm-dialog path.
- **Dialog closes route through `AnyWindowHandle`** passed into `submit_rename`/`execute_kill` (`Option` for dialog-less direct kill), closed from any `AppContext` after the form is cleared.
- **`pending_rename` carries `(target, new_name, window_handle)`** and is flushed on ensure success / dropped with an inline error on ensure failure, so the `is_submitting` flag never sticks (T-03-07).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] E: drive nearly full (274 MB free) broke linking; cleared regenerable artifacts and serialized the suite**
- **Found during:** Task 1 GREEN verification
- **Issue:** `cargo test -p webtmux` failed at link time (`No space left on device`); the follow-up full-workspace run failed harder (parallel rustc jobs mmap-ing huge rlibs exhausted the paging file: `failed to mmap ... libgpui_component-*.rlib ... too small` + cascading `can't find crate` errors).
- **Fix:** Deleted `desktop-gpui/target/debug/incremental` (~1.5 GB, 100% regenerable; no source or config touched) and ran the suite with `cargo test -j 2` (first `-j 2` run green, second confirmation run green). No code impact.
- **Files modified:** none (build cache only)
- **Committed in:** N/A (environment only)

**2. [Rule 3 - Blocker] Stable row ids use `format!(\"session-row/{name}\")`, not the tuple `.id((\"session-row\", name))`**
- **Found during:** Task 3
- **Issue:** `gpui-pre 0.3.3` implements `From` for `ElementId` tuples only as `(&'static str, int)` / `(SharedString, usize)` — no `(&str, String)` impl, so the plan's literal tuple form does not compile.
- **Fix:** Formatted `String` id per session. Equally stable across poll ticks (no counters/indices), which is the actual Pitfall-5 requirement the plan cites.
- **Files modified:** `desktop-gpui/crates/webtmux/src/views/sidebar.rs`
- **Committed in:** `2c0fbe7`

**3. [Rule 3 - Blocker] WindowTabs container needed `.id(\"window-tabs\")` before `.overflow_x_scroll()`**
- **Found during:** Task 2
- **Issue:** `overflow_x_scroll` lives on `StatefulInteractiveElement`, which plain `Div` does not implement — it becomes available only after `.id()` wraps the div into `Stateful<Div>` (same reason the sidebar scroll area calls `.id()` first). `cargo check` failed with `no method named overflow_x_scroll`.
- **Fix:** Added the static `.id(\"window-tabs\")` first in the chain. No behavior change (one tabs container per window).
- **Files modified:** `desktop-gpui/crates/webtmux/src/views/tab_strip.rs`
- **Committed in:** `f8b7b4a`

**4. [Rule 3 - Blocker] Cargo 1.96 accepts a single test-name filter (carried 03-01 deviation)**
- **Found during:** Tasks 1/3 verification
- **Issue:** Plan verify commands pass multiple filters (`cargo test -p <pkg> <f1> <f2>`), which cargo 1.96 rejects with `unexpected argument`.
- **Fix:** Ran each named filter as a separate invocation (all green) plus the full-workspace suite as the authoritative gate.
- **Committed in:** N/A (verification procedure only)

**5. [Rule 2 - Missing critical] SelectSessionView picker cards open-or-activate instead of bare-selecting**
- **Found during:** Task 3
- **Issue:** Cards set `active_session` without opening a tab or socket, rendering the socket-less dead fallback and contradicting the view's own "Pick a session to open it as a tab" copy (SESS-02: opening a session must connect it).
- **Fix:** Cards now `open_session` + `ensure_session_socket` (when `base_url` is present) + notify, mirroring the sidebar row click. Within plan files (`session_states.rs` is a Task-2 file).
- **Files modified:** `desktop-gpui/crates/webtmux/src/views/session_states.rs`
- **Committed in:** `2c0fbe7`

---

**Total deviations:** 5 auto-fixed (4 blockers, 1 missing-critical; 2 environment/procedure, 3 code)
**Impact on plan:** None. No scope expansion, no new crates (backend protocol untouched, `gpui-component` menu/Dialog/Input already pinned at 0.6.0), no Plus button, no chip menu, no dead Phase-5 entries, rename never rides the active socket for a non-active target.

## Issues Encountered

- **Disk/resource pressure (see deviation 1):** E: holds an 18 GB `target/` dir (13.4 GB `deps` alone). If linking fails again, the durable fix is `CARGO_TARGET_DIR` on a roomier drive (Y: has ~900 GB) — left unchanged to avoid altering the user's build setup; flagging for the phase verifier.
- **Pre-existing Go diagnostic:** `be/test/tmux/service_integration_test.go:147` type error surfaces in LSP output; backend untouched, out of scope (same as 03-01).
- **Pre-existing `.planning/` modifications:** `.planning/ROADMAP.md`, `.planning/STATE.md`, `.planning/config.json` (plus `state.json`, `02-VERIFICATION.md`, `ui-reviews/`) were already modified/untracked before execution started (orchestrator planning state). Per the lane instruction to limit changes to plan files + SUMMARY, STATE/ROADMAP were NOT updated and no metadata commit was made — the orchestrator owns those.

## User Setup Required

None - no external service configuration required. Mock WS servers run in-process; no live backend needed for the suite.

## Next Phase Readiness

- Manual-only UAT remains for the phase verifier (same class as 02-02): (a) title-bar tabs render + chip click selects windows + gear opens the placeholder and Back returns; (b) right-click menu opens Rename/Kill Session, survives a poll tick, Rename prefills/focuses/submits on Enter and the tab migrates old→new, Kill confirms per setting and the tab closes with neighbor activation; (c) direct kill with the setting off; (d) rename/kill of an unopened sidebar session (ensure/ephemeral paths).
- The correlated mutation path (`send_command` + 10s timeout + inline-error + `is_submitting` guard) is now proven in shape for Phase 5 window/pane mutations to reuse; `KillRoute` documents the victim/other/ephemeral selection order.
- Phase 6 owns the real Settings page behind `showing_settings`; Phase 7 owns reconnect banners/toasts (direct-kill errors currently land in `entry.last_error`, which Phase 7 can render).

---
*Phase: 03-websocket-multi-session-tabs*
*Completed: 2026-09-06*

## Self-Check: PASSED
- All 3 created files exist on disk (`rename_session_dialog.rs`, `session_context_menu.rs`, `kill_confirm_test.rs`).
- All 4 task commits verified in git log: `1260623`, `3b4206d`, `f8b7b4a`, `2c0fbe7` (`git log --oneline -6`).
- Full workspace suite green: 39 passed / 0 failed (11 webtmux incl. 3 new + 12 backend-client + 7 settings incl. 2 new + 9 supervisor); carried 03-01 guards (`test_tab_lifecycle`, `test_rename_reresolution`, `test_window_tabs_model`, `test_ws_generation_guard`) individually re-run green.
- `cargo check -p webtmux -p webtmux-settings` after touching all plan files: zero warnings, zero errors.
- Stub scan over `crates/webtmux/src/views`: no TODO/FIXME/unimplemented; the only "placeholder" hits are Input placeholder strings and the plan-mandated honest Phase-6 Settings page.
- Threat scan: no new surface outside the plan `<threat_model>` — rename input pre-flights via `validate_session_name` (T-03-05), target-socket sends + generation bump (T-03-06), `is_submitting` + 10s timeouts (T-03-07), static placeholder copy (T-03-08), zero new packages (T-03-SC).
